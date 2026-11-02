//! The words that change with time ("working · 3m" in a sidebar row, the breadcrumb and the status bar,
//! a command's clock, "resets 34m" in a panel tab): one timer per window, armed for the next moment any
//! shown word would read differently, and for nothing at all while none is shown.
//!
//! Part of the window. The timer is re-planned when the facts it depends on move: a terminal's summary
//! (`sync_summary_at`), the window's activation, and the tick as the backstop for what has no single
//! event (the sidebar or a panel tab being shown or hidden, the density, the status bar's switch).
//! Only the redrawing is the timer's; reading the other windows' terminals stays on the tick
//! (`refresh_status_bar`), since nothing tells this window what another one's terminals do.

use std::time::{Duration, Instant};

use gpui_kit::*;

use wormhole_core::view::sidebar::wording::until_next;

use crate::sidebar::next_row_change;
use crate::window::Workspace;
use crate::window::title::breadcrumb::crumb_next_change;

/// The timer fires a little after the boundary, so the clock it was planned from has passed it.
const SLACK: Duration = Duration::from_millis(30);

/// The window's timer for time-bound words.
#[derive(Default)]
pub(in crate::window) struct Clock {
    /// When the armed timer fires, and the timer itself (dropping it cancels it).
    armed: Option<(Instant, Task<()>)>,
    /// Whether the window is the one in front, as last told: a window nobody looks at draws no clocks.
    window_active: bool,
}

impl Workspace {
    /// How long until a word shown now reads differently, or `None` while none moves.
    fn next_word_change(&self, window_active: bool, cx: &App) -> Option<Duration> {
        let mut next: Option<Duration> = None;
        let mut soon = |wait: Option<Duration>| {
            if let Some(wait) = wait {
                next = Some(next.map_or(wait, |n| n.min(wait)));
            }
        };
        if window_active && self.sidebar_times_shown() {
            soon(next_row_change(&self.row_infos(cx), &self.profile, cx));
        }
        if self.crumb_words_shown(cx) {
            soon(crumb_next_change(&self.active_session().pane.summary));
        }
        if crate::settings::settings_in(&self.profile, cx).status_bar {
            for counted in self.status_counted(cx) {
                if counted.is_quiet() {
                    continue;
                }
                soon(Some(until_next(counted.since().elapsed(), Duration::from_secs(60))));
            }
        }
        soon(self.tab_next_word_change(cx));
        next
    }

    /// Plan the timer again: keep one that fires no later than the next change, else arm it for that.
    pub(crate) fn sync_clock(&mut self, window_active: bool, cx: &mut Context<Self>) {
        self.regions.drawn.clock.window_active = window_active;
        // Nothing moves now: a timer still armed is left to fire once, find nothing and end (arming
        // and dropping one on every status flip of a busy terminal would cost more than that wake).
        let Some(wait) = self.next_word_change(window_active, cx) else { return };
        let wait = wait + SLACK;
        let due = Instant::now() + wait;
        // A timer a moment later than the plan is as good: planning again and again from clocks that
        // have moved a few microseconds would arm one each time.
        if self.regions.drawn.clock.armed.as_ref().is_some_and(|(armed, _)| *armed <= due + SLACK) {
            return;
        }
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            this.update(cx, |this, cx| this.clock_fired(cx)).ok();
        });
        self.regions.drawn.clock.armed = Some((due, task));
    }

    /// The timer ran out: draw what reads differently now, and plan the next.
    fn clock_fired(&mut self, cx: &mut Context<Self>) {
        self.regions.drawn.clock.armed = None;
        let window_active = self.regions.drawn.clock.window_active;
        self.redraw_times(window_active, cx);
        self.sync_clock(window_active, cx);
    }

    /// Redraw each place whose words would read differently than when it was drawn.
    fn redraw_times(&mut self, window_active: bool, cx: &mut Context<Self>) {
        if window_active {
            self.sidebar_times(cx);
        }
        self.crumb_tick(cx);
        self.redraw_status_bar_if_stale(cx);
        // The tab on show, if its words move: they are read from the clock when it draws.
        if self.tab_next_word_change(cx).is_some() {
            cx.notify();
        }
    }

    /// Plan the timer again with what the window last knew of its activation: for a change in a
    /// terminal's summary, which may start or end a time-bound word.
    pub(super) fn replan_clock(&mut self, cx: &mut Context<Self>) {
        let window_active = self.regions.drawn.clock.window_active;
        self.sync_clock(window_active, cx);
    }
}
