//! The status machine's small decisions, each a pure function of values: what a bell does, whether
//! output is only an answer to the person, whether a program has left, and how fast a title may
//! rename a terminal. They are the rules of the app's `terminal_view/status.rs`, with the clock as a
//! `Millis` instead of an `Instant`.

use crate::view::sidebar::Status;

use super::super::facts::Millis;

/// What a bell does to a terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Bell {
    /// The row shows the bell until someone looks.
    pub mark: bool,
    /// The terminal needs input now.
    pub needs_you: bool,
}

/// How often a terminal without an agent may count as needing you for its bell.
pub(super) const BELL_COUNTS_EVERY: Millis = 60_000;

/// What the bell does. A terminal whose status is told (`reported`: an agent's hooks, or a
/// program's own reports) ignores it: they say exactly when it needs input. An agent without hooks
/// rings to ask, so it needs input. A terminal without an agent marks the bell until you look, and
/// counts as needing you at most once a minute (`since_counted`: how long ago it last did), so a
/// program ringing in a loop doesn't keep asking. An ended shell ignores it.
pub(super) fn on_bell(status: Status, reported: bool, agent: bool, since_counted: Option<Millis>) -> Bell {
    match (status, reported, agent) {
        (Status::Exited, _, _) | (_, true, _) => Bell { mark: false, needs_you: false },
        (_, false, true) => Bell { mark: false, needs_you: true },
        (_, false, false) => {
            Bell { mark: true, needs_you: since_counted.is_none_or(|since| since >= BELL_COUNTS_EVERY) }
        }
    }
}

/// The shell has the terminal back after a program had it: that program's process is gone, and with
/// it whatever only it could say (a crash leaves no word of its own). `now` is `None` where the
/// platform can't tell, which says nothing either way.
pub(super) fn program_left_the_terminal(was_in_front: bool, now: Option<bool>) -> bool {
    was_in_front && now == Some(false)
}

/// When the person last did something that makes a program print: typed, resized, told it of a focus
/// change.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Answered {
    pub input: Option<Millis>,
    pub resize: Option<Millis>,
    pub focus: Option<Millis>,
}

impl Answered {
    /// Whether output arriving `now` answers something Wormhole did rather than being the program's
    /// own work: the echo of typing, or the redraw after a resize or a focus report. An agent without
    /// hooks redraws when Wormhole goes behind; counted as work, that settles into "done" and a
    /// notification nothing finished for.
    pub(super) fn covers(&self, now: Millis) -> bool {
        let within =
            |at: Option<Millis>, window: Millis| at.is_some_and(|at| now.saturating_sub(at) < window);
        within(self.input, 300) || within(self.resize, 700) || within(self.focus, 700)
    }
}

/// How often the program's title may rename a terminal.
pub(super) const TITLE_EVERY: Millis = 250;

/// Holds a program's title changes to one rename per [`TITLE_EVERY`]; the latest one wins. A
/// program counting its progress in the title renames at that pace, not per frame (every rename
/// redraws the sidebar).
#[derive(Debug, Default)]
pub(super) struct TitleGate {
    /// The title waiting to be shown, `Some(None)` for a reset.
    pending: Option<Option<String>>,
    /// The title as last shown, so a change back to it is no change.
    shown: Option<String>,
    shown_at: Option<Millis>,
}

impl TitleGate {
    /// A new title from the program (`None`: it was reset).
    pub(super) fn offer(&mut self, title: Option<String>) {
        self.pending = (title != self.shown).then_some(title);
    }

    /// The title to show now, if one is waiting and the pace allows it.
    pub(super) fn take_due(&mut self, now: Millis) -> Option<Option<String>> {
        if self.pending.is_none() || self.wait(now) > 0 {
            return None;
        }
        let title = self.pending.take()?;
        self.shown.clone_from(&title);
        self.shown_at = Some(now);
        Some(title)
    }

    /// When the waiting title is due, if one waits.
    pub(super) fn due_at(&self) -> Option<Millis> {
        self.pending.as_ref()?;
        Some(self.shown_at.map_or(0, |at| at + TITLE_EVERY))
    }

    fn wait(&self, now: Millis) -> Millis {
        self.shown_at.map_or(0, |at| TITLE_EVERY.saturating_sub(now.saturating_sub(at)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_has_left_when_the_shell_has_the_terminal_back() {
        assert!(program_left_the_terminal(true, Some(false)));
        assert!(!program_left_the_terminal(true, Some(true)), "still there");
        assert!(!program_left_the_terminal(false, Some(false)), "never seen in front");
        assert!(!program_left_the_terminal(true, None), "the platform can't tell");
    }

    #[test]
    fn the_bell_asks_for_you_only_where_it_means_something() {
        let (quiet, ask, mark) = (
            Bell { mark: false, needs_you: false },
            Bell { mark: false, needs_you: true },
            Bell { mark: true, needs_you: true },
        );
        assert_eq!(
            on_bell(Status::Busy, true, true, None),
            quiet,
            "hooks or reports say when it needs input"
        );
        assert_eq!(on_bell(Status::Busy, false, true, None), ask, "an agent without hooks rings to ask");
        assert_eq!(
            on_bell(Status::Idle, false, false, None),
            mark,
            "a plain terminal: the bell, and needs input"
        );
        assert_eq!(on_bell(Status::Exited, false, false, None), quiet);
        // Again within the minute: marked, but it doesn't ask again.
        let soon = on_bell(Status::Idle, false, false, Some(10_000));
        assert_eq!(soon, Bell { mark: true, needs_you: false });
        assert_eq!(on_bell(Status::Idle, false, false, Some(BELL_COUNTS_EVERY)), mark);
    }

    #[test]
    fn a_redraw_after_what_the_person_did_is_not_work() {
        let answered = Answered { input: Some(0), resize: Some(0), focus: Some(0) };
        assert!(answered.covers(200), "the echo of typing");
        assert!(answered.covers(500), "a resize's redraw");
        // Output well after any of them is the program's own.
        assert!(!answered.covers(800));
        let long_ago = Answered { input: Some(0), resize: Some(0), focus: Some(60_000) };
        assert!(long_ago.covers(60_200), "Wormhole went behind and told the program; it redraws");
        assert!(!Answered::default().covers(10), "nothing was ever done");
    }

    #[test]
    fn a_title_that_keeps_changing_renames_the_terminal_at_a_steady_pace() {
        let title = |t: &str| Some(t.to_string());
        let mut gate = TitleGate::default();
        // The first title shows at once.
        gate.offer(title("1%"));
        assert_eq!(gate.take_due(0), Some(title("1%")));
        // The next ones within the pace wait; the latest wins, the ones in between never show.
        gate.offer(title("2%"));
        assert_eq!((gate.take_due(50), gate.due_at()), (None, Some(TITLE_EVERY)));
        gate.offer(title("3%"));
        assert_eq!(gate.take_due(TITLE_EVERY), Some(title("3%")));
        // The same title again is no rename, and drops one waiting.
        gate.offer(title("4%"));
        gate.offer(title("3%"));
        assert_eq!((gate.take_due(TITLE_EVERY * 3), gate.due_at()), (None, None));
        // A reset is a change like any other.
        gate.offer(None);
        assert_eq!(gate.take_due(TITLE_EVERY * 3), Some(None));
    }
}
