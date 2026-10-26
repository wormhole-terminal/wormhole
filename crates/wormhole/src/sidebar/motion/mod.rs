//! How the terminal list moves. Each motion is a file of its own beside the list's: `list` holds what is
//! under way and wraps the lines in it, `plan` decides it when the list changes, `insert` and `fold` say
//! which lines came and went as a terminal opening or closing and as a group's fold, `leaving` keeps the
//! rows that went drawn until their motion is over, `room` opens the gap a row dragged over the list has
//! the lines make for it, `join` previews the split a row dropped on the middle of another makes, and `pose` draws a line in the pose core's
//! `view::motion::pose` gives it, which is also what the Settings previews draw. The sidebar reaches
//! them through this module.

mod fold;
mod insert;
mod join;
mod leaving;
mod list;
mod plan;
mod pose;
mod room;

pub(crate) use join::Snap;
pub(super) use join::draw_bracket;
pub(super) use list::{ItemKey, ListMotion};

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;

use crate::settings::settings_in;
use crate::sidebar::Sidebar;
use crate::sidebar::terminal_drag::DragOrigin;

impl Sidebar {
    /// Whether the person's Motion feature is on: what a drag does is as it always was without it.
    pub(in crate::sidebar) fn motion_on(&self, cx: &App) -> bool {
        settings_in(&self.input.profile, cx).feature_enabled(FeatureFlag::Motion)
    }

    /// The terminal being dragged by its row, while the list moves for it.
    pub(in crate::sidebar) fn row_in_flight(&self, cx: &App) -> Option<i64> {
        let (id, origin) = self.input.terminal_drag?;
        (cx.has_active_drag() && origin == DragOrigin::Row && self.motion_on(cx)).then_some(id)
    }
}
