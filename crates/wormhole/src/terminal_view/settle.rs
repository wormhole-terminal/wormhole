//! Busy → done: a terminal whose output went quiet, or whose command finished, settles.
//!
//! A deadline, so a one-shot timer rather than a periodic check: it is armed when a terminal is busy,
//! and fires when the output has been quiet for [`QUIET`] (later output only moves the deadline, which
//! the timer checks when it fires, so a printing terminal costs a comparison per event, not a task). The
//! terminal can't settle itself, because the result depends on what the window shows (a terminal in front
//! goes idle, not done): the timer asks the window with [`TerminalEvent::Quiet`], which calls
//! [`TerminalView::tick`].

use std::time::{Duration, Instant};

use gpui_kit::*;

use super::TerminalView;
use super::status::{Status, TerminalEvent};

/// How long a busy terminal prints nothing before it counts as done.
const QUIET: Duration = Duration::from_millis(1800);

impl TerminalView {
    /// How long until this terminal is due to settle: `None` when nothing waits on it.
    fn settle_in(&self, now: Instant) -> Option<Duration> {
        if self.reported() {
            return None;
        }
        if self.finished_mark {
            return Some(Duration::ZERO);
        }
        (self.status == Status::Busy)
            .then(|| QUIET.saturating_sub(now.saturating_duration_since(self.last_output)))
    }

    /// Arm the timer if the terminal is busy (or a command just finished) and none runs. Called after
    /// anything that may have made it busy; a status change cancels the timer (`set_status`).
    pub(crate) fn sync_settle(&mut self, cx: &mut Context<Self>) {
        // A command's end is due now, whatever the output's timer still has to wait.
        if self.settle.is_some() && !self.finished_mark {
            return;
        }
        let Some(wait) = self.settle_in(Instant::now()) else { return };
        // Due already (a command's end): no timer to wait for, the window is asked now.
        if wait.is_zero() {
            cx.emit(TerminalEvent::Quiet);
            return;
        }
        self.settle = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            this.update(cx, |this, cx| this.settle_fired(cx)).ok();
        }));
    }

    /// The timer ran out: ask the window to settle, or wait out what output moved since.
    fn settle_fired(&mut self, cx: &mut Context<Self>) {
        self.settle = None;
        self.sync_settle(cx);
    }

    /// Settle busy → idle/done/failed: once the output has been quiet for a while, or at once when
    /// the shell's marks said the command finished. Seen at once, there is nothing to settle into:
    /// the result is on screen (and a failure's cue stays on the row, see [`super::Failure`]). Run by the
    /// window when the terminal says [`TerminalEvent::Quiet`]. Also where a progress bar its program
    /// stopped renewing ends.
    pub fn tick(&mut self, is_active: bool, window_active: bool) -> bool {
        let expired = self.expire_progress_at(Instant::now());
        self.settle(is_active, window_active) | expired
    }

    fn settle(&mut self, is_active: bool, window_active: bool) -> bool {
        if self.reported() {
            return false;
        }
        let finished = std::mem::take(&mut self.finished_mark);
        let looking = is_active && window_active;
        // `failure` is this command's when the mark just came: it replaced the last one's.
        let failed = finished && self.failure.is_some();
        if self.status == Status::Busy && (finished || self.last_output.elapsed() >= QUIET) {
            self.set_status(match (looking, failed) {
                (true, _) => Status::Idle,
                (false, true) => Status::Failed,
                (false, false) => Status::Done,
            });
            return true;
        }
        // A command that went quiet counts as done; its exit code, when it comes, can say it failed.
        if failed && !looking && self.status == Status::Done {
            self.set_status(Status::Failed);
            return true;
        }
        false
    }
}
