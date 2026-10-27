//! Who may connect: the code signature of the peer (`session-host.md`, section 9, spike 3).
//!
//! The policy comes from the host's own signature and is decided once, at start. A host signed
//! with a Developer ID takes only a peer that is signed with the same team's Developer ID as the
//! app (identifier and team, never a cdhash or a version, so an older host serves a newer app).
//! Any other host (ad-hoc, linker-signed by `cargo build`, unsigned, Apple Development) takes
//! every peer that passed the uid check: an identifier proves nothing when anyone can pick it, so
//! there is nothing to check there. No team id is compiled in, so forks and dev builds need no
//! switch.

use wormhole_term::sys::socket::Stream;

use wormhole_term::sys::codesign::{self, Refusal, Requirement};

/// The identifier the app's executable is signed with: `BUNDLE_ID` in `packaging/app.env`
/// (a test compares them). The host's own is `<this>.host`, and no host connects to a host, so
/// it is not let in.
const APP_IDENTIFIER: &str = "app.getwormhole.terminal";

/// Apple's team ids are ten uppercase letters and digits. Anything else would put text of
/// another shape into a requirement, so it is not used.
fn valid_team_id(team: &str) -> bool {
    team.len() == 10 && team.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// The part of the requirement that makes it a Developer ID signature of `team` (the host holds
/// itself to just this, whatever its identifier): Apple's anchor,
/// the Developer ID CA (marker 6.2.6), a Developer ID Application leaf (marker 6.1.13) and the
/// team in the leaf's organizational unit. An ad-hoc or Apple Development signature fails it.
fn developer_id_clauses(team: &str) -> Option<String> {
    valid_team_id(team).then(|| {
        format!(
            "anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] \
             and certificate leaf[field.1.2.840.113635.100.6.1.13] \
             and certificate leaf[subject.OU] = \"{team}\""
        )
    })
}

/// What a peer must satisfy: the same, and the app's identifier.
fn peer_requirement(team: &str) -> Option<String> {
    developer_id_clauses(team).map(|clauses| format!("identifier \"{APP_IDENTIFIER}\" and {clauses}"))
}

/// Which peers the host serves, after the uid check.
pub(crate) enum Policy {
    /// Every peer of the same user.
    UidOnly,
    /// Only a peer that satisfies the requirement.
    Require(Requirement),
}

impl Policy {
    /// Read the host's own signature and choose; says which in the log.
    pub(crate) fn detect() -> Self {
        let Some(team) = codesign::own_team_id() else {
            tracing::info!(
                "peer check: uid only (the host has no team id: ad-hoc, unsigned or linker-signed)"
            );
            return Self::UidOnly;
        };
        match Self::require_team(&team) {
            Some(policy) => {
                tracing::info!(team, "peer check: requiring the app's Developer ID signature of this team");
                policy
            }
            None => {
                tracing::info!(
                    team,
                    "peer check: uid only (the host is not signed with this team's Developer ID)"
                );
                Self::UidOnly
            }
        }
    }

    /// The requirement for `team`, when the host itself satisfies the Developer ID part of it
    /// (a host signed by some other means must not lock itself out of its own app).
    fn require_team(team: &str) -> Option<Self> {
        let own = codesign::requirement(&developer_id_clauses(team)?)?;
        codesign::self_satisfies(&own).then_some(())?;
        codesign::requirement(&peer_requirement(team)?).map(Self::Require)
    }

    /// Whether to serve the peer at the other end of `socket`; says why not in the log.
    pub(crate) fn allows(&self, socket: &Stream) -> bool {
        match self.check(socket) {
            Ok(()) => true,
            Err(refusal) => {
                tracing::warn!(
                    peer_pid = refusal.pid,
                    status = refusal.status,
                    "refused a connection: {}",
                    refusal.stage.describe()
                );
                false
            }
        }
    }

    /// Check the peer's code, found from its audit token, against the requirement.
    pub(crate) fn check(&self, socket: &Stream) -> Result<(), Refusal> {
        match self {
            Self::UidOnly => Ok(()),
            Self::Require(requirement) => codesign::check_peer(socket, requirement),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{APP_IDENTIFIER, peer_requirement, valid_team_id};

    #[test]
    fn the_peer_requirement_is_the_app_of_the_team_and_nothing_more() {
        assert_eq!(
            peer_requirement("ABCDE12345").as_deref(),
            Some(
                "identifier \"app.getwormhole.terminal\" and anchor apple generic \
                 and certificate 1[field.1.2.840.113635.100.6.2.6] \
                 and certificate leaf[field.1.2.840.113635.100.6.1.13] \
                 and certificate leaf[subject.OU] = \"ABCDE12345\""
            )
        );
        // Identifier and team only: a newer app must be let in by an older host.
        let text = peer_requirement("ABCDE12345").unwrap_or_default();
        assert!(!text.contains("cdhash") && !text.contains("version"));
    }

    #[test]
    fn a_team_id_of_another_shape_makes_no_requirement() {
        assert!(valid_team_id("3VL3FJXX72"));
        for bad in ["", "abcde12345", "ABCDE1234", "ABCDE123456", "ABCDE1234\"", "ABCDE 2345", "ABCDE1234é"]
        {
            assert!(!valid_team_id(bad), "{bad:?}");
            assert_eq!(peer_requirement(bad), None, "{bad:?}");
        }
        assert_eq!(peer_requirement("A\" or anchor apple or \"B"), None);
    }

    #[test]
    fn the_app_identifier_is_the_bundle_id() {
        let env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/app.env");
        let Ok(text) = std::fs::read_to_string(env) else { return };
        let bundle = text.lines().find_map(|line| line.strip_prefix("BUNDLE_ID=")).unwrap_or_default();
        assert_eq!(bundle.trim_matches('"'), APP_IDENTIFIER);
    }

    mod socket_peer {
        use wormhole_term::sys::socket::{Listener, Stream};

        use super::super::{Policy, peer_requirement};
        use wormhole_term::sys::codesign::{self, Stage};

        /// A client connected to a listener in a temporary folder, and the server's side of it.
        fn connected() -> (tempfile::TempDir, Stream, Stream) {
            let dir = tempfile::tempdir().expect("a temporary folder");
            let path = dir.path().join("s");
            let listener = Listener::bind(&path).expect("bind");
            let client = Stream::connect(&path).expect("connect");
            let (server, _) = listener.accept().expect("accept");
            (dir, client, server)
        }

        #[test]
        fn a_test_binary_is_ad_hoc_so_only_the_uid_counts() {
            // Linker-signed by cargo: no team id, so no Developer ID policy.
            assert_eq!(codesign::own_team_id(), None);
            assert!(matches!(Policy::detect(), Policy::UidOnly));
            let (_dir, _client, server) = connected();
            assert!(Policy::UidOnly.allows(&server));
        }

        #[test]
        fn a_host_that_is_not_developer_id_signed_does_not_require_a_team() {
            assert!(Policy::require_team("ABCDE12345").is_none());
        }

        #[test]
        fn an_ad_hoc_peer_is_refused_under_a_requirement_through_its_audit_token() {
            let requirement = codesign::requirement(&peer_requirement("ABCDE12345").expect("a team id"))
                .expect("the requirement compiles");
            let (_dir, _client, server) = connected();
            let policy = Policy::Require(requirement);
            // The peer is this process: found by its audit token, and not of that team.
            let refusal = policy.check(&server).expect_err("an ad-hoc peer must be refused");
            assert_eq!(refusal.stage, Stage::Requirement);
            assert_eq!(refusal.pid, Some(std::process::id()));
            assert!(!policy.allows(&server));
        }
    }
}
