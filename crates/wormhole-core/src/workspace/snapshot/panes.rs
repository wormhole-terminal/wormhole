//! The panes as records.

use crate::view::panes::TabPanes;

use super::super::model::WorkspaceModel;
use super::{DividerInfo, PaneRect, PanesSnapshot};

impl WorkspaceModel {
    /// Where the selected terminal's panes are in an area `width` x `height` points: every pane in
    /// reading order, or the zoomed one alone over the whole area.
    pub fn panes_snapshot(&self, width: f32, height: f32) -> PanesSnapshot {
        let revision = self.revisions().layout;
        let Some(ix) = self.selected_index() else {
            return PanesSnapshot {
                revision,
                split: None,
                panes: Vec::new(),
                focused: None,
                zoomed: None,
                dividers: Vec::new(),
                unzoomed: Vec::new(),
            };
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
        let place = |(pane, rect): (i64, crate::view::panes::Rect)| PaneRect {
            id: pane,
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            focused: pane == focused,
        };
        let panes = layout.shown(width, height).into_iter().map(place).collect();
        let unzoomed = layout.tree().rects(width, height).into_iter().map(place).collect();
        let dividers = layout
            .dividers(width, height)
            .into_iter()
            .map(|divider| DividerInfo {
                path: divider.id.path.iter().map(|&at| at as u32).collect(),
                index: divider.id.index as u32,
                axis: divider.axis,
                position: divider.position(),
                x: divider.rect.x,
                y: divider.rect.y,
                w: divider.rect.w,
                h: divider.rect.h,
            })
            .collect();
        PanesSnapshot {
            revision,
            split: split.map(|split| split.id),
            panes,
            focused: Some(focused),
            zoomed: layout.zoomed(),
            dividers,
            unzoomed,
        }
    }
}
