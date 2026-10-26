//! Who is at the other end of a control connection, as far as the system can tell.
//!
//! A caller names itself in `from` (its `WORMHOLE_TAB_ID`), and any process of the user can name
//! any terminal there. So a claim can only restrict: it names who must not be typed into. What
//! grants (a terminal reading itself) must be something a process can't choose, and its place in the
//! process tree is: no process can make itself a descendant of a shell it does not run under.

use std::time::SystemTime;

use crate::platform::process::{ancestors, start_time, tty};
use crate::platform::socket::{Stream, peer_pid};

/// How many parents are followed: deeper than any shell, script and tool chain, and a bound on the
/// system calls one request costs.
pub const ANCESTORS_MAX: usize = 32;

/// The process that connected, and where it runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Peer {
    /// The connecting process, when the system says (`LOCAL_PEERPID` on macOS) and it is the one that
    /// connected (see [`Peer::of`]).
    pub pid: Option<u32>,
    /// Its parent chain, nearest first, at most [`ANCESTORS_MAX`], stopping at process 1. Empty for a
    /// process whose parent has gone (it was given to launchd).
    pub ancestors: Vec<u32>,
    /// The device of its controlling terminal, which every process of one terminal shares, whatever
    /// became of its parents.
    pub tty: Option<u32>,
}

impl Peer {
    /// A peer seen only as a pid and the chain above it (tests, and a caller that knows both).
    pub fn new(pid: u32, ancestors: Vec<u32>) -> Self {
        Self { pid: Some(pid), ancestors, tty: None }
    }

    /// The same peer on the controlling terminal `tty`.
    #[must_use]
    pub fn on_tty(mut self, tty: u32) -> Self {
        self.tty = Some(tty);
        self
    }

    /// Who is at the other end of `stream`, which was accepted at `accepted`. A few system calls: for
    /// control requests only.
    pub(super) fn of(stream: &Stream, accepted: SystemTime) -> Self {
        Self::identify(peer_pid(stream), accepted, start_time, |pid| {
            (ancestors(pid, ANCESTORS_MAX), tty(pid))
        })
    }

    /// A pid is a number the system hands out again, so the process behind it is checked to be the one
    /// that connected: it must have started before the connection was accepted, and be the same
    /// process (the same start time) before and after what is read of it. A process that can't be read
    /// is not vouched for: no pid at all, so it can neither be refused as a terminal nor granted its
    /// reads, only be what it claimed.
    fn identify(
        pid: Option<u32>,
        accepted: SystemTime,
        start: impl Fn(u32) -> Option<SystemTime>,
        look: impl FnOnce(u32) -> (Vec<u32>, Option<u32>),
    ) -> Self {
        let Some(pid) = pid else { return Self::default() };
        let Some(before) = start(pid).filter(|started| *started <= accepted) else { return Self::default() };
        let (ancestors, tty) = look(pid);
        if start(pid) != Some(before) {
            return Self::default();
        }
        Self { pid: Some(pid), ancestors, tty }
    }

    /// Whether the caller runs inside the process `shell`: it is that process or below it. This is
    /// how a request is known to come from a terminal (its shell) and not from a claim.
    pub fn runs_under(&self, shell: u32) -> bool {
        self.pid == Some(shell) || self.ancestors.contains(&shell)
    }

    /// Whether the caller is in the terminal `shell` leads: under it, or, when its parents are gone
    /// (`(env -u WORMHOLE_TAB_ID wh send-text … &)` hands `wh` to launchd), on the same controlling
    /// terminal. This is a defence in part: a process that makes a session of its own (`setsid`) leaves
    /// both behind.
    pub fn runs_in(&self, shell: u32) -> bool {
        self.runs_under(shell) || self.tty.is_some_and(|mine| tty(shell) == Some(mine))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::Peer;

    #[test]
    fn a_caller_runs_under_a_shell_that_is_it_or_above_it() {
        let peer = Peer::new(300, vec![200, 100]);
        assert!(peer.runs_under(300) && peer.runs_under(200) && peer.runs_under(100));
        assert!(!peer.runs_under(400));
        assert!(!Peer::default().runs_under(100), "an unknown peer is under nothing");
    }

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn a_process_started_after_the_connection_is_not_the_one_that_connected() {
        let look = |_| (vec![5], Some(7));
        // The pid was reused by a process that began after the accept: nobody is vouched for.
        assert_eq!(Peer::identify(Some(9), at(100), |_| Some(at(101)), look), Peer::default());
        // The one that connected began before.
        let peer = Peer::identify(Some(9), at(100), |_| Some(at(50)), look);
        assert_eq!(peer, Peer { pid: Some(9), ancestors: vec![5], tty: Some(7) });
        // One that is gone, or can't be read, can't be vouched for either.
        assert_eq!(Peer::identify(Some(9), at(100), |_| None, look), Peer::default());
        assert_eq!(Peer::identify(None, at(100), |_| Some(at(50)), look), Peer::default());
    }

    #[test]
    fn a_pid_that_changes_hands_while_it_is_read_is_not_vouched_for() {
        let calls = std::cell::Cell::new(0);
        let start = |_| {
            calls.set(calls.get() + 1);
            Some(at(if calls.get() == 1 { 50 } else { 60 }))
        };
        assert_eq!(Peer::identify(Some(9), at(100), start, |_| (vec![5], None)), Peer::default());
    }

    /// `(env -u WORMHOLE_TAB_ID wh send-text … &)`: the parents are gone, the terminal is not.
    #[test]
    fn an_orphaned_caller_is_still_in_its_terminal_by_its_tty() {
        let orphan = Peer::new(900_001, vec![]).on_tty(123_456);
        assert!(!orphan.runs_under(40), "no chain left to walk");
        // `runs_in` asks the system for the shell's tty: a shell that does not exist has none.
        assert!(!orphan.runs_in(u32::MAX));
        // This very process is on whatever terminal it has; a peer on it runs in it.
        let mine = std::process::id();
        if let Some(tty) = crate::platform::process::tty(mine) {
            assert!(Peer::new(900_001, vec![]).on_tty(tty).runs_in(mine));
            assert!(!Peer::new(900_001, vec![]).on_tty(tty + 1).runs_in(mine));
        }
    }
}
