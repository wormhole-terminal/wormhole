//! The frame round a live preview in Settings, and the stage inside it: the card a page puts its
//! miniature of the app in, so every preview sits in the same box. The miniature is made of the
//! app's own components, never a drawing of them.

use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::Edges;

/// The box a preview stands in: on the window's background, with the room round it a card has.
/// Clips what moves inside it, so a row sliding past the edge is cut there.
#[track_caller]
pub fn preview_frame(ui: UiColors) -> Div {
    div().p_4().rounded(ui.radius_lg).outlined(ui, ui.border).bg(ui.background).overflow_hidden()
}

/// A surface `height` points tall for a preview's parts to be placed on (absolutely, by their
/// distance from its top): the sidebar's panel, with the corner a list has.
#[track_caller]
pub fn preview_stage(ui: UiColors, height: f32) -> Div {
    div()
        .relative()
        .w_full()
        .h(px(height))
        .rounded(ui.radius)
        .overflow_hidden()
        .outlined(ui, ui.border)
        .bg(ui.panel)
}
