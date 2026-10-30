//! The code signature of the program at the other end of a connection (`session-host.md`, section 9,
//! spike 3). What a requirement says and who may connect is the policy in the host's `peer`; reading
//! signatures is the system's, and the app reads the same way who an agent is (`agent-permissions.md`).

use crate::sys::socket::Stream;

#[cfg(target_os = "linux")]
use super::linux::codesign as os;
#[cfg(target_os = "macos")]
use super::macos::codesign as os;

pub use os::Requirement;

/// Where a refusal came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The socket gave no audit token.
    Token,
    /// The system found no code for the token.
    Code,
    /// The code does not satisfy the requirement.
    Requirement,
    /// The peer's executable is gone from disk, and the kernel's record of its signature is not the
    /// app's.
    Kernel,
}

impl Stage {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Token => "no audit token for the peer",
            Self::Code => "no code for the peer's audit token",
            Self::Requirement => "the peer's signature does not satisfy the requirement",
            Self::Kernel => "the peer's executable is gone and the kernel's record of it is not the app's",
        }
    }
}

/// A peer that is not let in.
#[derive(Debug)]
pub struct Refusal {
    pub stage: Stage,
    /// The peer's pid, when it could be told.
    pub pid: Option<u32>,
    /// The system's status for the call that failed, or the `errno` for the token.
    pub status: i32,
}

/// A requirement compiled once from its text; `None` (said in the log) when it does not parse. The text
/// names the signing `identifier` and the `team` too: a peer whose executable is gone from disk is held
/// to those two alone (the system cannot read its certificate any more).
pub fn requirement(text: &str, identifier: &str, team: &str) -> Option<Requirement> {
    os::requirement(text, identifier, team)
}

/// Whether this process satisfies `requirement`.
pub fn self_satisfies(requirement: &Requirement) -> bool {
    os::self_satisfies(requirement)
}

/// The team id in this process's own signature, if it has one.
pub fn own_team_id() -> Option<String> {
    os::own_team_id()
}

/// Find the peer's code and check it against `requirement`.
pub fn check_peer(stream: &Stream, requirement: &Requirement) -> Result<(), Refusal> {
    os::check_peer(stream, requirement)
}

/// How a program is signed, as far as that tells who it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignerKind {
    /// A Developer ID Application certificate of a team: the signer is who the user trusts.
    DeveloperId,
    /// Apple's own system software.
    AppleSystem,
    /// An ad-hoc signature (many Rust and Go builds on arm64 are signed so by the linker): it names
    /// nobody, only its hash pins the program.
    AdHoc,
    /// Signed by something that is neither Developer ID nor Apple (a development certificate): pinned
    /// by its hash, as an ad-hoc one.
    Other,
    /// No signature.
    Unsigned,
    /// The system would not say (the process is gone, or not the user's).
    Unreadable,
}

/// What a running program's signature says. Everything but `kind` is absent when the signature has
/// no such part (an ad-hoc one has no team).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub kind: SignerKind,
    /// The signing identifier, e.g. `com.anthropic.claude-code`. Anyone can pick one: only with a
    /// Developer ID does it mean anything.
    pub identifier: Option<String>,
    /// Apple's ten-character team id.
    pub team_id: Option<String>,
    /// The certificate's organisation, for display: `Anthropic PBC`.
    pub subject: Option<String>,
    /// The code directory's hash of the code that is running, in hex.
    pub cdhash: Option<String>,
    /// A Developer ID program without the hardened runtime, which a library can be injected into: it is
    /// reported as ad hoc (pinned by its hash) and this says why.
    pub injectable: bool,
}

impl Signature {
    /// A signature that says nothing but its `kind`.
    pub fn of(kind: SignerKind) -> Self {
        Self { kind, identifier: None, team_id: None, subject: None, cdhash: None, injectable: false }
    }
}

/// The signature of the running process `pid`.
pub fn of_pid(pid: u32) -> Signature {
    os::of_pid(pid)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::{SignerKind, of_pid};

    #[test]
    fn a_test_binary_is_signed_ad_hoc_by_the_linker_and_says_its_hash() {
        let own = of_pid(std::process::id());
        assert_eq!(own.kind, SignerKind::AdHoc, "{own:?}");
        assert!(own.identifier.is_some() && own.team_id.is_none() && own.subject.is_none(), "{own:?}");
        let hash = own.cdhash.expect("a hash");
        assert!(hash.len() >= 40 && hash.bytes().all(|b| b.is_ascii_hexdigit()), "{hash}");
    }

    #[test]
    fn a_system_program_is_apple_and_a_pid_nobody_has_is_unreadable() {
        let mut child = crate::sys::process::command("/bin/sleep").arg("5").spawn().expect("sleep starts");
        let signature = of_pid(child.id());
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(signature.kind, SignerKind::AppleSystem, "{signature:?}");
        assert_eq!(of_pid(u32::MAX).kind, SignerKind::Unreadable);
    }
}
