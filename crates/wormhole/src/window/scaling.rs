//! The window being scaled, as the design fixed: resizing in steps of the shown terminal's cell and taking
//! off the strip a window off the cell grid leaves when the drag ends (the Live resize feature), and which
//! windows open and close with the system's own animation (the Motion feature; without the features a window
//! is as it always was). What the terminal does meanwhile (the PTY told calmly, the text easing, the size
//! chip) is `terminal_view::scaling`; the rules are core's (`view::motion::{live_resize, window_animation}`).
//!
//! Part of the window: it may use the platform and the settings, and must not use the surfaces above it.

use std::time::Duration;

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::motion::live_resize::{Dragged, Event, settle, steps};
use wormhole_core::view::motion::window_animation::{Facts, animates, behaviour};

use crate::app_state::{AppState, app};
use crate::platform;
use crate::settings::settings_in;
use crate::window::Workspace;

/// How long after a drag ends the strip is measured and taken off: the frame the last size was drawn in is
/// over by then.
const AFTER_THE_LAST_FRAME: Duration = Duration::from_millis(50);

fn facts(profile: &ProfileId, grows: bool, cx: &App) -> Facts {
    Facts {
        motion_feature: settings_in(profile, cx).motion_feature_on(),
        reduce_motion: cx.reduce_motion(),
        background: crate::background::on(),
        grows,
        platform: platform::window::has_system_animation(),
    }
}

/// Whether a window of `profile` opens and closes with the system's animation: `grows` when it opens as a
/// torn-off terminal's, which has a growth of its own.
pub(super) fn terminal_window_animates(profile: &ProfileId, grows: bool, cx: &App) -> bool {
    animates(facts(profile, grows, cx))
}

impl Workspace {
    /// Make the window resize in steps of the shown terminal's cell while one terminal fills the area, as
    /// Terminal does, so no part of a cell is left at the right or bottom. A split's panes do not share a
    /// grid, so there the window resizes freely. Called when the steps can change (the terminal's cell, the
    /// selection, a tick for the rest), not for every frame; the platform sets them only when they differ.
    pub(super) fn sync_resize_increments(&self, window: &Window, cx: &App) {
        let on = settings_in(&self.profile, cx).live_resize_on() && self.active_split().is_none();
        // Through the probe the terminal shares, not the terminal: a window that reads a terminal while it draws
        // is drawn again whenever the terminal prints.
        let cell = on
            .then(|| self.active_session().pane.grid.cell())
            .filter(|(width, height)| *width > 0.0 && *height > 0.0)
            .map(steps);
        platform::live_resize::set_increments(window, cell);
    }

    /// A live resize of this window ended (the system said so, outside any update). The terminals are drawn
    /// again, so the one that was held back is told the size now; then, if one terminal fills the window and
    /// the drag left it off the cell grid, the window gives up the strip (less than a cell, across and down).
    ///
    /// AppKit applies the steps but does not say what they count from, and keeps them in whole points, so a
    /// window may end off the grid: this is what makes the strip vanish. It is done a moment later, from a
    /// task, with the window free of any update: a frame set inside one is never heard by GPUI (its resize
    /// callback finds the window borrowed and gives up), and the drawn size would stay the old one.
    pub(crate) fn live_resize_ended(
        &mut self,
        dragged: Dragged,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !settings_in(&self.profile, cx).live_resize_on() {
            return;
        }
        for session in &self.sessions {
            session.terminal().update(cx, |_, cx| cx.notify());
        }
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(AFTER_THE_LAST_FRAME).await;
            let job = cx
                .update(|window, cx| {
                    let strip = this.read_with(cx, |workspace, _| workspace.settle_strip()).ok().flatten()?;
                    Some((platform::window::Native::of(window)?, strip))
                })
                .ok()
                .flatten();
            // Outside the update that found it.
            if let Some((native, strip)) = job {
                native.settle(strip, dragged);
            }
        })
        .detach();
    }

    /// The strip (across, down) the window is to give up, if one terminal fills it and it has one.
    fn settle_strip(&self) -> Option<(f32, f32)> {
        if self.active_split().is_some() {
            return None;
        }
        let grid = &self.active_session().pane.grid;
        settle(grid.spare(), grid.cell())
    }

    /// Give the window the open and close animation the settings and Reduce Motion say now: the system's,
    /// none, or its default. Called every tick; the platform changes nothing that is already so, and this is
    /// also what gives a window back the animation a profile switch took off it for a moment.
    pub(super) fn sync_window_animation(&self, window: &Window, cx: &App) {
        let grows = crate::window::grow::active(self.window_id, cx);
        platform::window::set_animation(window, behaviour(facts(&self.profile, grows, cx)));
    }
}

/// What the system said about a window's live resize (the app hears it and passes it on from a task, so the
/// call comes outside every update).
pub(crate) fn live_resize_changed(event: Event, cx: &mut App) {
    let Event::Ended { window: number, dragged } = event else { return };
    let windows = app(cx).windows.clone();
    for open in windows {
        let is_it = open.handle.update(cx, |_, window, _| platform::window::number(window) == Some(number));
        if is_it.unwrap_or(false) {
            AppState::with_workspace(&open, cx, |workspace, window, cx| {
                workspace.live_resize_ended(dragged, window, cx);
            });
            return;
        }
    }
}

/// What devctl asks of the scaling.
#[cfg(feature = "devctl")]
impl Workspace {
    /// `scaling`: the window's resize steps and animation, the shown terminal's cell, grid and spare room.
    pub(crate) fn devctl_scaling(&self, window: &Window, cx: &App) -> String {
        let terminal = self.active_terminal().read(cx);
        let pane = terminal.pane_size();
        format!(
            "{} cell={:?} grid={}x{} spare={:?} split={}\n",
            platform::window::scaling_state(window).unwrap_or_else(|| "no native window".into()),
            pane.cell,
            pane.cols,
            pane.rows,
            self.active_session().pane.grid.spare(),
            self.active_split().is_some()
        )
    }

    /// `settle`: a live resize ends now, with the bottom right corner dragged.
    pub(crate) fn devctl_settle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.live_resize_ended(Dragged::default(), window, cx);
    }
}
