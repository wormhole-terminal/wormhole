//! A terminal while its window is scaled (the Motion feature; without it a terminal is told every size at
//! once and shows no chip): what the design fixed for a window being dragged to another size and for the
//! text zoomed with ⌘+ / ⌘−. The rules are core's (`view::motion::{live_resize, size_readout, text_zoom}`);
//! this is the terminal's side of them.
//!
//! - `live` decides, each frame, which size the PTY is told and which grid is drawn: the pane's size at once,
//!   or the PTY's own while a text zoom moves or a live resize holds the size back (the drawn grid is
//!   clipped or leaves blank edges, anchored at the top left as always).
//! - `zoom` eases the text size to its new one, and the PTY is told once, when it has arrived.
//! - `readout` is the chip with the grid's size.

mod live;
mod readout;
mod zoom;

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::view::motion::live_resize::ResizeGate;
use wormhole_core::view::motion::text_zoom::Ease;

use super::TerminalView;
use crate::settings::settings_in;

/// What a terminal keeps between frames for the scaling of its window.
#[derive(Default)]
pub(super) struct Scaling {
    /// Whether the PTY is told the size now, in a live resize.
    gate: ResizeGate,
    /// The draw that comes when a size held back is due, and when a live resize may have ended.
    wake: Option<Task<()>>,
    /// The grid the pane made at the last frame, to tell a change from the first sight of a size.
    wanted: Option<(usize, usize)>,
    /// The text size on its way, in points over Settings (`TerminalView::font_zoom` is where it goes).
    zoom: Option<Ease>,
    /// The text was zoomed at this moment: the chip says the size the grid has once it is settled.
    readout_due: Option<Instant>,
    /// The chip with the size, while it shows.
    readout: Option<readout::Readout>,
}

impl TerminalView {
    /// Whether this terminal's window scales as the design fixed: the Motion feature is on in this build.
    pub(in crate::terminal_view) fn scaling_on(&self, cx: &App) -> bool {
        settings_in(&self.profile, cx).motion_feature_on()
    }
}

/// What the tests read.
#[cfg(test)]
impl TerminalView {
    /// The columns and rows the PTY has been told.
    pub(crate) fn told_cells(&self) -> Option<(usize, usize)> {
        self.pty.as_ref().map(|pty| (usize::from(pty.size().cols), usize::from(pty.size().rows)))
    }

    /// What the size chip says, while it shows.
    pub(crate) fn size_readout_text(&self) -> Option<String> {
        self.scaling.readout.as_ref().map(|readout| readout.text())
    }
}
