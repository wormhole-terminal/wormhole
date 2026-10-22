//! The side columns as the person last left them, app-wide. A window opened with no saved layout
//! of its own (the first launch, or the Dock icon with every window closed) starts this way
//! instead of with [`Panes::default`]; a saved window keeps its own, and ⌘N copies the front
//! window's.
//!
//! What counts is what the person *wanted* (`panel_wanted`), never whether the panel was drawn:
//! a panel hidden for lack of room in a narrow window is still wanted and must not be
//! remembered as closed.

use serde::{Deserialize, Serialize};

use super::{PANEL_DEFAULT, Panes, SIDEBAR_DEFAULT};

/// The store's `meta` key the JSON is kept under.
pub const STORE_KEY: &str = "last_panes";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RememberedPanes {
    pub sidebar_open: bool,
    pub sidebar_width: f32,
    pub panel_open: bool,
    pub panel_width: f32,
}

impl Default for RememberedPanes {
    /// Nothing remembered yet (a first launch): the terminal list is open and the right panel,
    /// which has little to show in a fresh terminal, stays out of the way. [`Panes::default`] is
    /// not this: it is the layout a window has when no one asked, and many tests build on it.
    fn default() -> Self {
        Self {
            sidebar_open: true,
            sidebar_width: SIDEBAR_DEFAULT,
            panel_open: false,
            panel_width: PANEL_DEFAULT,
        }
    }
}

impl RememberedPanes {
    /// What was stored (`None`: nothing yet). Damaged JSON counts as nothing, and widths outside
    /// their range are brought back into it.
    pub fn parse(json: Option<&str>) -> Self {
        let parsed = json.and_then(|json| serde_json::from_str::<Self>(json).ok()).unwrap_or_default();
        Self::of(&parsed.panes())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// The columns of a window the person just changed.
    pub fn of(panes: &Panes) -> Self {
        Self {
            sidebar_open: panes.sidebar_open,
            sidebar_width: panes.sidebar_width,
            panel_open: panes.panel_wanted,
            panel_width: panes.panel_width,
        }
    }

    /// The columns a new window starts with.
    pub fn panes(&self) -> Panes {
        Panes {
            sidebar_open: self.sidebar_open,
            sidebar_width: self.sidebar_width,
            panel_wanted: self.panel_open,
            panel_width: self.panel_width,
        }
        .restored()
    }
}

#[cfg(test)]
mod tests {
    use super::RememberedPanes;
    use crate::view::layout::{PANEL_MAX, PANEL_MIN, Panes, SIDEBAR_MAX};

    #[test]
    fn nothing_remembered_is_the_sidebar_open_and_the_panel_closed() {
        let first = RememberedPanes::parse(None).panes();
        assert!(first.sidebar_open && !first.panel_wanted);
        assert_eq!(
            (first.sidebar_width, first.panel_width),
            (Panes::default().sidebar_width, Panes::default().panel_width)
        );
        assert_eq!(RememberedPanes::parse(None), RememberedPanes::default());
    }

    #[test]
    fn what_the_person_left_comes_back() {
        let left =
            Panes { sidebar_open: false, sidebar_width: 300.0, panel_wanted: true, panel_width: 500.0 };
        let json = RememberedPanes::of(&left).to_json();
        assert_eq!(RememberedPanes::parse(Some(&json)).panes(), left);
    }

    #[test]
    fn a_panel_hidden_for_room_is_still_wanted() {
        // `panel_wanted` is what is kept, however narrow the window: 600 has no room for it.
        let narrow = Panes { panel_wanted: true, ..Panes::default() };
        assert!(!narrow.panel_visible(600.0));
        assert!(RememberedPanes::of(&narrow).panel_open);
    }

    #[test]
    fn damaged_or_wild_values_settle() {
        assert_eq!(RememberedPanes::parse(Some("not json")), RememberedPanes::default());
        assert_eq!(RememberedPanes::parse(Some("")), RememberedPanes::default());
        // A key a later build adds is ignored, a missing one takes its default.
        let partial = RememberedPanes::parse(Some(r#"{"panel_open":true,"future":1}"#));
        assert!(partial.panel_open && partial.sidebar_open);
        let wild = RememberedPanes::parse(Some(
            r#"{"sidebar_open":true,"sidebar_width":9000,"panel_open":true,"panel_width":1}"#,
        ));
        assert_eq!((wild.sidebar_width, wild.panel_width), (SIDEBAR_MAX, PANEL_MIN));
        let wide = RememberedPanes::parse(Some(r#"{"panel_width":99999}"#));
        assert_eq!(wide.panel_width, PANEL_MAX);
    }
}
