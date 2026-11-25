//! Who may connect: the code signature of the peer (`session-host.md`, section 9, spike 3).
//!
//! The policy comes from the host's own signature and is decided once, at start. A host signed
//! with a Developer ID takes only a peer that is signed with the same team's Developer ID as the
//! app it serves (identifier and team, never a cdhash or a version, so an older host serves a newer
//! app). That app is the host's own signing identifier without its `.host`: the bundling signs each
//! app's host `<bundle id>.host`, so the 1.x host serves `app.getwormhole.terminal`, the beta's
//! `app.getwormhole.terminal.beta`, and neither the other. Only the team can sign a Developer ID host,
//! so the identifier is as much the team's word as the team id is, and the host checks that it
//! satisfies it itself before it requires anything of a peer.
//! Any other host (ad-hoc, linker-signed by `cargo build`, unsigned, Apple Development) takes
//! every peer that passed the uid check: an identifier proves nothing when anyone can pick it, so
//! there is nothing to check there. No team id is compiled in, so forks and dev builds need no
//! switch.
//!
//! A peer whose executable was replaced on disk while it runs (an app updated by Homebrew or a copy)
//! cannot be found by the system any more; it is then held to the kernel's record of its signature,
//! the app's identifier and this team, valid and not ad hoc (wormhole-term's `sys/macos/kernel_peer.rs`).

use wormhole_term::sys::socket::Stream;

use wormhole_term::sys::codesign::{self, Refusal, Requirement};

/// The app a Developer ID host serves when its own identifier is not `<app>.host`: `BUNDLE_ID` in
/// `packaging/app.env` (a test compares them), which every host served before the identifier was read.
const APP_IDENTIFIER: &str = "app.getwormhole.terminal";

/// What the bundling appends to an app's bundle id to sign its host.
const HOST_SUFFIX: &str = ".host";

