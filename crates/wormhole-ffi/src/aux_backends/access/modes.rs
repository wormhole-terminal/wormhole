//! The mode in force, and a mode widened outside Wormhole (`agent-permissions.md` 9.1; the GPUI app's
//! `agent_access/change.rs`): a `config.json` edited by an editor, a script or a program must not let programs in by
//! itself. A change that widens access (Off to Ask, Off to Open, Ask to Open) and did not come from Settings keeps
//! the earlier mode in force and asks the person whether to keep it; narrowing applies at once. A question nobody
//! answers within a minute is a no, and is not remembered.
//!
//! What Settings did is known by its value, not by a count (GPUI counts edits, `change.rs:44-57`, and so lets through
//! whatever the file holds at the look after one): the mode the person set is in force from the look that sees their
//! edit, whatever the file holds by then, and a file that holds anything wider is asked about. A look is made from a
//! read of the file, and a read from before the mode last moved says nothing about it: such a look is refused, and
//! made again from a fresh read.

use wormhole_core::config::AgentAccess;

use crate::settings::edits::Edit;

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

/// One look at what the profile's file says.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look {
    /// The mode the file held when it was read.
    pub want: AgentAccess,
    /// The person's last change of the mode in Settings, read after the file was.
    pub edit: Option<Edit>,
    /// [`Modes::stamp`] from before the file was read.
    pub stamp: u64,
    /// Whether a widening may be asked about now: not while the Automation flag is off, when nobody is let in anyway.
    pub asks: bool,
}

/// What a profile's mode is, as the guard has seen it.
#[derive(Debug, Default)]
pub(crate) struct Modes {
    /// The mode in force. Unset until the first look: that is the mode Wormhole was started with.
    enforced: Option<AgentAccess>,
    /// The person's edit in Settings last taken in, by its serial.
    seen_edit: u64,
    change: Option<Change>,
    tokens: u64,
    /// How many times the mode in force, or the question about it, moved.
    moves: u64,
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
    /// What a look is stamped with: taken before the file is read, so a look whose read is older than the last
    /// move is told apart.
    pub(crate) fn stamp(&self) -> u64 {
        self.moves
    }

    /// The mode in force, with no look at the file.
    pub(crate) fn held(&self) -> Option<AgentAccess> {
        self.enforced
    }

    /// The mode in force after `look`, and whether this look is what raised a question about a widening. `None`
    /// when the look says nothing: the mode moved since its file was read (an answer put the earlier mode back, a
    /// look narrowed it), or the person's own change may not have been in the file yet. Read the file and look again.
    pub(crate) fn reconcile(&mut self, look: Look) -> Option<(AgentAccess, bool)> {
        if look.stamp != self.moves {
            return None;
        }
        if let Some(edit) = look.edit.filter(|edit| edit.serial != self.seen_edit) {
            // What the person set is in force, and is what an unanswered question goes back to.
            self.seen_edit = edit.serial;
            self.enforced = Some(edit.mode);
            if let Some(change) = &mut self.change {
                change.previous = edit.mode;
            }
            self.moves += 1;
            if look.want != edit.mode {
                // The file was read before their write landed, or something wrote after it: only a fresh read tells.
                return None;
            }
            return Some((edit.mode, false));
        }
        let Some(enforced) = self.enforced else {
            self.enforced = Some(look.want);
            self.moves += 1;
            return Some((look.want, false));
        };
        if look.want == enforced || self.change.is_some() {
            return Some((enforced, false));
        }
        if width(look.want) < width(enforced) {
            self.enforced = Some(look.want);
            self.moves += 1;
            return Some((look.want, false));
        }
        if !look.asks {
            return Some((enforced, false));
        }
        self.tokens += 1;
        self.moves += 1;
        self.change =
            Some(Change { previous: enforced, requested: look.want, token: self.tokens, asked: false });
        Some((enforced, true))
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
        // A read of the file from before the answer is no look at what the answer left.
        self.moves += 1;
        Some(change)
    }
}

#[cfg(test)]
mod tests {
    use AgentAccess::{Ask, Off, Open};

    use super::*;

    /// A look made from a read of the file as it is now, with nothing done in Settings since the last one.
    fn file(modes: &Modes, want: AgentAccess) -> Look {
        Look { want, edit: None, stamp: modes.stamp(), asks: true }
    }

    /// The same, after the person set `mode` in Settings as their edit number `serial`.
    fn after_edit(modes: &Modes, want: AgentAccess, serial: u64, mode: AgentAccess) -> Look {
        Look { edit: Some(Edit { serial, mode }), ..file(modes, want) }
    }

    fn started_with(mode: AgentAccess) -> Modes {
        let mut modes = Modes::default();
        let first = file(&modes, mode);
        assert_eq!(modes.reconcile(first), Some((mode, false)), "the first look is the mode at the start");
        modes
    }

    #[test]
    fn a_mode_widened_by_something_else_is_held_back_and_asked_about() {
        for (from, to) in [(Off, Ask), (Off, Open), (Ask, Open)] {
            let mut modes = started_with(from);
            let look = file(&modes, to);
            assert_eq!(modes.reconcile(look), Some((from, true)), "{from:?} to {to:?}");
            let look = file(&modes, to);
            assert_eq!(modes.reconcile(look), Some((from, false)), "asked once");
            assert!(modes.next_to_ask().is_some());
            assert!(modes.next_to_ask().is_none());
        }
    }

