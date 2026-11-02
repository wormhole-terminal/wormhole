//! The card a group of Settings rows sits in, and the group's heading: drawn by the `GroupCard` and
//! `GroupHead` components. The rows span the card with a padding of their own; each draws its line above
//! itself, and the card lifts them by that line and clips it, so the first needs no special case.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::ui::{GroupCardProps, GroupHeadProps, Look};

/// `rows` in one card.
#[derive(IntoElement)]
pub struct GroupCard {
    rows: Vec<AnyElement>,
}

pub fn group_card(_: UiColors, rows: impl IntoIterator<Item = AnyElement>) -> GroupCard {
    GroupCard { rows: rows.into_iter().collect() }
}

impl RenderOnce for GroupCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        GroupCardProps { rows: div().flex().flex_col().children(self.rows).into_any_element() }
            .render(&Look::of(cx))
    }
}

/// The head of a group of rows: its name, 13 pt semibold and muted.
#[derive(IntoElement)]
pub struct SettingGroupHead {
    title: SharedString,
}

pub fn setting_group_head(_: UiColors, title: impl Into<SharedString>) -> SettingGroupHead {
    SettingGroupHead { title: title.into() }
}

impl RenderOnce for SettingGroupHead {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        GroupHeadProps { title: self.title }.render(&Look::of(cx))
    }
}
