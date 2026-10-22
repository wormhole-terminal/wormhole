//! The decision about one terminal's clipboard reads.
//!
//! A read waits for the person only when it can be seen and nothing else is waiting: a terminal
//! that is not on screen cannot show the question, so it is refused rather than left to wait behind
//! a window nobody looks at (a refusal is what every other way of not answering comes to, and the
//! program can ask again once the terminal is shown); a second read while one waits is refused
//! too, so a program cannot queue questions until one gets clicked through.

use std::time::Duration;

use super::ClipboardRead;

/// How long a question waits before it counts as "Deny". The replies the program asked for after
/// the read wait behind it (they keep their order), so it must not be long.
pub const ASK_TIMEOUT: Duration = Duration::from_secs(30);

/// What to do with a read that has just been asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Give the program the clipboard.
    Answer,
    /// Give it nothing (answer with an empty reply so the ones behind it go out).
    Deny,
    /// Put the question to the person and settle it with [`ReadGate::resolve`] and this number
    /// (a [`Choice::Deny`] when nobody answers within [`ASK_TIMEOUT`]).
    Ask(u64),
}

/// What the person chose on the question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// This read only.
    Once,
    /// This read and every later one from this terminal, until it closes.
    Always,
    Deny,
}

/// One terminal's reads: whether it was granted "always", and the question that is out.
#[derive(Debug, Default)]
pub struct ReadGate {
    always: bool,
    pending: Option<u64>,
    asked: u64,
}

impl ReadGate {
    /// A read arrived. `on_screen` is whether the terminal is shown in a window the person is
    /// looking at.
    pub fn request(&mut self, policy: ClipboardRead, on_screen: bool) -> Request {
        match policy {
            ClipboardRead::Deny => Request::Deny,
            ClipboardRead::Allow => Request::Answer,
            // "Always" is the person's word for this terminal, given on a question they saw.
            ClipboardRead::Ask if self.always => Request::Answer,
            ClipboardRead::Ask if self.pending.is_some() || !on_screen => Request::Deny,
            ClipboardRead::Ask => {
                self.asked += 1;
                self.pending = Some(self.asked);
                Request::Ask(self.asked)
            }
        }
    }

    /// The question `ask` was settled. `Some(allowed)` if it was the one out, which is now closed;
    /// `None` for one that was settled already (a timeout after a click), which has nothing left
    /// to answer.
    pub fn resolve(&mut self, ask: u64, choice: Choice) -> Option<bool> {
        if self.pending != Some(ask) {
            return None;
        }
        self.pending = None;
        self.always |= choice == Choice::Always;
        Some(choice != Choice::Deny)
    }

    /// The question that is out, if any.
    pub fn pending(&self) -> Option<u64> {
        self.pending
    }
}

#[cfg(test)]
mod tests {
    use super::{Choice, ReadGate, Request};
    use crate::clipboard::ClipboardRead::{Allow, Ask, Deny};

    #[test]
    fn allow_answers_and_deny_refuses_without_asking() {
        let mut gate = ReadGate::default();
        assert_eq!(gate.request(Allow, true), Request::Answer);
        assert_eq!(gate.request(Allow, false), Request::Answer);
        assert_eq!(gate.request(Deny, true), Request::Deny);
        assert_eq!(gate.pending(), None, "neither put a question out");
    }

    #[test]
    fn ask_puts_one_question_out_and_refuses_the_ones_that_come_while_it_waits() {
        let mut gate = ReadGate::default();
        let Request::Ask(first) = gate.request(Ask, true) else { panic!("asks") };
        assert_eq!(gate.pending(), Some(first));
        assert_eq!(gate.request(Ask, true), Request::Deny, "one question at a time");
        assert_eq!(gate.pending(), Some(first), "and the refused one did not replace it");
    }

    #[test]
    fn a_terminal_nobody_can_see_is_refused_not_asked() {
        let mut gate = ReadGate::default();
        assert_eq!(gate.request(Ask, false), Request::Deny);
        assert_eq!(gate.pending(), None);
        assert!(matches!(gate.request(Ask, true), Request::Ask(_)), "asked once it is shown");
    }

    #[test]
    fn once_answers_this_read_and_the_next_asks_again() {
        let mut gate = ReadGate::default();
        let Request::Ask(first) = gate.request(Ask, true) else { panic!("asks") };
        assert_eq!(gate.resolve(first, Choice::Once), Some(true));
        assert_eq!(gate.pending(), None);
        let Request::Ask(second) = gate.request(Ask, true) else { panic!("asks again") };
        assert_ne!(first, second);
    }

    #[test]
    fn always_answers_every_later_read_of_this_terminal_even_out_of_sight() {
        let mut gate = ReadGate::default();
        let Request::Ask(first) = gate.request(Ask, true) else { panic!("asks") };
        assert_eq!(gate.resolve(first, Choice::Always), Some(true));
        assert_eq!(gate.request(Ask, true), Request::Answer);
        assert_eq!(gate.request(Ask, false), Request::Answer);
        assert_eq!(gate.request(Deny, true), Request::Deny, "the setting still wins");
        assert_eq!(gate.request(Allow, true), Request::Answer);
    }

    #[test]
    fn deny_closes_the_question_and_grants_nothing() {
        let mut gate = ReadGate::default();
        let Request::Ask(first) = gate.request(Ask, true) else { panic!("asks") };
        assert_eq!(gate.resolve(first, Choice::Deny), Some(false));
        assert_eq!(gate.pending(), None, "the slot is free for the next question");
        assert!(matches!(gate.request(Ask, true), Request::Ask(_)));
    }

    #[test]
    fn a_question_settled_twice_is_answered_once() {
        // The click and the timeout race; whichever comes second finds nothing to answer.
        let mut gate = ReadGate::default();
        let Request::Ask(first) = gate.request(Ask, true) else { panic!("asks") };
        assert_eq!(gate.resolve(first, Choice::Once), Some(true));
        assert_eq!(gate.resolve(first, Choice::Deny), None);
        // And an old timeout does not close the question that replaced it.
        let Request::Ask(second) = gate.request(Ask, true) else { panic!("asks") };
        assert_eq!(gate.resolve(first, Choice::Deny), None);
        assert_eq!(gate.pending(), Some(second));
    }

    #[test]
    fn the_question_that_is_out_always_resolves_to_a_verdict_for_its_reply() {
        // The reply slot is released by an answer or a refusal; no choice leaves the question
        // open, so a reply is never held for a later request to close.
        for choice in [Choice::Once, Choice::Always, Choice::Deny] {
            let mut gate = ReadGate::default();
            let Request::Ask(n) = gate.request(Ask, true) else { panic!("asks") };
            assert!(gate.resolve(n, choice).is_some(), "{choice:?}");
            assert_eq!(gate.pending(), None);
        }
    }
}