    #[test]
    fn narrowing_and_the_persons_own_widening_apply_at_once() {
        let mut modes = started_with(Open);
        let look = file(&modes, Off);
        assert_eq!(modes.reconcile(look), Some((Off, false)), "narrowed from outside");
        let look = after_edit(&modes, Open, 1, Open);
        assert_eq!(modes.reconcile(look), Some((Open, false)), "widened in Settings, and the file holds it");
    }

    /// Order: the person sets Ask in Settings; a program writes Open into the file before anything looked. The count
    /// of edits let Open through; the value does not.
    #[test]
    fn only_the_value_the_person_set_passes_without_a_question() {
        for from in [Off, Open] {
            let mut modes = started_with(from);
            let look = after_edit(&modes, Open, 1, Ask);
            assert_eq!(modes.reconcile(look), None, "the file holds another mode than theirs: read again");
            assert_eq!(modes.held(), Some(Ask), "what the person set is in force meanwhile, from {from:?}");
            let look = after_edit(&modes, Open, 1, Ask);
            assert_eq!(modes.reconcile(look), Some((Ask, true)), "the file still holds Open: asked about");
        }
    }

    /// Order: the person's change is noted, a look reads the file before their write lands, the write lands. The
    /// look from the old read is not taken for a narrowing made outside.
    #[test]
    fn a_read_from_before_the_persons_write_is_made_again() {
        let mut modes = started_with(Off);
        let early = after_edit(&modes, Off, 1, Open);
        assert_eq!(modes.reconcile(early), None);
        let look = after_edit(&modes, Open, 1, Open);
        assert_eq!(modes.reconcile(look), Some((Open, false)), "their Open is in force, with nothing asked");
    }

    /// Order: the Automation flag is off while the person sets Open in Settings and then narrows the file to Ask by
    /// hand; later a program writes Open and turns the flag on. Their Settings change is no credit for that.
    #[test]
    fn a_look_while_automation_is_off_follows_the_mode_and_asks_nothing() {
        let mut modes = started_with(Off);
        let quiet = |modes: &Modes, want| Look { asks: false, ..file(modes, want) };
        let look = Look { asks: false, ..after_edit(&modes, Open, 1, Open) };
        assert_eq!(modes.reconcile(look), Some((Open, false)), "set in Settings while the flag is off");
        let look = quiet(&modes, Ask);
        assert_eq!(modes.reconcile(look), Some((Ask, false)), "narrowed by hand while the flag is off");
        let look = quiet(&modes, Open);
        assert_eq!(modes.reconcile(look), Some((Ask, false)), "widened outside while off: held, not asked");
        assert!(modes.next_to_ask().is_none());
        let look = after_edit(&modes, Open, 1, Open);
        assert_eq!(modes.reconcile(look), Some((Ask, true)), "the flag is on: their edit counted once");
    }

    /// Order: the file was read while it held the wider mode; the person says Don't Keep, which writes the earlier
    /// mode back; a request then looks with the read from before. And: a look narrows the mode, a request then looks
    /// with a wider read from before that.
    #[test]
    fn a_look_from_a_read_made_before_the_mode_moved_is_refused() {
        let mut modes = started_with(Off);
        let look = file(&modes, Open);
        modes.reconcile(look);
        let stale = file(&modes, Open);
        let token = modes.next_to_ask().map(|change| change.token).expect("asked");
        assert_eq!(modes.answered(token, false).map(|change| change.previous), Some(Off));
        assert_eq!(modes.reconcile(stale), None, "the question does not come back from the old read");
        let look = file(&modes, Off);
        assert_eq!(modes.reconcile(look), Some((Off, false)), "the file has the earlier mode back");
        assert!(modes.next_to_ask().is_none());

        let mut modes = started_with(Open);
        let stale = file(&modes, Open);
        let look = file(&modes, Ask);
        assert_eq!(modes.reconcile(look), Some((Ask, false)), "narrowed by hand");
        assert_eq!(modes.reconcile(stale), None, "the wider read from before is no widening");
    }

    #[test]
    fn keeping_makes_the_new_mode_the_one_in_force_and_not_keeping_leaves_the_old() {
        let mut modes = started_with(Off);
        let look = file(&modes, Open);
        modes.reconcile(look);
        let token = modes.next_to_ask().map(|change| change.token).expect("asked");
        assert_eq!(modes.answered(token + 1, true), None, "another question's answer");
        assert_eq!(modes.answered(token, true).map(|change| change.requested), Some(Open));
        let look = file(&modes, Open);
        assert_eq!(modes.reconcile(look), Some((Open, false)));

        let mut modes = started_with(Off);
        let look = file(&modes, Ask);
        modes.reconcile(look);
        let token = modes.next_to_ask().map(|change| change.token).expect("asked");
        assert_eq!(modes.answered(token, false).map(|change| change.previous), Some(Off));
        let look = file(&modes, Off);
        assert_eq!(modes.reconcile(look), Some((Off, false)), "the file has the old mode back");
        assert_eq!(modes.answered(token, true), None, "answered once");
    }

    /// Order: a widening made outside is asked about; before the person answers they set a mode in Settings. Don't
    /// Keep then goes back to what they set, not to the mode from before it.
    #[test]
    fn a_change_in_settings_while_the_question_is_up_is_what_dont_keep_goes_back_to() {
        let mut modes = started_with(Ask);
        let look = file(&modes, Open);
        modes.reconcile(look);
        let token = modes.next_to_ask().map(|change| change.token).expect("asked");
        let look = after_edit(&modes, Off, 1, Off);
        assert_eq!(modes.reconcile(look), Some((Off, false)));
        assert_eq!(modes.answered(token, false).map(|change| change.previous), Some(Off));
    }
}
