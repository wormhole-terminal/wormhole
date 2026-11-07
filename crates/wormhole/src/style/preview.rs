//! The stage a live preview in Settings stands on, and the surfaces inside it: one flat rounded card,
//! as System Settings shows a wallpaper or an appearance, with the app's own components (never a
//! drawing of them) on it. Like a group's card it is the `settings.group` fill with no outline; under
//! Increase Contrast, where a faint fill is no edge, it gets the `card.edge` outline.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::Edges;

/// The stage: the card a preview stands on, in a group's fill and `radius_lg` corners. It sets no size
/// and no inset (the strip does, so every page's stage is the same box) and clips what moves inside it, so a
/// row sliding past the edge is cut there.
#[track_caller]
pub fn preview_frame(ui: UiColors) -> Div {
    div()
        .relative()
        .rounded(ui.radius_lg)
        .bg(ui.group)
        .overflow_hidden()
        .when(ui.increase_contrast, |d| d.outlined_always(ui, ui.card_edge))
}

/// The sidebar's panel as a preview draws it on its own: the surface a list stands on, with the corner
/// a list has, clipped. Flat on the stage, as the panel is on the window; under Increase Contrast it gets
/// the border a surface needs then.
#[track_caller]
pub fn preview_surface(ui: UiColors) -> Div {
    div()
        .rounded(ui.radius)
        .overflow_hidden()
        .bg(ui.panel)
        .when(ui.increase_contrast, |d| d.outlined_always(ui, ui.border))
}

/// [`preview_surface`] `height` points tall for a preview's parts to be placed on (absolutely, by
/// their distance from its top).
#[track_caller]
pub fn preview_stage(ui: UiColors, height: f32) -> Div {
    preview_surface(ui).relative().w_full().h(px(height))
}
