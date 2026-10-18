//! Who may connect: the code signature of the peer (`session-host.md`, section 9, spike 3).
//!
//! The policy comes from the host's own signature and is decided once, at start. A host signed
//! with a Developer ID takes only a peer that is signed with the same team's Developer ID as the
//! app (identifier and team, never a cdhash or a version, so an older host serves a newer app).
//! Any other host (ad-hoc, linker-signed by `cargo build`, unsigned, Apple Development) takes
//! every peer that passed the uid check: an identifier proves nothing when anyone can pick it, so
//! there is nothing to check there. No team id is compiled in, so forks and dev builds need no
//! switch.

use std::os::fd::AsRawFd;

/// The identifier the app's executable is signed with: `BUNDLE_ID` in `packaging/app.env`
/// (a test compares them). The host's own is `<this>.host`, and no host connects to a host, so
/// it is not let in.
const APP_IDENTIFIER: &str = "app.getwormhole.terminal";

/// Apple's team ids are ten uppercase letters and digits. Anything else would put text of
/// another shape into a requirement, so it is not used.
// Only the macOS policy builds requirements; the tests run the text everywhere.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn valid_team_id(team: &str) -> bool {
    team.len() == 10 && team.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// The part of the requirement that makes it a Developer ID signature of `team` (the host holds
/// itself to just this, whatever its identifier): Apple's anchor,
/// the Developer ID CA (marker 6.2.6), a Developer ID Application leaf (marker 6.1.13) and the
/// team in the leaf's organizational unit. An ad-hoc or Apple Development signature fails it.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
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
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn peer_requirement(team: &str) -> Option<String> {
    developer_id_clauses(team).map(|clauses| format!("identifier \"{APP_IDENTIFIER}\" and {clauses}"))
}

/// Which peers the host serves, after the uid check.
pub enum Policy {
    /// Every peer of the same user.
    UidOnly,
    /// Only a peer that satisfies the requirement.
    #[cfg(target_os = "macos")]
    Require(security_framework::os::macos::code_signing::SecRequirement),
}

#[cfg(not(target_os = "macos"))]
impl Policy {
    /// There is no code signature to read here.
    pub fn detect() -> Self {
        tracing::info!("peer check: uid only (this platform has no code signatures)");
        Self::UidOnly
    }

    pub fn allows(&self, _socket: &impl AsRawFd) -> bool {
        true
    }
}

#[cfg(target_os = "macos")]
pub use mac::Refusal;

