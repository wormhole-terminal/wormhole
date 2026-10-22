//! A profile as the app draws it: its avatar (the colour with one letter) and the edge a window
//! wears when it is in a profile other than the one Wormhole is in. The colours are the
//! theme's, from core's `FolderColor::color`.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::groups::PopoverTrigger;
use super::scale::{CONTROL_H, Edges, TextRole};
use crate::settings::UiColors;

/// The avatar of a profile in the chips and rows: a `size` circle of the profile's colour with
/// its letter in the window's own colour, or black or white where that would not read (the
/// window's white on a light theme's yellow or green).
#[track_caller]
pub fn profile_avatar(ui: UiColors, color: Hsla, letter: char, size: f32) -> Div {
    let fill = crate::theme_bridge::color(color);
    let ink = crate::theme_bridge::color(ui.background);
    let ink = if wormhole_core::theme::tokens::Color::contrast_ratio(ink, fill) >= LETTER_CONTRAST {
        ui.background
    } else {
        crate::theme_bridge::hsla(fill.text_on())
    };
    let avatar = div()
        .flex_none()
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(color)
        .text_color(ink)
        .font_weight(FontWeight::BOLD)
        .child(letter.to_uppercase().to_string());
    if size >= AVATAR_BODY_FROM { avatar.text_body() } else { avatar.text_caption() }
}

/// The chip in the top row of the sidebar and of the tab bar (RF-21): a borderless pop-up that names
/// the profile Wormhole is in. 24 tall, a 16 pt avatar, the name in 13, up-down chevrons, and a
/// fill only under the pointer. It is only drawn while there are two or more profiles. As a
/// `Popover`'s trigger it opens the profile menu.
#[track_caller]
pub fn profile_chip(
    id: impl Into<ElementId>,
    ui: UiColors,
    color: Hsla,
    letter: char,
    name: impl Into<SharedString>,
    spoken: impl Into<SharedString>,
) -> PopoverTrigger {
    let chip = div()
        .id(id)
        .role(Role::Button)
        .aria_label(spoken)
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .h(px(CHIP_H))
        .pl_1()
        .pr_2()
        .rounded(ui.radius)
        .cursor_pointer()
        .hover(move |d| d.bg(ui.band_hover))
        .child(profile_avatar(ui, color, letter, CHIP_AVATAR))
        .child(
            div()
                .max_w(px(CHIP_NAME_MAX))
                .truncate()
                .text_label()
                .font_weight(FontWeight::MEDIUM)
                .text_color(ui.text)
                .child(name.into()),
        )
        .child(Icon::new(IconName::ChevronsUpDown).xsmall().text_color(ui.muted));
    PopoverTrigger::new(chip)
}

/// The chip's height, its avatar, and the longest name it shows whole.
pub const CHIP_H: f32 = 24.0;
const CHIP_AVATAR: f32 = 16.0;
const CHIP_NAME_MAX: f32 = 120.0;

/// The least the letter is kept from the colour: the 4.5:1 WCAG asks of text.
const LETTER_CONTRAST: f32 = 4.5;

/// From this size the avatar's letter is body text, below it caption text.
const AVATAR_BODY_FROM: f32 = 24.0;

/// How tall the edge is across the top of a window in another profile than the shown one.
pub const PROFILE_EDGE_H: f32 = 3.0;

/// The edge across the top of a window that is not in the profile Wormhole is in, in that
/// profile's colour: with the name in the status bar, the only mark of the odd window.
#[track_caller]
pub fn profile_edge(color: Hsla) -> Div {
    div().flex_none().w_full().h(px(PROFILE_EDGE_H)).bg(color)
}

/// One option of a short list of exclusive choices (the "New profile" sheet's START FROM): a
/// round mark, filled when chosen, then the label. Add `.on_click(…)`.
#[track_caller]
pub fn choice_row(
    id: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    chosen: bool,
) -> Stateful<Div> {
    let mark = div()
        .flex_none()
        .size(px(CHOICE_MARK))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .outlined(ui, if chosen { ui.primary } else { ui.border_strong })
        .when(chosen, |d| d.child(div().size(px(CHOICE_DOT)).rounded_full().bg(ui.primary)));
    div()
        .id(id)
        .role(Role::RadioButton)
        .aria_selected(chosen)
        .flex()
        .items_center()
        .gap_2()
        .h(px(CONTROL_H))
        .cursor_pointer()
        .text_body()
        .text_color(ui.text)
        .child(mark)
        .child(label.into())
}

/// The round mark of a [`choice_row`] and the dot in it when chosen.
const CHOICE_MARK: f32 = 16.0;
const CHOICE_DOT: f32 = 8.0;
