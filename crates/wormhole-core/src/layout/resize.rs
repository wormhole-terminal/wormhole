//! Dragging a panel's inner edge: the width follows the pointer between the panel's
//! minimum and maximum, sticks at the minimum for [`COLLAPSE_SLACK`] more, then hides the panel;
//! dragging back past that point opens it again in the same gesture. A readout beside the
//! pointer says the width and why it stopped.

use super::{COLLAPSE_SLACK, PANEL_DEFAULT, PANEL_MAX, PANEL_MIN, SIDEBAR_DEFAULT, SIDEBAR_MAX, SIDEBAR_MIN};

/// The edge being dragged: the sidebar's right edge or the right panel's left edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Sidebar,
    Panel,
}

impl Edge {
    /// The widths a drag moves this panel between, before the window has its say.
    pub fn limits(self) -> (f32, f32) {
        match self {
            Edge::Sidebar => (SIDEBAR_MIN, SIDEBAR_MAX),
            Edge::Panel => (PANEL_MIN, PANEL_MAX),
        }
    }

    /// What a double-click on the handle puts the panel back to.
    pub fn default_width(self) -> f32 {
        match self {
            Edge::Sidebar => SIDEBAR_DEFAULT,
            Edge::Panel => PANEL_DEFAULT,
        }
    }
}

/// Why a drag stopped following the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Held at the minimum, until the pointer is [`COLLAPSE_SLACK`] past it.
    Min,
    /// At the most the panel may take: its own maximum, or what the window can spare.
    Max,
}

/// Where a drag leaves a panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resize {
    /// Still open (not dragged past its minimum by [`COLLAPSE_SLACK`]).
    pub open: bool,
    /// Its width, within `min..=max` (the minimum while it is hidden, which is what it reopens at).
    pub width: f32,
    /// Set while the width no longer follows the pointer and the panel is open.
    pub stop: Option<Stop>,
}

impl Resize {
    /// What the readout beside the pointer says: the width, with why it stopped
    /// ("430 · max"), or that letting go here hides the panel.
    pub fn readout(&self, edge: Edge) -> String {
        if !self.open {
            return match edge {
                Edge::Sidebar => "Hide".into(),
                Edge::Panel => "Close".into(),
            };
        }
        let width = self.width.round();
        match self.stop {
            None => format!("{width}"),
            Some(Stop::Min) => format!("{width} · min"),
            Some(Stop::Max) => format!("{width} · max"),
        }
    }
}

/// The panel's state with the pointer `extent` from the window edge the panel hangs on (the
/// pointer's x for the sidebar, the window's width minus it for the right panel). `max` below
/// `min` (a window that can't spare more) counts as `min`.
pub fn resize(extent: f32, min: f32, max: f32) -> Resize {
    let max = max.max(min);
    let open = extent >= min - COLLAPSE_SLACK;
    let stop = if !open {
        None
    } else if extent < min {
        Some(Stop::Min)
    } else if extent > max {
        Some(Stop::Max)
    } else {
        None
    };
    Resize { open, width: extent.clamp(min, max), stop }
}