#[cfg(target_os = "macos")]
impl Policy {
    /// Read the host's own signature and choose; says which in the log.
    pub fn detect() -> Self {
        let Some(team) = mac::own_team_id() else {
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
        let own = mac::requirement(&developer_id_clauses(team)?)?;
        mac::self_satisfies(&own).then_some(())?;
        mac::requirement(&peer_requirement(team)?).map(Self::Require)
    }

    /// Whether to serve the peer at the other end of `socket`; says why not in the log.
    pub fn allows(&self, socket: &impl AsRawFd) -> bool {
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
    pub fn check(&self, socket: &impl AsRawFd) -> Result<(), Refusal> {
        match self {
            Self::UidOnly => Ok(()),
            Self::Require(requirement) => mac::check_peer(socket.as_raw_fd(), requirement),
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::c_void;
    use std::io;
    use std::os::fd::RawFd;
    use std::str::FromStr as _;

    use core_foundation::base::{CFType, TCFType as _};
    use core_foundation::data::CFData;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::{CFString, CFStringRef};
    use security_framework::os::macos::code_signing::{
        Flags, GuestAttributes, SecCode, SecRequirement, SecStaticCode,
    };
    use security_framework_sys::code_signing::{SecCSFlags, SecCodeRef, SecStaticCodeRef};

    /// `kSecCSSigningInformation`: also read the signature's details, the team id among them.
    const SIGNING_INFORMATION: SecCSFlags = 1 << 1;

    /// An `audit_token_t`: eight 32-bit values, the pid the sixth.
    const TOKEN_WORDS: usize = 8;
    const TOKEN_PID: usize = 5;

    // Not in `security-framework-sys`. The framework is linked already (by that crate).
    #[allow(non_upper_case_globals, unsafe_code)] // declarations only; each call has its SAFETY note.
    unsafe extern "C" {
        static kSecCodeInfoTeamIdentifier: CFStringRef;

        fn SecCodeCopyStaticCode(code: SecCodeRef, flags: SecCSFlags, out: *mut SecStaticCodeRef) -> i32;

        fn SecCodeCopySigningInformation(
            code: SecStaticCodeRef,
            flags: SecCSFlags,
            out: *mut CFDictionaryRef,
        ) -> i32;
    }

    /// Where a refusal came from.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Stage {
        /// The socket gave no audit token.
        Token,
        /// The system found no code for the token.
        Code,
        /// The code does not satisfy the requirement.
        Requirement,
    }

    impl Stage {
        pub fn describe(self) -> &'static str {
            match self {
                Self::Token => "no audit token for the peer",
                Self::Code => "no code for the peer's audit token",
                Self::Requirement => "the peer's signature does not satisfy the requirement",
            }
        }
    }

    /// A peer that is not let in.
    #[derive(Debug)]
    pub struct Refusal {
        pub stage: Stage,
        /// The peer's pid, when it could be told.
        pub pid: Option<u32>,
        /// The `OSStatus` of the call that failed, or the `errno` for the token.
        pub status: i32,
    }

    /// A requirement compiled once from its text; `None` (said in the log) when it does not parse.
    pub fn requirement(text: &str) -> Option<SecRequirement> {
        match SecRequirement::from_str(text) {
            Ok(requirement) => Some(requirement),
            Err(err) => {
                tracing::warn!(status = err.code(), "peer check: the requirement does not compile");
                None
            }
        }
    }

    /// Whether this process satisfies `requirement`.
    pub fn self_satisfies(requirement: &SecRequirement) -> bool {
        SecCode::for_self(Flags::NONE).and_then(|own| own.check_validity(Flags::NONE, requirement)).is_ok()
    }

    /// The team id in this process's own signature, if it has one.
    #[allow(unsafe_code)] // Security framework FFI around two calls the crate does not wrap.
    pub fn own_team_id() -> Option<String> {
        let code = SecCode::for_self(Flags::NONE).ok()?;
        let mut static_code: SecStaticCodeRef = std::ptr::null_mut();
        // SAFETY: `code` is a live SecCode; the call writes one owned reference (or nothing, on
        // failure) into a local.
        let status = unsafe { SecCodeCopyStaticCode(code.as_concrete_TypeRef(), 0, &mut static_code) };
        if status != 0 || static_code.is_null() {
            return None;
        }
        // SAFETY: the reference was returned by a Copy function, so this takes the one retain.
        let static_code = unsafe { SecStaticCode::wrap_under_create_rule(static_code) };
        let mut info: CFDictionaryRef = std::ptr::null();
        // SAFETY: as above; the call writes one owned dictionary (or nothing) into a local.
        let status = unsafe {
            SecCodeCopySigningInformation(static_code.as_concrete_TypeRef(), SIGNING_INFORMATION, &mut info)
        };
        if status != 0 || info.is_null() {
            return None;
        }
        // SAFETY: a Copy function's dictionary, taken once; its keys are CFStrings and its values
        // CFTypes, which is how it is read.
        let info: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_create_rule(info) };
        // SAFETY: the key is a constant CFString that lives as long as the framework.
        let key = unsafe { CFString::wrap_under_get_rule(kSecCodeInfoTeamIdentifier) };
        // An ad-hoc signature has no team id: the key is missing.
        info.find(&key)?.downcast::<CFString>().map(|team| team.to_string())
    }

    /// The peer's audit token: who is on the other end of the connection, in a form that cannot
    /// be spoofed by a pid that is reused.
    #[allow(unsafe_code)] // libc FFI; the descriptor is open for the call, the output is a local.
    fn audit_token(fd: RawFd) -> io::Result<[u32; TOKEN_WORDS]> {
        let mut token = [0u32; TOKEN_WORDS];
        let mut len = size_of_val(&token) as libc::socklen_t;
        // SAFETY: getsockopt writes at most `len` bytes into `token`, which is that long.
        let status = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_LOCAL,
                libc::LOCAL_PEERTOKEN,
                token.as_mut_ptr().cast::<c_void>(),
                &mut len,
            )
        };
        if status == 0 && len as usize == size_of_val(&token) {
            Ok(token)
        } else if status == 0 {
            Err(io::Error::from(io::ErrorKind::InvalidData))
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// The pid of the peer, for a log line when there is no token.
    #[allow(unsafe_code)] // libc FFI; the descriptor is open for the call, the output is a local.
    fn peer_pid(fd: RawFd) -> Option<u32> {
        let mut pid: libc::pid_t = 0;
        let mut len = size_of_val(&pid) as libc::socklen_t;
        // SAFETY: getsockopt writes at most `len` bytes into `pid`, which is that long.
        let status = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_LOCAL,
                libc::LOCAL_PEERPID,
                (&raw mut pid).cast::<c_void>(),
                &mut len,
            )
        };
        (status == 0).then_some(pid as u32)
    }

    /// Find the peer's code from its audit token and check it against `requirement`.
    pub fn check_peer(fd: RawFd, requirement: &SecRequirement) -> Result<(), Refusal> {
        let token = audit_token(fd).map_err(|err| Refusal {
            stage: Stage::Token,
            pid: peer_pid(fd),
            status: err.raw_os_error().unwrap_or(0),
        })?;
        let pid = Some(token[TOKEN_PID]);
        let bytes: Vec<u8> = token.iter().flat_map(|word| word.to_ne_bytes()).collect();
        let data = CFData::from_buffer(&bytes);
        let mut attributes = GuestAttributes::new();
        attributes.set_audit_token(data.as_concrete_TypeRef());
        let code = SecCode::copy_guest_with_attribues(None, &attributes, Flags::NONE)
            .map_err(|err| Refusal { stage: Stage::Code, pid, status: err.code() })?;
        code.check_validity(Flags::NONE, requirement).map_err(|err| Refusal {
            stage: Stage::Requirement,
            pid,
            status: err.code(),
        })
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

    #[cfg(target_os = "macos")]
    mod macos {
        use std::os::unix::net::{UnixListener, UnixStream};

        use super::super::mac::{self, Stage};
        use super::super::{Policy, peer_requirement};

        /// A client connected to a listener in a temporary folder, and the server's side of it.
        fn connected() -> (tempfile::TempDir, UnixStream, UnixStream) {
            let dir = tempfile::tempdir().expect("a temporary folder");
            let path = dir.path().join("s");
            let listener = UnixListener::bind(&path).expect("bind");
            let client = UnixStream::connect(&path).expect("connect");
            let (server, _) = listener.accept().expect("accept");
            (dir, client, server)
        }

        #[test]
        fn a_test_binary_is_ad_hoc_so_only_the_uid_counts() {
            // Linker-signed by cargo: no team id, so no Developer ID policy.
            assert_eq!(mac::own_team_id(), None);
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
            let requirement = mac::requirement(&peer_requirement("ABCDE12345").expect("a team id"))
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
