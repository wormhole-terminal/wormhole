//! A split in the terminal list: every terminal of a split is a row of its own, next to the others
//! in one card, joined by a thin line left of the tiles; a chain mark on each says they belong
//! together when they can't be next to each other. The row that has the selected look stays one: the focused terminal's.

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::icon;
use super::metrics::Metrics;
use crate::settings::UiColors;
use gpui_kit::assets::IconName;
use wormhole_core::sidebar::Bracket;

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

/// The line along a split's rows, left of their tiles: from the middle of the first row to the
/// middle of the last, so it never reaches a card's edge. Only its two ends are rounded, so the
/// stretches in the rows between meet without a notch.
#[track_caller]
pub fn split_line(part: Bracket, metrics: Metrics, ui: UiColors) -> Div {
    let (top, bottom) = match part {
        Bracket::Start => (relative(0.5), px(0.0).into()),
        Bracket::Mid => (px(0.0).into(), px(0.0).into()),
        Bracket::End => (px(0.0).into(), relative(0.5)),
    };
    div()
        .absolute()
        .left(px((metrics.row_px / 2.0 - 1.0).max(1.0)))
        .top(top)
        .bottom(bottom)
        .w(px(2.0))
        .bg(ui.muted)
        .when(part == Bracket::Start, |d| d.rounded_t(px(1.0)))
        .when(part == Bracket::End, |d| d.rounded_b(px(1.0)))
}
