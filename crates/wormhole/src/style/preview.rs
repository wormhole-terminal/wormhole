//! The frame round a live preview in Settings, and the stage inside it: the card a page puts its
//! miniature of the app in, so every preview sits in the same box. The miniature is made of the
//! app's own components, never a drawing of them.

use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::{self, Edges, TextRole};

/// What a preview card says around its picture, all of it words from the catalogs.
#[derive(Clone, Copy)]
pub struct PreviewCard<'a> {
    /// The card's own name, so it is one element for the keyboard and the tests.
    pub id: &'static str,
    /// The heading's first word, "Preview".
    pub label: &'a str,
    /// What is previewed: "Window", "Fonts", ... shown after the label.
    pub what: &'a str,
    /// The picture's name for VoiceOver: a picture of the settings, never a control.
    pub spoken: &'a str,
    /// The line under the picture, saying what it shows.
    pub caption: &'a str,
}

/// A page's preview as a card: the label and what is previewed above `frame` (a [`preview_frame`] holding
/// the miniature), the caption under it. One group for VoiceOver, named by `spoken`; the picture
/// holds nothing to operate, so the card takes no focus of its own.
#[track_caller]
pub fn preview_card(ui: UiColors, card: PreviewCard<'_>, frame: Div) -> Stateful<Div> {
    div()
        .id(card.id)
        .role(Role::Group)
        .aria_label(SharedString::from(card.spoken.to_owned()))
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .h(px(style::CONTROL_H))
                .child(style::section_label(ui, card.label))
                .child(div().ml_2().text_caption().text_color(ui.faint).child(format!("· {}", card.what))),
        )
        .child(frame)
        .child(div().text_caption().text_color(ui.muted).child(SharedString::from(card.caption.to_owned())))
}

/// The box a preview stands in: on the window's background, with the room round it a card has.
/// Clips what moves inside it, so a row sliding past the edge is cut there.
#[track_caller]
pub fn preview_frame(ui: UiColors) -> Div {
    div().p_4().rounded(ui.radius_lg).outlined(ui, ui.border).bg(ui.background).overflow_hidden()
}

/// The sidebar's panel as a preview draws it on its own: the surface a list stands on, with the corner
/// a list has, clipped.
#[track_caller]
pub fn preview_surface(ui: UiColors) -> Div {
    div().rounded(ui.radius).overflow_hidden().outlined(ui, ui.border).bg(ui.panel)
}

/// [`preview_surface`] `height` points tall for a preview's parts to be placed on (absolutely, by
/// their distance from its top).
#[track_caller]
pub fn preview_stage(ui: UiColors, height: f32) -> Div {
    preview_surface(ui).relative().w_full().h(px(height))
}
