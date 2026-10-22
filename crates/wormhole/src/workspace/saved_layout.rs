//! A window's layout as it is saved with the window: the side columns, the right panel's tab and
//! the sidebar's view.

use wormhole_core::view::layout::Panes;
use wormhole_core::view::sidebar::WindowSidebar;

/// What the right panel shows.
#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RightTab {
    #[default]
    Changes,
    History,
    Notes,
    Usage,
    Project,
}

/// A window's layout as saved with it.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct Layout {
    pub(super) sidebar_width: f32,
    pub(super) notes_width: f32,
    pub(super) sidebar_open: bool,
    pub(super) notes_open: bool,
    pub(super) right_tab: RightTab,
    /// The sidebar's grouping, filter and search, and its folded bands.
    pub(super) sidebar: WindowSidebar,
}

impl Default for Layout {
    fn default() -> Self {
        Self::new(Panes::default(), RightTab::default(), WindowSidebar::default())
    }
}

impl Layout {
    pub(super) fn new(panes: Panes, right_tab: RightTab, sidebar: WindowSidebar) -> Self {
        Self {
            sidebar_width: panes.sidebar_width,
            notes_width: panes.panel_width,
            sidebar_open: panes.sidebar_open,
            notes_open: panes.panel_wanted,
            right_tab,
            sidebar,
        }
    }

    pub(super) fn panes(&self) -> Panes {
        Panes {
            sidebar_open: self.sidebar_open,
            sidebar_width: self.sidebar_width,
            panel_wanted: self.notes_open,
            panel_width: self.notes_width,
        }
        .restored()
    }
}
