//! The window being scaled, as the design fixed (the Motion feature; without it a window is as it always
//! was): resizing in whole cells of the shown terminal, taking off the strip a window off the cell grid
//! leaves when the drag ends. What the terminal does meanwhile (the PTY told calmly, the text easing, the
//! size chip) is `terminal_view::scaling`; the rules are core's (`view::motion::live_resize`).
//!
//! Part of the window: it may use the platform and the settings, and must not use the surfaces above it.

use gpui_kit::*;
use wormhole_core::view::motion::live_resize::settle;

use crate::platform;
use crate::settings::settings_in;
use crate::window::Workspace;

impl Workspace {
    /// Make the window resize in steps of the shown terminal's cell while one terminal fills the area, as
    /// Terminal does, so no part of a cell is left at the right or bottom. A split's panes do not share a
    /// grid, so there the window resizes freely. Cheap to call: the platform sets it only when it changes.
    pub(super) fn sync_resize_increments(&self, window: &Window, cx: &App) {
        let on = settings_in(&self.profile, cx).motion_feature_on() && self.active_split().is_none();
        // Through the probe the terminal shares, not the terminal: a window that reads a terminal while it draws
        // is drawn again whenever the terminal prints.
        let cell = on
            .then(|| self.active_session().pane.grid.cell())
            .filter(|(width, height)| *width > 0.0 && *height > 0.0);
        platform::window::set_resize_increments(window, cell);
    }

    /// A live resize ended in the terminal `ix`. If it is the one that fills the window, and the drag left
    /// the window off the cell grid, the window gives up the strip (less than a cell, across and down).
    ///
    /// AppKit applies the increments but does not say what they count from, so a window that was off the
    /// grid when the drag began may still be: this is what makes the strip vanish whatever it counts from.
    pub(super) fn live_resize_ended(&mut self, ix: usize, window: &Window, cx: &mut Context<Self>) {
        if ix != self.active
            || self.active_split().is_some()
            || !settings_in(&self.profile, cx).motion_feature_on()
        {
            return;
        }
        let pane = self.sessions[ix].terminal().read(cx);
        let (cell, spare) = (pane.pane_size().cell, pane.spare());
        if let Some(shrink) = settle(spare, cell) {
            platform::window::settle(window, shrink);
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
            terminal.spare(),
            self.active_split().is_some()
        )
    }

    /// `settle`: a live resize ends now.
    pub(crate) fn devctl_settle(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.live_resize_ended(self.active, window, cx);
    }
}
