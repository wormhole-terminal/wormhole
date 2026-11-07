//! The panes and the chrome as records.

use crate::view::panes::TabPanes;

use super::super::model::WorkspaceModel;
use super::{ChromeSnapshot, PaneRect, PanesSnapshot};

impl WorkspaceModel {
    /// Where the selected terminal's panes are in an area `width` x `height` points: every pane in
    /// reading order, or the zoomed one alone over the whole area.
    pub fn panes_snapshot(&self, width: f32, height: f32) -> PanesSnapshot {
        let revision = self.revisions().layout;
        let Some(ix) = self.selected_index() else {
            return PanesSnapshot { revision, split: None, panes: Vec::new(), focused: None, zoomed: None };
        };
        let id = self.sessions[ix].id;
        let split = self.sessions[ix].split.and_then(|split| self.split_of(split));
        let alone;
        let layout = match split {
            Some(split) => &split.layout,
            None => {
                alone = TabPanes::single(id);
                &alone
            }
        };
        let focused = layout.focused();
        let panes = layout
            .shown(width, height)
            .into_iter()
            .map(|(pane, rect)| PaneRect {
                id: pane,
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
                focused: pane == focused,
            })
            .collect();
        PanesSnapshot {
            revision,
            split: split.map(|split| split.id),
            panes,
            focused: Some(focused),
            zoomed: layout.zoomed(),
        }
    }

    pub fn chrome_snapshot(&self) -> ChromeSnapshot {
        let chrome = &self.chrome;
        ChromeSnapshot {
            revision: self.revisions().chrome,
            sidebar_visible: chrome.sidebar_visible,
            panel_visible: chrome.panel_visible,
            panel_tab: chrome.panel_tab,
            search: chrome.search.clone(),
            overlay: chrome.overlay,
        }
    }
}
