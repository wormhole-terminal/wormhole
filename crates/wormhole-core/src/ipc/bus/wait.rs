//! The bus's side of `wait` and `run`: a waiter is a subscription like a stream's, with a filter of the
//! events its conditions need (a terminal's state, its commands, its end) and, when it looks at the
//! screen, a note that it wants to hear when the terminal prints.
//!
//! The note costs nothing while nobody waits: it is one bit of the interest mask the app reads before
//! it emits anything (`Bus::wants_prints`), and what the app does for a print is to poke the waiters of
//! that terminal (`Bus::pulse`): a flag and a wake-up, coalesced however often the terminal prints. The
//! waiter's own thread looks at the screen, at most every [`LOOK_EVERY`], never the printing thread and
//! never the UI.

use std::sync::Arc;
use std::time::Duration;

use super::{Bus, Caller, Grant, Refused, Started, Subscription};
use crate::ipc::control::event::{EventFilter, Topics};
use crate::ipc::control::wait::{WaitPlan, run_topics};

/// A waiter looks at a screen at most this often.
pub const LOOK_EVERY: Duration = Duration::from_millis(100);

/// What a waiter watches of its terminal beyond the events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Watch {
    pub terminal: i64,
    /// It wants to hear when the terminal prints.
    pub screen: bool,
    /// It reads the screen's text for its conditions, so it ends when the caller may no longer read.
    pub needs_read: bool,
}

impl Watch {
    /// What a `wait` with `plan` watches of `terminal`.
    pub fn of_wait(terminal: i64, plan: &WaitPlan) -> (Self, Topics) {
        (Self { terminal, screen: plan.watches_screen(), needs_read: plan.needs_read() }, plan.topics())
    }

    /// What a `run` in `terminal` watches: no screen, its output is read once at the end.
    pub fn of_run(terminal: i64) -> (Self, Topics) {
        (Self { terminal, screen: false, needs_read: false }, run_topics())
    }
}

impl Bus {
    /// A subscription for a waiter on `watch.terminal`, hearing `topics` of it. It counts among the
    /// streams (the same caps), and is given back when dropped.
    pub fn wait(
        self: &Arc<Self>,
        (watch, topics): (Watch, Topics),
        grant: Grant,
        caller: Caller,
    ) -> Result<(Subscription, Started), Refused> {
        let filter = EventFilter { topics, terminals: vec![watch.terminal], windows: Vec::new() };
        self.register(filter, Some(watch), grant, caller)
    }

    /// Whether somebody wants to hear that terminals print: an `output` stream, or a waiter on a
    /// screen. One relaxed load; the app asks before it emits anything about a print.
    pub fn wants_prints(&self) -> bool {
        self.interest_bits() & (super::Topic::Output.bit() | super::SCREEN_BIT) != 0
    }

    /// Claim `terminal` for a `run`: `None` while another is under way in it. A command is typed a moment
    /// before its shell marks it as running, and until then the shell still looks to be at its prompt, so a
    /// second `run` would pass the checks, clear the line of the first and both would report the second.
    /// The claim is the job's and ends with it, wherever it ends.
    pub fn claim_run(self: &Arc<Self>, terminal: i64) -> Option<RunClaim> {
        let mut state = self.lock();
        if state.runs.contains(&terminal) {
            return None;
        }
        state.runs.push(terminal);
        Some(RunClaim { bus: self.clone(), terminal })
    }

    /// Wake the waiters that watch `terminal`'s screen.
    pub(super) fn poke_screens(&self, terminal: i64) {
        for subscriber in self.lock().subscribers.iter() {
            if subscriber.watch.is_some_and(|watch| watch.screen && watch.terminal == terminal) {
                subscriber.queue.poke();
            }
        }
    }
}

/// A terminal held for a `run` ([`Bus::claim_run`]); given back when dropped.
#[derive(Debug)]
pub struct RunClaim {
    bus: Arc<Bus>,
    terminal: i64,
}

impl Drop for RunClaim {
    fn drop(&mut self) {
        self.bus.lock().runs.retain(|&held| held != self.terminal);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Grant, Watch};
    use crate::ipc::bus::{Bus, Next, STREAMS_PER_CALLER};
    use crate::ipc::control::event::{Body, CloseReason, Event, Topic};
    use crate::ipc::control::wait::{WaitSpec, run_topics};
    use crate::ipc::control::{Target, TerminalState};

    fn plan(f: impl FnOnce(&mut WaitSpec)) -> crate::ipc::control::WaitPlan {
        let mut spec = WaitSpec {
            target: Target::Terminal(3),
            status: Vec::new(),
            pattern: None,
            gone: None,
            stable_ms: None,
            block_done: false,
            after: None,
            timeout_ms: None,
        };
        f(&mut spec);
        spec.plan().expect("a plan")
    }

