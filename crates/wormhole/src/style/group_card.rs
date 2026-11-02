//! The card a group of Settings rows sits in: one rounded surface a step off the page, the rows
//! inside it with a hairline between them while Borders is on and none while it is off.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;

/// How far a card's rows are inset from its edges. The hover fill of a row reaches 8 pt past its
/// text, so the card's inset leaves it room to stay inside.
pub(super) const CARD_PAD_X: f32 = 16.0;

/// `rows` in one card: `settings.group` as its fill, `radius_lg` corners, a [`CARD_PAD_X`] inset at the
/// sides and none above or below, so a row's hover fill meets the card's own corners.
///
/// Every row draws a hairline above itself (`rule_top`), the first one included. The card takes that
/// one out by lifting its rows by the line's width and clipping what rises past its top edge, so a
/// row keeps one look wherever it stands and the group needs no first-row special case. Nothing
/// moves when Borders changes: the lines keep their width and only their colour goes.
#[track_caller]
pub fn group_card(ui: UiColors, rows: impl IntoIterator<Item = AnyElement>) -> Div {
    div()
        .flex()
        .flex_col()
        .w_full()
        .overflow_hidden()
        .rounded(ui.radius_lg)
        .bg(ui.group)
        .px(px(CARD_PAD_X))
        .child(div().flex().flex_col().mt(-ui.border_width).children(rows))
}
