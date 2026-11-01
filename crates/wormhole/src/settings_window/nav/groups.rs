//! The groups of the open page as sub-rows of the nav, and the scroll that a click on one makes.
//!
//! The list is the page's own: the titled groups of core's settings index that the page draws a row of
//! (`SettingsWindow::group_drawn`), so no list of groups is written twice and a sub-row never leads to
//! nothing. A group without a title gets no sub-row.

use gpui_kit::*;
use wormhole_core::config::settings_index::{Group, SectionId as Section};

use super::super::SettingsWindow;
use super::super::open::REVEAL_INSET;
use crate::settings::Settings;

/// The group last picked on the open page, and the scroll to it that is still to be made.
#[derive(Default)]
pub(in crate::settings_window) struct GroupPick {
    /// An index into the section's groups; none until a sub-row is picked.
    index: Option<usize>,
    /// Hangs on the picked group's element, which tells it where it was laid out.
    anchor: Option<ScrollAnchor>,
    /// The anchor has not scrolled yet: it knows its group's place only after the draw that laid it out.
    pending: bool,
}

impl GroupPick {
    /// The anchor of group `at` while it is the picked one.
    pub(in crate::settings_window) fn anchor_of(&self, at: usize) -> Option<ScrollAnchor> {
        if self.index == Some(at) { self.anchor.clone() } else { None }
    }
}

impl SettingsWindow {
    /// The groups of `section`'s page that the nav lists under it, with their place in the page.
    pub(in crate::settings_window) fn nav_groups<'a>(
        &'a self,
        section: Section,
        settings: &'a Settings,
    ) -> impl Iterator<Item = (usize, &'static Group)> + 'a {
        // The profile page is the card above the list, which has no sub-rows of its own.
        let groups = if section == Section::Profile { &[] } else { section.groups() };
        groups
            .iter()
            .enumerate()
            .filter(move |(_, group)| !group.title().is_empty() && self.group_drawn(group, settings))
    }

    /// A sub-row was picked: the page scrolls to its group once drawn.
    pub(super) fn pick_group(&mut self, at: usize, cx: &mut Context<Self>) {
        self.group = GroupPick {
            index: Some(at),
            anchor: Some(ScrollAnchor::for_handle(self.content_scroll.clone())),
            pending: true,
        };
        cx.notify();
    }

    /// Which sub-row reads as current: the picked group, else the page's first.
    pub(super) fn current_group(&self, first: Option<usize>) -> Option<usize> {
        self.group.index.or(first)
    }

    /// Scroll to the picked group. Called while the page is built: the anchor of the draw that follows
    /// knows where the group is, and the scroll takes effect a frame later, as a revealed row's does.
    pub(in crate::settings_window) fn scroll_to_group(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !std::mem::take(&mut self.group.pending) {
            return;
        }
        let Some(anchor) = self.group.anchor.clone() else { return };
        cx.on_next_frame(window, move |_, window, cx| {
            anchor.scroll_to(window, cx);
            // After the anchor's own scroll: the room for the title row, and a redraw, which
            // `set_offset` doesn't ask for.
            cx.on_next_frame(window, |this, _, cx| {
                let offset = this.content_scroll.offset();
                this.content_scroll.set_offset(point(offset.x, offset.y + px(REVEAL_INSET)));
                cx.notify();
            });
        });
    }
}