    fn status(terminal: i64, now: TerminalState) -> Event {
        Event {
            window: Some(1),
            terminal: Some(terminal),
            body: Body::Status { status: now, was: TerminalState::Idle, agent: None, message: None },
        }
    }

    #[test]
    fn a_waiter_hears_the_events_of_its_terminal_it_asked_for_and_nothing_else() {
        let bus = Bus::new();
        let by_status = plan(|s| s.status = vec![TerminalState::Finished]);
        let (waiter, _) = bus.wait(Watch::of_wait(3, &by_status), Grant::default(), Some(7)).expect("room");
        assert!(bus.wants(Topic::Status) && bus.wants(Topic::Terminals) && !bus.wants(Topic::Blocks));
        bus.publish(Topic::Status, || status(4, TerminalState::Finished));
        assert!(matches!(waiter.next(Duration::ZERO), Next::Idle), "another terminal's");
        bus.publish(Topic::Status, || status(3, TerminalState::Finished));
        assert!(matches!(waiter.next(Duration::ZERO), Next::Event(_)));
        drop(waiter);
        assert!(!bus.wants(Topic::Status), "nothing wanted once it is gone");
    }

    #[test]
    fn a_print_wakes_the_waiters_of_that_terminal_once_however_often_it_prints() {
        let bus = Bus::new();
        assert!(!bus.wants_prints());
        let looking = plan(|s| s.pattern = Some("ok".into()));
        let (waiter, _) = bus.wait(Watch::of_wait(3, &looking), Grant::default(), None).expect("room");
        let quiet = plan(|s| s.block_done = true);
        let (other, _) = bus.wait(Watch::of_wait(3, &quiet), Grant::default(), None).expect("room");
        assert!(bus.wants_prints(), "a waiter on a screen is a listener to prints");
        for _ in 0..50 {
            bus.pulse(1, 3, || None);
        }
        bus.pulse(1, 4, || None);
        assert!(matches!(waiter.next(Duration::ZERO), Next::Changed));
        assert!(matches!(waiter.next(Duration::ZERO), Next::Idle), "fifty prints, one wake-up");
        assert!(matches!(other.next(Duration::ZERO), Next::Idle), "it does not look at the screen");
        drop(waiter);
        assert!(!bus.wants_prints(), "the last one gone, nobody listens");
    }

    #[test]
    fn waiters_count_among_the_streams() {
        let bus = Bus::new();
        let (watch, topics) = Watch::of_run(3);
        let mut held = Vec::new();
        for _ in 0..STREAMS_PER_CALLER {
            held.push(bus.wait((watch, topics), Grant::default(), Some(7)).expect("room"));
        }
        assert!(bus.wait((watch, topics), Grant::default(), Some(7)).is_err(), "one caller's share");
        assert!(bus.wait((watch, topics), Grant::default(), Some(8)).is_ok(), "another caller's is its own");
        assert_eq!(run_topics(), topics);
    }

    #[test]
    fn a_terminal_is_held_for_one_run_at_a_time_until_the_claim_is_given_back() {
        let bus = Bus::new();
        let first = bus.claim_run(3).expect("free");
        assert!(bus.claim_run(3).is_none(), "one is under way");
        assert!(bus.claim_run(4).is_some(), "another terminal is its own");
        drop(first);
        assert!(bus.claim_run(3).is_some(), "free again");
    }

    #[test]
    fn reading_going_off_ends_a_waiter_on_text_but_not_one_in_its_own_terminal() {
        let bus = Bus::new();
        let reads = plan(|s| s.pattern = Some("ok".into()));
        let granted = Grant { reads_all: true, own: vec![9], ..Grant::default() };
        let (on_text, _) = bus.wait(Watch::of_wait(3, &reads), granted.clone(), None).expect("room");
        let (own, _) = bus.wait(Watch::of_wait(9, &reads), granted.clone(), None).expect("room");
        let status_only = plan(|s| s.status = vec![TerminalState::Idle]);
        let (no_text, _) = bus.wait(Watch::of_wait(3, &status_only), granted, None).expect("room");
        assert!(on_text.may_read(3));
        bus.revoke_reads();
        assert!(matches!(on_text.next(Duration::ZERO), Next::Closed(CloseReason::NotAllowed)));
        assert!(!on_text.may_read(3));
        assert!(matches!(own.next(Duration::ZERO), Next::Idle), "it reads itself");
        assert!(own.may_read(9));
        assert!(matches!(no_text.next(Duration::ZERO), Next::Idle), "it never read");
    }
}
