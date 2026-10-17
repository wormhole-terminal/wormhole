//! Window layout rules that don't need the UI: how wide the
//! sidebar, the terminal and the right panel are, when the panel has room, what the panel and
//! sidebar toggles do when it hasn't, and how a drag on a panel's edge resolves.
//!
//! The window is three columns and a strip: the sidebar (open or hidden), the terminal (never
//! narrower than [`TERMINAL_MIN`]), the right panel, and the panel's tab strip down the window's
//! right edge, which belongs to the open panel: closed or without room, both are gone and the
//! terminal runs to the window's edge. The panel is the first to
//! give way: it shrinks from its chosen width to [`PANEL_MIN`], then hides until the window is
//! wide enough again.

mod remembered;
mod resize;
pub mod status_bar;
pub mod title_row;

pub use remembered::{RememberedPanes, STORE_KEY as REMEMBERED_PANES_KEY};
pub use resize::{Edge, Resize, Stop, resize};

/// The sidebar's width when it is open: from its minimum up to its maximum, 320 unless dragged.
pub const SIDEBAR_DEFAULT: f32 = 320.0;
pub const SIDEBAR_MIN: f32 = 240.0;
pub const SIDEBAR_MAX: f32 = 420.0;
/// The right panel's width: its default, and the range a drag moves it in.
pub const PANEL_DEFAULT: f32 = 300.0;
pub const PANEL_MIN: f32 = 240.0;
pub const PANEL_MAX: f32 = 720.0;
/// The strip of the panel's tabs on the window's right edge: there while the panel is on screen
/// (it comes and goes with the panel), and not part of the panel's width.
pub const PANEL_STRIP: f32 = 28.0;
/// Narrowest the terminal column may get (padding included, about 41 columns at the default
/// font). Below this the side panels give way instead, so the grid never collapses to a sliver
/// and the PTY is never resized to a few columns.
pub const TERMINAL_MIN: f32 = 320.0;
/// Dragging a panel's edge this far past its minimum width hides the panel.
pub const COLLAPSE_SLACK: f32 = 70.0;
/// The smallest window: a usable terminal with the open panel and its strip beside it (the sidebar
/// hidden), so the panel's toggle always has an effect. With the sidebar open as well, the panel
/// waits for room.
pub const WINDOW_MIN_WIDTH: f32 = TERMINAL_MIN + PANEL_STRIP + PANEL_MIN;
pub const WINDOW_MIN_HEIGHT: f32 = 400.0;

/// Whether the right panel has room next to the sidebar and the middle column.
///
/// `sidebar` is the sidebar's width while it is showing, `None` when hidden.
/// The panel is measured at its minimum width: it shrinks from its chosen width
/// before anything else gives way, so that is the smallest it can ever take.
/// A pure function of the window's current width, so the panel comes back by
/// itself when the window grows again and nothing has to be remembered.
pub fn right_panel_fits(window_width: f32, sidebar: Option<f32>, middle_min: f32, panel_min: f32) -> bool {
    window_width >= sidebar.unwrap_or(0.0) + middle_min + panel_min
}

/// One window's side panels as the user left them. Whether the right panel is on screen also
/// depends on the window's width ([`Panes::panel_visible`]), which is not stored: it follows the
/// window, so widening it brings the panel back by itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panes {
    pub sidebar_open: bool,
    /// The sidebar's chosen width, kept while it is hidden.
    pub sidebar_width: f32,
    /// The user wants the right panel (they opened it, or never closed it).
    pub panel_wanted: bool,
    /// The panel's chosen width; it shows narrower while the window can't spare it.
    pub panel_width: f32,
}

impl Default for Panes {
    fn default() -> Self {
        Self {
            sidebar_open: true,
            sidebar_width: SIDEBAR_DEFAULT,
            panel_wanted: true,
            panel_width: PANEL_DEFAULT,
        }
    }
}

/// How the right panel's toggle looks: the panel open, closed, or wanted but hidden for
/// lack of room (a small primary dot on the toggle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelToggle {
    Open,
    /// Closed by the user. `hides_sidebar`: opening it now would hide the sidebar for room.
    Closed {
        hides_sidebar: bool,
    },
    /// Wanted, waiting for room; opening it hides the sidebar.
    Waiting,
}

/// The columns' widths in a window, left to right; a hidden panel is 0 wide, and
/// so is the strip: it is [`PANEL_STRIP`] at the right edge only with the panel on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Columns {
    pub sidebar: f32,
    pub terminal: f32,
    pub panel: f32,
    pub strip: f32,
}

