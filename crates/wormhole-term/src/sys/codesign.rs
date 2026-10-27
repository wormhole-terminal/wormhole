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
    /// The system's status for the call that failed, or the `errno` for the token.
    pub status: i32,
}

/// A requirement compiled once from its text; `None` (said in the log) when it does not parse.
pub fn requirement(text: &str) -> Option<Requirement> {
    os::requirement(text)
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
