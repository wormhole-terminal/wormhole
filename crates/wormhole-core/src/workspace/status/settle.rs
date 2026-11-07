//! Busy → done: a terminal whose output went quiet, or whose command finished, settles.
//!
//! A deadline, so the shell arms one timer for it ([`StatusMachine::next_due`]) rather than polling:
//! later output only moves the deadline, which the tick checks when it comes, so a printing terminal
//! costs a comparison per event. The terminal can't settle itself without the window: what it settles
//! into depends on whether it is in front (a terminal in front goes idle, not done).

use crate::view::sidebar::Status;

use super::super::facts::{Millis, TerminalFacts};
use super::{Context, StatusMachine};

/// How long a busy terminal prints nothing before it counts as done.
pub(super) const QUIET: Millis = 1_800;

impl StatusMachine {
    /// When a busy terminal is due to settle: `None` when nothing waits on it.
    pub(super) fn settle_at(&self, facts: &TerminalFacts) -> Option<Millis> {
        (!self.reported() && facts.status == Status::Busy).then(|| self.last_output + QUIET)
    }

    /// Settle busy → idle/done/failed: once the output has been quiet for a while, or at once when
    /// the shell's marks said the command finished. Seen at once, there is nothing to settle into: the
    /// result is on screen (and a failure's cue stays on the row).
    pub(super) fn settle(&mut self, facts: &mut TerminalFacts, at: Millis, cx: Context) {
        if self.reported() {
            return;
        }
        let finished = std::mem::take(&mut self.finished_mark);
        let looking = cx.looking();
        // The failure is this command's when the mark just came: it replaced the last one's.
        let failed = finished && facts.failure.is_some();
        if facts.status == Status::Busy && (finished || at.saturating_sub(self.last_output) >= QUIET) {
            let settled = match (looking, failed) {
                (true, _) => Status::Idle,
                (false, true) => Status::Failed,
                (false, false) => Status::Done,
            };
            self.set_status(facts, settled, at);
        } else if failed && !looking && facts.status == Status::Done {
            // A command that went quiet counted as done; its exit code, when it came, says it failed.
            self.set_status(facts, Status::Failed, at);
        }
    }
}