impl Panes {
    /// A saved layout, with widths brought back into their ranges (a hand-edited or older
    /// record can hold anything).
    pub fn restored(self) -> Self {
        Self {
            sidebar_width: self.sidebar_width.clamp(SIDEBAR_MIN, SIDEBAR_MAX),
            panel_width: self.panel_width.clamp(PANEL_MIN, PANEL_MAX),
            ..self
        }
    }

    /// The sidebar's width while it is showing. That is its chosen width unless the window
    /// can't spare it: the terminal keeps [`TERMINAL_MIN`], and the sidebar gives way down to
    /// its own minimum (the window is never narrower than both).
    pub fn sidebar(&self, window: f32) -> Option<f32> {
        self.sidebar_open.then(|| self.sidebar_width.min(window - TERMINAL_MIN).max(SIDEBAR_MIN))
    }

    /// Whether the window is wide enough for the right panel next to the sidebar and the
    /// terminal.
    pub fn panel_fits(&self, window: f32) -> bool {
        right_panel_fits(window, self.sidebar(window), TERMINAL_MIN + PANEL_STRIP, PANEL_MIN)
    }

    /// Whether the right panel is on screen: wanted, and with room. Hiding it for room leaves
    /// `panel_wanted` alone, so widening the window brings it back unless the user closed it.
    pub fn panel_visible(&self, window: f32) -> bool {
        self.panel_wanted && self.panel_fits(window)
    }

    /// Make the panel appear. With no room for it the sidebar steps aside (the user asked for
    /// the panel just now, so it wins); showing the sidebar again hides the panel again.
    pub fn reveal_panel(&mut self, window: f32) {
        self.panel_wanted = true;
        if !self.panel_fits(window) {
            self.sidebar_open = false;
        }
    }

    /// The panel toggle: it toggles what the user sees, so with the panel hidden for lack of
    /// room it opens it rather than "closing" something already invisible. Returns whether the
    /// panel is on screen afterwards.
    pub fn toggle_panel(&mut self, window: f32) -> bool {
        if self.panel_visible(window) {
            self.panel_wanted = false;
        } else {
            self.reveal_panel(window);
        }
        self.panel_visible(window)
    }

    /// What the panel's toggle shows in a window this wide.
    pub fn panel_toggle(&self, window: f32) -> PanelToggle {
        match (self.panel_wanted, self.panel_fits(window)) {
            (true, true) => PanelToggle::Open,
            (true, false) => PanelToggle::Waiting,
            (false, fits) => PanelToggle::Closed { hides_sidebar: !fits },
        }
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar_open = !self.sidebar_open;
    }

    /// The columns' widths in a window this wide. The panel yields first (from its chosen width
    /// down to [`PANEL_MIN`]) so the terminal keeps [`TERMINAL_MIN`]; the terminal takes the rest.
    pub fn columns(&self, window: f32) -> Columns {
        let sidebar = self.sidebar(window).unwrap_or(0.0);
        let (panel, strip) = if self.panel_visible(window) {
            (self.panel_width.min(window - sidebar - PANEL_STRIP - TERMINAL_MIN).max(PANEL_MIN), PANEL_STRIP)
        } else {
            (0.0, 0.0)
        };
        let terminal = (window - sidebar - panel - strip).max(TERMINAL_MIN);
        Columns { sidebar, terminal, panel, strip }
    }

    /// The widest `edge`'s panel may be dragged in a window this wide: its own maximum, or less
    /// when the window can't spare it (a 1100 window with the sidebar open stops the panel
    /// at 1100 − 320 − 28 − 320 = 432). Never below the panel's minimum.
    pub fn max_width(&self, edge: Edge, window: f32) -> f32 {
        let (min, max) = edge.limits();
        let room = match edge {
            Edge::Sidebar => {
                window - if self.panel_visible(window) { PANEL_STRIP } else { 0.0 } - TERMINAL_MIN
            }
            Edge::Panel => window - self.sidebar(window).unwrap_or(0.0) - PANEL_STRIP - TERMINAL_MIN,
        };
        max.min(room).max(min)
    }

    /// A drag on `edge` with the pointer `extent` from the window edge that panel hangs on.
    pub fn drag(&mut self, edge: Edge, extent: f32, window: f32) -> Resize {
        let to = resize(extent, edge.limits().0, self.max_width(edge, window));
        match edge {
            Edge::Sidebar => (self.sidebar_open, self.sidebar_width) = (to.open, to.width),
            Edge::Panel => (self.panel_wanted, self.panel_width) = (to.open, to.width),
        }
        to
    }

    /// A double-click on `edge`'s handle: back to the default width.
    pub fn reset(&mut self, edge: Edge) {
        match edge {
            Edge::Sidebar => self.sidebar_width = edge.default_width(),
            Edge::Panel => self.panel_width = edge.default_width(),
        }
    }
}

#[cfg(test)]
mod tests;
