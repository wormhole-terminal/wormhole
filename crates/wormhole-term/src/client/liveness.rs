//! Is the host there? A host that stopped (a stopped or deadlocked process) keeps its socket open
//! and says nothing, which looks like a quiet terminal. The client pings a connection it is
//! watching (`PING_IDLE` of silence), and a ping nobody answers for [`PATIENCE`] marks the host
//! **not responding** (`session-host.md` section 11), until anything comes from it again. Pure
//! state over instants the caller passes in, so the rules are tested with a clock that moves when
//! a test says so.

use std::time::{Duration, Instant};

/// How long a ping may go unanswered before the host is called not responding.
pub const PATIENCE: Duration = Duration::from_secs(5);

/// What the client knows of the host's liveness.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Liveness {
    /// When anything last came from the host (any frame proves it is there).
    heard: Instant,
    /// When the oldest ping nothing has answered since went out.
    pending: Option<Instant>,
}

/// How the host stands at an instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Responsiveness {
    Responsive,
    /// A ping has gone unanswered for this long.
    NotResponding(Duration),
}

impl Liveness {
    pub(crate) fn new(now: Instant) -> Self {
        Self { heard: now, pending: None }
    }

    /// A frame came: the host is there, and no ping is owed an answer any more.
    pub(crate) fn heard(&mut self, now: Instant) {
        self.heard = now;
        self.pending = None;
    }

    /// A ping went out. Only the first of a silence counts: a later one does not move the clock.
    pub(crate) fn ping_sent(&mut self, now: Instant) {
        self.pending.get_or_insert(now);
    }

    /// When anything last came.
    pub(crate) fn last_heard(&self) -> Instant {
        self.heard
    }

    pub(crate) fn state(&self, now: Instant) -> Responsiveness {
        match self.pending {
            Some(since) if now.saturating_duration_since(since) >= PATIENCE => {
                Responsiveness::NotResponding(now.saturating_duration_since(since))
            }
            _ => Responsiveness::Responsive,
        }
    }

    /// When the state changes if nothing comes: the moment the pending ping runs out of patience.
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.pending.map(|since| since + PATIENCE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn a_pong_that_does_not_come_marks_the_host_unresponsive() {
        let t0 = Instant::now();
        let mut live = Liveness::new(t0);
        assert_eq!(live.state(t0), Responsiveness::Responsive);
        live.ping_sent(t0 + secs(2));
        assert_eq!(live.state(t0 + secs(6)), Responsiveness::Responsive, "4 s after the ping: still patient");
        assert_eq!(live.deadline(), Some(t0 + secs(7)));
        assert_eq!(
            live.state(t0 + secs(7)),
            Responsiveness::NotResponding(secs(5)),
            "5 s of silence after a ping"
        );
        assert_eq!(live.state(t0 + secs(12)), Responsiveness::NotResponding(secs(10)));
        // The host answers (or says anything): it is back.
        live.heard(t0 + secs(13));
        assert_eq!(live.state(t0 + secs(13)), Responsiveness::Responsive);
        assert_eq!(live.deadline(), None);
        assert_eq!(live.last_heard(), t0 + secs(13));
    }

    #[test]
    fn a_second_ping_does_not_restart_the_clock() {
        let t0 = Instant::now();
        let mut live = Liveness::new(t0);
        live.ping_sent(t0);
        live.ping_sent(t0 + secs(4));
        assert_eq!(live.state(t0 + secs(5)), Responsiveness::NotResponding(secs(5)));
    }

    #[test]
    fn a_host_that_answers_every_ping_is_never_called_unresponsive() {
        let t0 = Instant::now();
        let mut live = Liveness::new(t0);
        for i in 1..=20u64 {
            live.ping_sent(t0 + secs(i * 2));
            live.heard(t0 + secs(i * 2) + Duration::from_millis(3));
            assert_eq!(live.state(t0 + secs(i * 2 + 1)), Responsiveness::Responsive);
        }
    }
}
