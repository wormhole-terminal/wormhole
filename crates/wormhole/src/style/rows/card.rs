//! The sidebar's cards: a stretch of a card.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::metrics::Metrics;
use crate::style::scale::Edges;
use wormhole_core::view::sidebar::{WASH_FAR, WASH_PEAK, WASH_SPAN};

/// One stretch of a card in the sidebar: a group's head and its terminals, or a terminal's own, are
/// drawn as one rounded card (`metrics.card_margin` in from the sidebar's sides, `card_radius` round,
/// its rows `card_pad` in), each line of the list a stretch of it holding `content`. `top` is the
/// first line (the card's rounded top), `bottom` the last (its rounded bottom, and `card_gap` under
/// it). The gaps are part of the stretch, so a drop there lands on its line. Without cards the lines
/// stand in by the same margin with no fill or edge, and the gap under a group is what sets it apart.
/// A group's colour (`wash`) fills the card evenly at `WASH_PEAK` (22 %), or with `metrics.fade_wash`
/// tints it from its left edge, `WASH_PEAK` fading to 3 % at 60 % of its width: each stretch draws the
/// same, so the card reads as one.
#[track_caller]
pub fn card_segment(
    ui: UiColors,
    metrics: Metrics,
    top: bool,
    bottom: bool,
    wash: Option<Hsla>,
    content: impl IntoElement,
) -> Div {
    card_segment_in(ui, metrics, top, bottom, wash, None, content)
}

/// A card of one row: the row is the card, with no padding inside, and `selected` (or the pointer)
/// colours the card's own fill and edge instead of a second rounded shape inside it. Without cards
/// it is a plain stretch, as [`card_segment`] makes.
#[track_caller]
pub fn card_segment_solo(ui: UiColors, metrics: Metrics, selected: bool, content: impl IntoElement) -> Div {
    card_segment_in(ui, metrics, true, true, None, Some(selected), content)
}

#[track_caller]
fn card_segment_in(
    ui: UiColors,
    metrics: Metrics,
    top: bool,
    bottom: bool,
    wash: Option<Hsla>,
    solo: Option<bool>,
    content: impl IntoElement,
) -> Div {
    let outer = div().flex().flex_col().px(px(metrics.card_margin));
    let gap = px(metrics.card_gap);
    if !metrics.cards {
        return outer.when(bottom, |d| d.pb(gap)).child(content);
    }
    let card_pad = if solo.is_some() { 0.0 } else { metrics.card_pad };
    let mut segment = div().relative().flex().flex_col().px(px(card_pad));
    let edge = Some(ui.border_width.into());
    {
        let b = &mut segment.style().border_widths;
        b.left = edge;
        b.right = edge;
    }
    let selected = solo == Some(true);
    segment = segment.bg(if selected { ui.selected_row } else { ui.card }).border_color(if selected {
        ui.selected_row
    } else {
        ui.card_edge
    });
    if solo == Some(false) {
        segment = segment.hover(move |d| d.bg(ui.hover));
    }
    {
        let b = &mut segment.style().border_widths;
        if top {
            b.top = edge;
        }
        if bottom {
            b.bottom = edge;
        }
    }
    let radius = px(metrics.card_radius);
    let pad = px(card_pad);
    let segment = segment
        .when(top, |d| d.rounded_t(radius).pt(pad))
        .when(bottom, |d| d.rounded_b(radius).pb(pad))
        .when_some(wash, |d, color| {
            d.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .when(top, |d| d.rounded_t(radius))
                    .when(bottom, |d| d.rounded_b(radius))
                    // Every stretch of a card gets the same fill, so a card reads as one.
                    .when_else(
                        metrics.fade_wash,
                        |d| {
                            d.bg(linear_gradient(
                                90.0,
                                linear_color_stop(color.opacity(WASH_PEAK), 0.0),
                                linear_color_stop(color.opacity(WASH_FAR), WASH_SPAN),
                            ))
                        },
                        |d| d.bg(color.opacity(WASH_PEAK)),
                    ),
            )
        })
        .child(content);
    outer.when(bottom, |d| d.pb(gap)).child(segment)
}

/// A terminal's row lifted out of the list: a card of its own, `selected` as the row is, with the shadow of
/// something floating. It is what follows the pointer while a row is dragged, so it is as wide as the card
/// the row stood in (`width`, its inside) and holds `content` (the row, drawn as a lone card's, `solo`) with
/// no padding of its own.
#[track_caller]
pub fn card_lifted(
    ui: UiColors,
    metrics: Metrics,
    selected: bool,
    width: f32,
    content: impl IntoElement,
) -> Div {
    let (fill, edge) = if selected { (ui.selected_row, ui.selected_row) } else { (ui.card, ui.card_edge) };
    let card = div()
        .w(px(width))
        .flex()
        .flex_col()
        .rounded(px(metrics.card_radius))
        .outlined(ui, edge)
        .bg(fill)
        .child(content);
    crate::style::drag_lift(card, ui, 1.0)
}
