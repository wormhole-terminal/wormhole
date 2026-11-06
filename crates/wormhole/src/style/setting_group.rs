//! A settings page's group: its name above, then its rows in one rounded card.
//!
//! The card is the one surface and the rows sit flat on it, as in System Settings: no outline and no
//! line along its edge, and between two rows a faint divider that stops at the card's padding on both
//! sides. The group draws the dividers, not the rows, so one row, the first and the last all look right
//! and a row never has to know its neighbours. Under Increase Contrast, where a faint fill is not
//! enough to tell a surface from the page, the card gets its `card.edge` outline.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{Edges, TextRole};
use crate::settings::UiColors;

/// The head of a group, above its [`setting_group`] card: the name as a quiet label (13 / 600, muted)
/// from the card's inner edge, and a note at the right ("Same values as the funnel", caption, faint).
/// It has no space below it: what holds the head and the card puts `gap_2` between them.
#[track_caller]
pub fn setting_group_head(ui: UiColors, title: &str, aside: Option<&str>) -> Div {
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap_4()
        .px_4()
        .child(div().text_label().text_color(ui.muted).child(title.to_string()))
        .children(aside.map(|aside| {
            div().min_w_0().truncate().text_caption().text_color(ui.faint).child(aside.to_string())
        }))
}

/// A paragraph that belongs to a group, between its head and its card: muted caption text from the
/// card's inner edge.
#[track_caller]
pub fn setting_group_blurb(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div().px_4().text_caption().text_color(ui.muted).child(text.into())
}

/// A group's rows in one card (`radius_lg`, the `card` fill, 16 pt of padding at the sides), a divider
/// between each two. Each element is one row: a [`setting`](super::setting) with what hangs under it, or
/// a list item a page builds itself. They carry no line of their own.
///
/// With Borders off the dividers keep their space and draw nothing, and the card holds its shape by its
/// fill alone.
#[track_caller]
pub fn setting_group(ui: UiColors, rows: impl IntoIterator<Item = AnyElement>) -> Div {
    let mut card = div()
        .flex()
        .flex_col()
        .w_full()
        .px_4()
        .py_1()
        .rounded(ui.radius_lg)
        .bg(ui.band)
        .when(ui.increase_contrast, |card| card.outlined_always(ui, ui.card_edge));
    for (ix, row) in rows.into_iter().enumerate() {
        if ix > 0 {
            card = card.child(divider(ui));
        }
        card = card.child(row);
    }
    card
}

/// The line between two rows: as wide as the card's content, so it starts at the label and ends at the
/// controls' edge.
#[track_caller]
fn divider(ui: UiColors) -> Div {
    div().flex_none().w_full().h(ui.border_width).bg(ui.line(ui.border))
}
