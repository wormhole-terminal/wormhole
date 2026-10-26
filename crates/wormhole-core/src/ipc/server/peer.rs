//! Who is at the other end of a control connection, as far as the system can tell.
//!
//! A caller names itself in `from` (its `WORMHOLE_TAB_ID`), and any process of the user can name
//! any terminal there. So a claim can only restrict: it names who must not be typed into. What
//! grants (a terminal reading itself) must be something a process can't choose, and its place in the
//! process tree is: no process can make itself a descendant of a shell it does not run under.

use crate::platform::process::ancestors;
use crate::platform::socket::{Stream, peer_pid};

/// How many parents are followed: deeper than any shell, script and tool chain, and a bound on the
/// system calls one request costs.
pub const ANCESTORS_MAX: usize = 32;

/// The process that connected, and the processes above it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Peer {
    /// The connecting process, when the system says (`LOCAL_PEERPID` on macOS).
    pub pid: Option<u32>,
    /// Its parent chain, nearest first, at most [`ANCESTORS_MAX`], stopping at process 1.
    pub ancestors: Vec<u32>,
}

impl Peer {
    /// A peer seen only as a pid and the chain above it (tests, and a caller that knows both).
    pub fn new(pid: u32, ancestors: Vec<u32>) -> Self {
        Self { pid: Some(pid), ancestors }
    }

    /// Who is at the other end of `stream`. A few system calls: for control requests only.
    pub(super) fn of(stream: &Stream) -> Self {
        let Some(pid) = peer_pid(stream) else { return Self::default() };
        // A peer that has already ended has no parents left to read: its chain is empty, and only
        // what it claimed counts, which can refuse but never grant.
        Self { pid: Some(pid), ancestors: ancestors(pid, ANCESTORS_MAX) }
    }

    /// Whether the caller runs inside the process `shell`: it is that process or below it. This is
    /// how a request is known to come from a terminal (its shell) and not from a claim.
    pub fn runs_under(&self, shell: u32) -> bool {
        self.pid == Some(shell) || self.ancestors.contains(&shell)
    }
}

#[cfg(test)]
mod tests {
    use super::Peer;

    #[test]
    fn a_caller_runs_under_a_shell_that_is_it_or_above_it() {
        let peer = Peer::new(300, vec![200, 100]);
        assert!(peer.runs_under(300) && peer.runs_under(200) && peer.runs_under(100));
        assert!(!peer.runs_under(400));
        assert!(!Peer::default().runs_under(100), "an unknown peer is under nothing");
    }
}
