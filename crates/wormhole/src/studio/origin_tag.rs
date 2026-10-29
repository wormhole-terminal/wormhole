//! The two small marks on a token's row: where its value comes from (yours, base, worked out), and,
//! for a theme with a light and a dark appearance, which of them the value is for (the chip).

use gpui_kit::*;
use wormhole_core::theme::tokens::appearance::{DARK, LIGHT};
use wormhole_text::keys::{
    STUDIO_ORIGIN_BASE, STUDIO_ORIGIN_WORKED_OUT, STUDIO_ORIGIN_YOURS, STUDIO_REACH_BOTH, STUDIO_REACH_DARK,
    STUDIO_REACH_LIGHT,
};
use wormhole_text::text;

use super::document::places::{Origin, Reach};
use crate::settings::UiColors;
use crate::style::TextRole as _;

/// The word after a token's name that says where its value comes from.
pub(super) fn origin_tag(origin: Origin, color: Hsla) -> Div {
    let word = match origin {
        Origin::Yours => text(STUDIO_ORIGIN_YOURS),
        Origin::Base => text(STUDIO_ORIGIN_BASE),
        Origin::WorkedOut => text(STUDIO_ORIGIN_WORKED_OUT),
    };
    div().flex_none().text_caption().text_color(color).child(word)
}

/// The chip of a row in a pair: "both", or the appearance (`shown`, its name in the file) the value
/// is for alone. Clicking it moves the value to the other.
pub(super) fn reach_chip(
    token: &str,
    reach: Reach,
    shown: &str,
    color: Hsla,
    ui: UiColors,
    on_click: impl Fn(String, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let word = match (reach, shown) {
        (Reach::Both, _) => text(STUDIO_REACH_BOTH),
        (Reach::ThisAppearance, DARK) => text(STUDIO_REACH_DARK),
        (Reach::ThisAppearance, LIGHT) => text(STUDIO_REACH_LIGHT),
        (Reach::ThisAppearance, _) => text(STUDIO_REACH_BOTH),
    };
    let token = token.to_string();
    div()
        .id(SharedString::from(format!("studio-reach-{token}")))
        .flex_none()
        .cursor_pointer()
        .child(crate::style::status_badge(ui, color, word))
        .on_click(move |_, window, cx| on_click(token.clone(), window, cx))
}