/// Apple's team ids are ten uppercase letters and digits. Anything else would put text of
/// another shape into a requirement, so it is not used.
fn valid_team_id(team: &str) -> bool {
    team.len() == 10 && team.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// A bundle id's shape: dot-separated labels of ASCII letters, digits and hyphens, none empty. Nothing
/// that could close the quoted string of a requirement (`"`, `\`) or start another clause gets in.
fn valid_identifier(identifier: &str) -> bool {
    identifier
        .split('.')
        .all(|label| !label.is_empty() && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

/// The app a host signed `own` serves: `own` without its `.host`, when the rest is a bundle id.
fn served_app(own: &str) -> Option<&str> {
    own.strip_suffix(HOST_SUFFIX).filter(|app| valid_identifier(app))
}

/// The part of the requirement that makes it a Developer ID signature of `team`: Apple's anchor,
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

/// What a peer must satisfy: the same, and the identifier of the `app` served.
fn peer_requirement(app: &str, team: &str) -> Option<String> {
    let clauses = developer_id_clauses(team)?;
    valid_identifier(app).then(|| format!("identifier \"{app}\" and {clauses}"))
}

/// What a host of `team` whose signing identifier is `own` holds itself to before it requires anything
/// of a peer, and the app it serves. Signed `<app>.host`: that identifier and the Developer ID clauses,
/// so the identifier the app comes from is checked by the system and not only read. Signed otherwise:
/// the clauses alone and the 1.x app, as before the identifier was read.
fn own_requirement<'a>(team: &str, own: Option<&'a str>) -> Option<(String, &'a str)> {
    let clauses = developer_id_clauses(team)?;
    Some(match own.and_then(|own| served_app(own).map(|app| (own, app))) {
        Some((own, app)) => (format!("identifier \"{own}\" and {clauses}"), app),
        None => (clauses, APP_IDENTIFIER),
    })
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
        let own = codesign::own_identifier();
        match Self::require_team(&team, own.as_deref()) {
            Some((policy, app)) => {
                tracing::info!(
                    team,
                    app,
                    "peer check: requiring the app's Developer ID signature of this team"
                );
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

    /// The requirement for `team` and the app it names, when the host signed `own` itself satisfies
    /// its own part (a host signed by some other means must not lock itself out of its own app).
    fn require_team<'a>(team: &str, own: Option<&'a str>) -> Option<(Self, &'a str)> {
        let (own_text, app) = own_requirement(team, own)?;
        codesign::self_satisfies(&codesign::requirement(&own_text, app, team)?).then_some(())?;
        let peer = codesign::requirement(&peer_requirement(app, team)?, app, team)?;
        Some((Self::Require(peer), app))
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
    use super::{APP_IDENTIFIER, own_requirement, peer_requirement, served_app, valid_team_id};

    const TEAM: &str = "ABCDE12345";
    const CLAUSES: &str = "anchor apple generic \
                           and certificate 1[field.1.2.840.113635.100.6.2.6] \
                           and certificate leaf[field.1.2.840.113635.100.6.1.13] \
                           and certificate leaf[subject.OU] = \"ABCDE12345\"";

    /// What the 1.x host required of its peers before the identifier was read, to the byte.
    const REQUIRED_BY_1X: &str = "identifier \"app.getwormhole.terminal\" and anchor apple generic \
                                  and certificate 1[field.1.2.840.113635.100.6.2.6] \
                                  and certificate leaf[field.1.2.840.113635.100.6.1.13] \
                                  and certificate leaf[subject.OU] = \"ABCDE12345\"";

    /// The app a Developer ID host signed `own` serves, and what it requires of a peer.
    fn served(own: Option<&str>) -> (&str, Option<String>) {
        let (_, app) = own_requirement(TEAM, own).expect("a team id");
        (app, peer_requirement(app, TEAM))
    }

    #[test]
    fn the_1x_host_requires_exactly_what_it_did() {
        let (app, peer) = served(Some("app.getwormhole.terminal.host"));
        assert_eq!(app, "app.getwormhole.terminal");
        assert_eq!(peer.as_deref(), Some(REQUIRED_BY_1X));
        // Identifier and team only: a newer app must be let in by an older host.
        assert!(!REQUIRED_BY_1X.contains("cdhash") && !REQUIRED_BY_1X.contains("version"));
        // And it holds itself to its own identifier first, so the app it serves is one it is signed for.
        let (own, _) = own_requirement(TEAM, Some("app.getwormhole.terminal.host")).expect("a team id");
        assert_eq!(own, format!("identifier \"app.getwormhole.terminal.host\" and {CLAUSES}"));
    }

    #[test]
    fn the_beta_host_serves_the_beta_and_only_it() {
        let (app, peer) = served(Some("app.getwormhole.terminal.beta.host"));
        assert_eq!(app, "app.getwormhole.terminal.beta");
        assert_eq!(peer, Some(format!("identifier \"app.getwormhole.terminal.beta\" and {CLAUSES}")));
        // `identifier "…"` is an exact match, so neither host names the other's app.
        assert_ne!(peer.as_deref(), Some(REQUIRED_BY_1X));
    }

    #[test]
    fn a_host_not_signed_app_dot_host_serves_the_1x_app_as_before() {
        for own in [None, Some("wormhole-host"), Some("app.getwormhole.terminal"), Some("host")] {
            let (text, app) = own_requirement(TEAM, own).expect("a team id");
            assert_eq!((text.as_str(), app), (CLAUSES, APP_IDENTIFIER), "{own:?}");
            assert_eq!(peer_requirement(app, TEAM).as_deref(), Some(REQUIRED_BY_1X), "{own:?}");
        }
    }

    #[test]
    fn an_identifier_of_another_shape_is_not_served_or_required() {
        assert_eq!(served_app(".host"), None, "an empty app");
        for bad in [
            "app\".host",
            "app\" or anchor apple or \"x.host",
            "app\\.host",
            "app terminal.host",
            "app.getwormhole.é.host",
            "app..terminal.host",
            ".app.host",
            "app..host",
            "app.\n.host",
        ] {
            assert_eq!(served_app(bad), None, "{bad:?}");
            let app = bad.strip_suffix(".host").unwrap_or(bad);
            assert_eq!(peer_requirement(app, TEAM), None, "{bad:?}");
            // Such a host is held to the Developer ID clauses alone and serves the 1.x app, as before.
            assert_eq!(own_requirement(TEAM, Some(bad)), Some((CLAUSES.to_owned(), APP_IDENTIFIER)));
        }
    }

    #[test]
    fn a_team_id_of_another_shape_makes_no_requirement() {
        assert!(valid_team_id("3VL3FJXX72"));
        for bad in ["", "abcde12345", "ABCDE1234", "ABCDE123456", "ABCDE1234\"", "ABCDE 2345", "ABCDE1234é"]
        {
            assert!(!valid_team_id(bad), "{bad:?}");
            assert_eq!(peer_requirement(APP_IDENTIFIER, bad), None, "{bad:?}");
            assert_eq!(own_requirement(bad, Some("app.getwormhole.terminal.host")), None, "{bad:?}");
        }
        assert_eq!(peer_requirement(APP_IDENTIFIER, "A\" or anchor apple or \"B"), None);
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

        use super::super::{APP_IDENTIFIER, Policy, own_requirement, peer_requirement};
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
            // Linker-signed: whatever identifier it says it has, it fails its own part.
            for own in
                [None, Some("app.getwormhole.terminal.host"), Some("app.getwormhole.terminal.beta.host")]
            {
                assert!(Policy::require_team("ABCDE12345", own).is_none(), "{own:?}");
            }
            let own = codesign::own_identifier();
            assert!(Policy::require_team("ABCDE12345", own.as_deref()).is_none());
        }

        #[test]
        fn the_requirements_of_a_beta_host_compile() {
            let (own, app) =
                own_requirement("ABCDE12345", Some("app.getwormhole.terminal.beta.host")).expect("a team id");
            assert!(codesign::requirement(&own, app, "ABCDE12345").is_some());
            let peer = peer_requirement(app, "ABCDE12345").expect("a bundle id");
            assert!(codesign::requirement(&peer, app, "ABCDE12345").is_some());
        }

        #[test]
        fn an_ad_hoc_peer_is_refused_under_a_requirement_through_its_audit_token() {
            let requirement = codesign::requirement(
                &peer_requirement(APP_IDENTIFIER, "ABCDE12345").expect("a team id"),
                APP_IDENTIFIER,
                "ABCDE12345",
            )
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
