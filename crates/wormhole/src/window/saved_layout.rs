//! A window's layout as it is saved with the window: the side columns, the right panel's tab and
//! the sidebar's view.
//!
//! Plain data that is serialized: it must not use GPUI, the shared state or any view.

use wormhole_core::config::TabId;
use wormhole_core::view::layout::Panes;
use wormhole_core::view::sidebar::WindowSidebar;

/// A window's layout as saved with it.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct Layout {
    pub(super) sidebar_width: f32,
    pub(super) notes_width: f32,
    pub(super) sidebar_open: bool,
    pub(super) notes_open: bool,
    pub(super) right_tab: TabId,
    /// The sidebar's grouping, filter and search, and its folded bands.
    pub(super) sidebar: WindowSidebar,
}

impl Default for Layout {
    fn default() -> Self {
        Self::new(Panes::default(), TabId::default(), WindowSidebar::default())
    }
}

impl Layout {
    pub(super) fn new(panes: Panes, right_tab: TabId, sidebar: WindowSidebar) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A window's layout as 1.2.0 saved it (before the tabs were `TabId`s).
    fn saved_by_1_2_0(tab: &str) -> String {
        format!(
            r#"{{"sidebar_width":264.0,"notes_width":312.0,"sidebar_open":true,"notes_open":false,"right_tab":"{tab}"}}"#
        )
    }

    #[test]
    fn a_layout_saved_by_1_2_0_opens_on_its_tab_and_writes_it_back_the_same() {
        for tab in TabId::ALL {
            let layout: Layout = serde_json::from_str(&saved_by_1_2_0(tab.key())).unwrap();
            assert_eq!(layout.right_tab, tab);
            assert_eq!(layout.sidebar_width, 264.0);
            assert!(!layout.notes_open);
            let written = serde_json::to_value(&layout).unwrap();
            assert_eq!(written["right_tab"], tab.key(), "1.2.0 reads what this writes");
        }
    }

    #[test]
    fn a_tab_of_a_newer_build_opens_changes_and_keeps_the_rest_of_the_layout() {
        let layout: Layout = serde_json::from_str(&saved_by_1_2_0("terminal-log")).unwrap();
        assert_eq!(layout.right_tab, TabId::Changes);
        assert_eq!((layout.sidebar_width, layout.notes_width), (264.0, 312.0));
        assert!(layout.sidebar_open && !layout.notes_open);
    }
}
