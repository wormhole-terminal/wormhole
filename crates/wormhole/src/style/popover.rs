//! The rows and rules inside a popover that is a menu with more than words in it: the profile chip's
//! menu (RF-21) and the group switcher (RF-02). A GPUI Kit `PopupMenu` stays the choice for plain
//! menus; these exist where a row carries an avatar, a note on its right, or a card.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{ROW_H, TextRole};
use crate::settings::UiColors;

/// The parts of a [`popover_row`].
pub struct PopoverRow {
    /// An avatar, a tile or an icon before the label.
    pub lead: Option<AnyElement>,
    pub label: SharedString,
    /// Muted words at the right ("every window switches", "current").
    pub note: Option<SharedString>,
    /// A key at the right, as Apple's notation ("⌘,").
    pub key: Option<SharedString>,
    /// The row is what is on now: its label is full strength and its note says so.
    pub current: bool,
    pub spoken: SharedString,
}

impl PopoverRow {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self { lead: None, spoken: label.clone(), label, note: None, key: None, current: false }
    }
}

/// One row of a popover: 28 tall, a hover fill, the lead and label at the left and the note or key
/// at the right. Add `.on_click(…)`.
#[track_caller]
pub fn popover_row(id: impl Into<ElementId>, ui: UiColors, row: PopoverRow) -> Stateful<Div> {
    let PopoverRow { lead, label, note, key, current, spoken } = row;
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(spoken)
        .flex()
        .items_center()
        .gap_2()
        .h(px(ROW_H))
        .px_2()
        .rounded(ui.radius)
        .cursor_pointer()
        .hover(move |d| d.bg(ui.hover))
        .children(lead)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_label()
                .font_weight(if current { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                .text_color(ui.text)
                .child(label),
        )
        .children(note.map(|note| div().flex_none().text_caption().text_color(ui.muted).child(note)))
        .children(key.map(|key| div().flex_none().text_caption().text_color(ui.faint).child(key)))
}

/// The small heading above a popover's rows ("Switch Wormhole to").
#[track_caller]
pub fn popover_heading(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div().px_2().h(px(ROW_H)).flex().items_center().text_caption().text_color(ui.muted).child(text.into())
}

/// The line between two groups of a popover's rows.
#[track_caller]
pub fn popover_rule(ui: UiColors) -> Div {
    div().h(px(1.0)).my_1().mx_1().bg(ui.border)
}
