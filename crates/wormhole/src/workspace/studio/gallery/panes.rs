//! Split panes: two pane headers, the focused one with its primary line, over a divider
//! at rest and one under the pointer.

use wormhole_core::panes::{Axis, Zone};

use super::*;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let header = |id: &'static str, focused: bool, glyph: style::StatusGlyph, name: &'static str| {
        style::pane_header(id, ui, focused, false)
            .flex_1()
            .min_w_0()
            .child(style::status_glyph((id, 1u64), ui, glyph, cx))
            .child(div().min_w_0().truncate().child(name))
            .child(
                div()
                    .min_w_0()
                    .flex_shrink(2.0)
                    .truncate()
                    .font_weight(FontWeight::NORMAL)
                    .text_color(ui.faint)
                    .child("~/Projects/terminal"),
            )
            .child(div().flex_1())
            .child(style::icon_button((id, 2u64), ui, IconName::Maximize, "Zoom pane").in_row())
            .child(style::icon_button((id, 3u64), ui, IconName::Ellipsis, "Pane menu").in_row())
            .child(style::hit_area(
                (id, 4u64),
                style::CONTROL_H_SM,
                style::close_x((id, 5u64), ui, style::SelectedFill::Plain),
                |hit| hit,
            ))
    };
    let body = |text: &'static str| {
        div().flex_1().min_w_0().h(px(PANE_SAMPLE_H)).p_3().text_caption().text_color(ui.muted).child(text)
    };
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            // A tab split in two: the left pane has the keyboard, the right one is dimmed.
            div()
                .flex()
                .w_full()
                .outlined(ui, ui.border)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(header("g-pane-focused", true, style::StatusGlyph::Working, "claude"))
                        .child(body("← focused: semibold, full strength, the primary line")),
                )
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(header("g-pane-other", false, style::StatusGlyph::Terminal { unread: false }, "zsh"))
                        .child(body("← another pane: muted, its text at 72 %").opacity(style::PANE_DIM))
                        .child(style::pane_divider("g-divider-rest", ui, Axis::Row, style::DividerLook::Rest, false)),
                ),
        )
        .child(
            // The dividers lit: under the pointer, and while dragged.
            div()
                .flex()
                .items_center()
                .gap_4()
                .h(px(DIVIDER_SAMPLE_H))
                .child(div().relative().h_full().w(px(DIVIDER_SAMPLE_W)).child(style::pane_divider(
                    "g-divider-hot",
                    ui,
                    Axis::Row,
                    style::DividerLook::Hot, false,
                )))
                .child(div().relative().h_full().w(px(DIVIDER_SAMPLE_W)).child(style::pane_divider(
                    "g-divider-drag",
                    ui,
                    Axis::Row,
                    style::DividerLook::Dragging, false,
                )))
                .child(div().relative().w(px(DIVIDER_SAMPLE_L)).h(px(DIVIDER_SAMPLE_W)).child(
                    style::pane_divider("g-divider-row", ui, Axis::Column, style::DividerLook::Hot, false),
                ))
                .child(
                    div()
                        .text_caption()
                        .text_color(ui.muted)
                        .child("← dividers: hovered, dragged, and a horizontal one (drag to resize, double-click to equalize)"),
                ),
        )
        .child(
            // With gaps between the terminals each pane is a rounded surface, outlined.
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(div().relative().w(px(DIVIDER_SAMPLE_L)).h(px(DIVIDER_SAMPLE_H)).child(style::pane_outline(ui, true)))
                .child(div().relative().w(px(DIVIDER_SAMPLE_L)).h(px(DIVIDER_SAMPLE_H)).child(style::pane_outline(ui, false)))
                .child(
                    div()
                        .text_caption()
                        .text_color(ui.muted)
                        .child("← split gaps on: the focused pane's outline, and another's"),
                ),
        )
        .child(
            // The window's columns, the lines between them as the theme draws them (the person's
            // Column dividers setting hides them in the app, not here).
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(env.mark(
                    "column.divider",
                    div()
                        .flex()
                        .w(px(DIVIDER_SAMPLE_L * 2.0))
                        .h(px(DIVIDER_SAMPLE_H))
                        .outlined(ui, ui.border)
                        .child(div().w_1_4().h_full().bg(ui.panel))
                        .child(div().w(ui.divider_width).h_full().bg(ui.divider))
                        .child(div().flex_1().h_full().bg(ui.background))
                        .child(div().w(ui.divider_width).h_full().bg(ui.divider))
                        .child(div().w_1_4().h_full().bg(ui.panel_right)),
                ))
                .child(
                    div()
                        .text_caption()
                        .text_color(ui.muted)
                        .child("← the lines between the sidebar, the terminal and the panel"),
                ),
        )
        .child(drop_zones(ui))
        .child(
            div()
                .text_caption()
                .text_color(ui.muted)
                .child("↑ a terminal dragged over a pane: where it would land, and the other sides"),
        )
        .into_any_element()
}

/// A pane as a terminal is dragged over it: the side it would land on lit with what it
/// will do, the other sides as faint outlines.
fn drop_zones(ui: UiColors) -> Div {
    div()
        .relative()
        .w_full()
        .max_w(px(DROP_SAMPLE_W))
        .h(px(DROP_SAMPLE_H))
        .outlined(ui, ui.border)
        .child(
            style::pane_drop_zone(ui, style::ZoneLook::Hint, Zone::Left)
                .left_0()
                .top_0()
                .w(px(DROP_BAND))
                .h(px(DROP_SAMPLE_H - DROP_BAND)),
        )
        .child(
            style::pane_drop_zone(ui, style::ZoneLook::Hint, Zone::Bottom)
                .left_0()
                .bottom_0()
                .w(px(DROP_SAMPLE_W))
                .h(px(DROP_BAND)),
        )
        .child(
            style::pane_drop_zone(ui, style::ZoneLook::Active, Zone::Right)
                .right_0()
                .top_0()
                .w(px(DROP_SAMPLE_W / 2.0))
                .h(px(DROP_SAMPLE_H))
                .child(style::drop_hint(ui, "Show on the right")),
        )
}

/// The sample panes' body under their headers, and room for the dividers on their own.
const PANE_SAMPLE_H: f32 = 56.0;
const DROP_SAMPLE_W: f32 = 320.0;
const DROP_SAMPLE_H: f32 = 120.0;
const DROP_BAND: f32 = 40.0;
const DIVIDER_SAMPLE_H: f32 = 48.0;
const DIVIDER_SAMPLE_W: f32 = 8.0;
const DIVIDER_SAMPLE_L: f32 = 64.0;
