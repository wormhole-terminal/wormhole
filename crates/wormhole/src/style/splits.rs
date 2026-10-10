//! A split in the terminal list: every terminal of a split is a row of its own, and the
//! list shows that they belong together with a thin bracket in the gutter along their rows, a chain
//! mark on each when they aren't next to each other, and a small "on screen" dot on the ones shown
//! beside the selected terminal. The row that has the selected look stays one: the focused terminal's.

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::sidebar::Bracket;

use super::buttons::icon;
use super::scale::{MARK_DOT, TextRole};
use crate::settings::UiColors;
use gpui_kit::assets::IconName;

/// Where a bracket runs in a row's gutter: its line [`BRACKET_X`] from the row's edge, drawn
/// [`BRACKET_LINE`] thick, and the elbow at its first and last row reaching [`BRACKET_REACH`] to the
/// right.
pub const BRACKET_X: f32 = 6.0;
pub const BRACKET_LINE: f32 = 1.5;
pub const BRACKET_REACH: f32 = 6.0;

/// One row's piece of the bracket that joins a split's rows: a line down the gutter in the
/// muted text color that starts at the middle of the first row with an elbow turning right, runs
/// through the rows between and ends at the middle of the last one with the same elbow turned the
/// other way. Lay it over the row (which is `.relative()`): it fills the row's height.
#[track_caller]
pub fn split_bracket(ui: UiColors, part: Bracket) -> Div {
    let line = px(BRACKET_LINE);
    let mut bracket = div().absolute().left(px(BRACKET_X)).w(px(BRACKET_REACH)).border_color(ui.muted);
    bracket.style().border_widths.left = Some(line.into());
    match part {
        Bracket::Start => {
            bracket.style().border_widths.top = Some(line.into());
            bracket.top(relative(0.5)).bottom_0().rounded_tl(ui.radius_sm)
        }
        Bracket::Mid => bracket.top_0().bottom_0(),
        Bracket::End => {
            bracket.style().border_widths.bottom = Some(line.into());
            bracket.top_0().bottom(relative(0.5)).rounded_bl(ui.radius_sm)
        }
    }
}

/// "● on screen" at the right of a split sibling's first line: this terminal is shown beside
/// the selected one.
#[track_caller]
pub fn on_screen_mark(ui: UiColors) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .text_caption()
        .text_color(ui.muted)
        .child(div().flex_none().size(px(MARK_DOT)).rounded_full().bg(ui.muted))
        .child("on screen")
}

/// The chain mark on a member of a split whose other members are elsewhere in the list (another
/// folder's band between them), at the right of its first line.
#[track_caller]
pub fn chain_mark(id: impl Into<ElementId>, ui: UiColors) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .text_color(ui.muted)
        .child(icon(IconName::Link2))
        .tooltip(|window, cx| Tooltip::new("In a split with terminals in other folders").build(window, cx))
}
