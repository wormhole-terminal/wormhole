//! The lists' rows: square, edge to edge, inset 16, with
//! one lead column at x 16–32 and the text at x 40. A selection is the primary's tint,
//! hover a 5 % fill, and neither moves anything. By file: `session` a terminal's row, `card` the sidebar's
//! cards and Settings' navigation row, `files` the Changes tab's rows, `commits` the two-line rows.

mod card;
mod commits;
mod detail;
mod files;
mod session;

pub use card::*;
pub use commits::*;
pub use detail::DetailRoll;
pub use files::*;
pub use session::*;

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{ITEM_LEAD_W, ROW_H, TextRole};
use crate::settings::UiColors;

/// A square row that runs from edge to edge: [`ROW_H`] tall, inset 16 on both sides (the right
/// panel's rows, which sit in cards, are inset [`CARD_INSET`] and rounded instead),
/// body text with a gap of `_2` between its parts. The hover fill is `state.hover`; a
/// selected row (an open file, an expanded commit) takes the primary's tint
/// (`state.selected`) with its text in `selected_text`, and keeps it under the pointer.
/// Start it with a [`ITEM_LEAD_W`] lead (`style::lead`) so its text lands at x 40.
#[track_caller]
pub fn flush_row(id: impl Into<ElementId>, ui: UiColors, selected: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .min_h(px(ROW_H))
        .px_4()
        .text_body()
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.selected).text_color(ui.selected_text))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
}

/// The lead column of a row or band (x 16–32): a status glyph, a chevron, a checkbox or a
/// file icon, centered.
#[track_caller]
pub fn lead(child: impl IntoElement) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .w(px(ITEM_LEAD_W))
        .h(px(ITEM_LEAD_W))
        .child(child)
}

/// The gap between a status tile and the lines beside it (at the regular density).
pub const TILE_GAP: f32 = 10.0;

/// The gap between the lead column and the name (`_2`), which a second line skips too.
pub(super) const SECOND_LINE_GAP: f32 = 8.0;
