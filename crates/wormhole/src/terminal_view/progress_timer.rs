//! The end of a stale OSC 9;4 progress bar: a program that stopped renewing its report (it died, or was
//! killed, mid-bar) must not leave the bar up for good, so the report lasts
//! `wormhole_term::program_status::PROGRESS_TIMEOUT` past its last renewal.
//!
//! A deadline, so a one-shot timer armed when a report appears rather than a look on every window tick.
//! A renewal changes nothing the terminal hears of (a repeated report raises no event), so it only moves
//! the deadline in the store, which the timer reads when it fires: it then waits out what is left, as
//! `settle` does for output. A removal, a prompt or the program's exit cancels the timer the next time
//! the terminal hears of a change. When the timer finds the report expired it drops it and announces the
//! summary, which tells the window outside a draw.

use std::time::Instant;

use gpui_kit::*;

use super::TerminalView;

impl TerminalView {
    /// Arm the timer for the progress that stands, if none runs; cancel it when none stands. Called after
    /// anything that may have set or ended a report (`PtyEvent::ProgramStatus`).
    pub(super) fn sync_progress_timer(&mut self, cx: &mut Context<Self>) {
        let wait = self.pty.as_ref().and_then(|pty| pty.program_status().progress_remaining(Instant::now()));
        let Some(wait) = wait else {
            self.progress_timer = None;
            return;
        };
        if self.progress_timer.is_some() {
            return;
        }
        self.progress_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            this.update(cx, |this, cx| this.fire_progress_timer(Instant::now(), cx)).ok();
        }));
    }

    /// The timer ran out at `now`: drop the report if nobody renewed it and tell the window, or wait out
    /// what a renewal left.
    pub(crate) fn fire_progress_timer(&mut self, now: Instant, cx: &mut Context<Self>) {
        self.progress_timer = None;
        if self.expire_progress_at(now) {
            self.announce_summary(cx);
        }
        self.sync_progress_timer(cx);
    }
}
