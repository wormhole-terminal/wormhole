//! The mode in force, and a mode widened outside Wormhole (`agent-permissions.md` 9.1; the GPUI app's
//! `agent_access/change.rs`): a `config.json` edited by an editor, a script or a program must not let programs in by
//! itself. A change that widens access (Off to Ask, Off to Open, Ask to Open) and did not come from Settings keeps
//! the earlier mode in force and asks the person whether to keep it; narrowing, and anything done in Settings,
//! applies at once. A question nobody answers within a minute is a no, and is not remembered.

use wormhole_core::config::AgentAccess;

/// A mode changed outside Wormhole to something wider, waiting for the person's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    /// The mode that stays in force until the person answers, and comes back if they say no.
    pub previous: AgentAccess,
    pub requested: AgentAccess,
    /// Tells this question from an earlier one when its minute's timer fires.
    pub token: u64,
    /// The question was handed to the window.
    pub asked: bool,
}

/// What a profile's mode is, as the guard has seen it.
#[derive(Debug, Default)]
pub(crate) struct Modes {
    /// The mode in force. Unset until the first look: that is the mode Wormhole was started with.
    enforced: Option<AgentAccess>,
    /// How many edits Settings had made at the last look.
    seen_edits: Option<u64>,
    change: Option<Change>,
    tokens: u64,
}

/// How much a mode lets in.
fn width(mode: AgentAccess) -> u8 {
    match mode {
        AgentAccess::Off => 0,
        AgentAccess::Ask => 1,
        AgentAccess::Open => 2,
    }
}

impl Modes {
    /// The mode in force when the settings say `want` and Settings has made `edits` changes so far; whether this
    /// look is what raised a question about a widening.
    pub(crate) fn reconcile(&mut self, want: AgentAccess, edits: u64) -> (AgentAccess, bool) {
        let seen = self.seen_edits.replace(edits);
        let Some(enforced) = self.enforced else {
            self.enforced = Some(want);
            return (want, false);
        };
        if want == enforced || self.change.is_some() {
            return (enforced, false);
        }
        if seen != Some(edits) || width(want) < width(enforced) {
            self.enforced = Some(want);
            return (want, false);
        }
        self.tokens += 1;
        self.change = Some(Change { previous: enforced, requested: want, token: self.tokens, asked: false });
        (enforced, true)
    }

    /// The question to put on the screen now: the change, once.
    pub(crate) fn next_to_ask(&mut self) -> Option<Change> {
        let change = self.change.as_mut().filter(|change| !change.asked)?;
        change.asked = true;
        Some(change.clone())
    }

    /// Whether question `token` is the one waiting.
    pub(crate) fn is_waiting(&self, token: u64) -> bool {
        self.change.as_ref().is_some_and(|change| change.token == token && change.asked)
    }

    /// The change question `token` is, while it waits.
    pub(crate) fn pending(&self, token: u64) -> Option<Change> {
        self.change.clone().filter(|change| change.token == token && change.asked)
    }

    /// The person's answer to question `token`: the change that was settled (to put the earlier mode back in the
    /// file when it was not kept), `None` when that question is not the one waiting.
    pub(crate) fn answered(&mut self, token: u64, keep: bool) -> Option<Change> {
        if !self.is_waiting(token) {
            return None;
        }
        let change = self.change.take()?;
        if keep {
            self.enforced = Some(change.requested);
        }
        Some(change)
    }
}

#[cfg(test)]
mod tests {
    use AgentAccess::{Ask, Off, Open};

    use super::*;

    #[test]
    fn the_first_look_is_the_mode_the_app_started_with() {
        let mut modes = Modes::default();
        assert_eq!(modes.reconcile(Open, 0), (Open, false));
    }

    #[test]
    fn a_mode_widened_by_something_else_is_held_back_and_asked_about() {
        for (from, to) in [(Off, Ask), (Off, Open), (Ask, Open)] {
            let mut modes = Modes::default();
            modes.reconcile(from, 3);
            assert_eq!(modes.reconcile(to, 3), (from, true), "{from:?} to {to:?}");
            assert_eq!(modes.reconcile(to, 3), (from, false), "asked once");
            assert!(modes.next_to_ask().is_some());
            assert!(modes.next_to_ask().is_none());
        }
    }

    #[test]
    fn narrowing_and_a_change_in_settings_apply_at_once() {
        let mut modes = Modes::default();
        modes.reconcile(Open, 0);
        assert_eq!(modes.reconcile(Off, 0), (Off, false), "narrowed from outside");
        assert_eq!(modes.reconcile(Open, 1), (Open, false), "widened in Settings (one more edit)");
    }

    #[test]
    fn keeping_makes_the_new_mode_the_one_in_force_and_not_keeping_leaves_the_old() {
        let mut modes = Modes::default();
        modes.reconcile(Off, 0);
        modes.reconcile(Open, 0);
        let token = modes.next_to_ask().map(|change| change.token).expect("asked");
        assert_eq!(modes.answered(token + 1, true), None, "another question's answer");
        assert_eq!(modes.answered(token, true).map(|change| change.requested), Some(Open));
        assert_eq!(modes.reconcile(Open, 0), (Open, false));

        let mut modes = Modes::default();
        modes.reconcile(Off, 0);
        modes.reconcile(Ask, 0);
        let token = modes.next_to_ask().map(|change| change.token).expect("asked");
        assert_eq!(modes.answered(token, false).map(|change| change.previous), Some(Off));
        assert_eq!(modes.reconcile(Off, 0), (Off, false), "the file has the old mode back");
        assert_eq!(modes.answered(token, true), None, "answered once");
    }
}
