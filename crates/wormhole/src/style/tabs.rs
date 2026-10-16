//! The tab bars a window shows instead of the sidebar (Settings → Show terminals in → Tabs): a
//! terminal's tab (its status tile, name and second line, like its row), a set's chip (two rows),
//! a pile of a set's tabs (stacks) and a segment of the bottom bar. A set is a folder's terminals
//! or one of the person's groups, coloured as its band is.

use gpui_kit::prelude::*;
use gpui_kit::*;

use std::sync::Arc;

use super::controls::keycap;
use super::motion::{EASE_IN, EASE_OUT, MOTION_BASE, MOTION_FAST};
use super::rows::{RowTone, TILE_GAP};
use super::scale::{CARD_ROW_RADIUS, Edges, META_LINE, NAME_LINE, TAB_MIN_W, TAB_W, TextRole};
use crate::settings::UiColors;

/// The parts of a [`terminal_tab`].
pub struct TerminalTab {
    /// A `style::status_tile`.
    pub tile: AnyElement,
    pub name: SharedString,
    /// The second line, as the terminal's row has it.
    pub detail: Option<SharedString>,
    pub tone: RowTone,
    /// The second line in this (code) font: a command, a path.
    pub detail_font: Option<SharedString>,
    pub selected: bool,
    /// The ⌘N keycap while ⌘ is held.
    pub hint: Option<SharedString>,
    /// Drawn [`TAB_W`] wide whatever its name (the stacks and the bottom bar), instead of
    /// sharing the row down to [`TAB_MIN_W`].
    pub fixed: bool,
}

/// One terminal's tab: its status tile, then its name over its second line; the selected one in
/// the sidebar's selection, the others with a hover fill. Add `.on_click(…)`.
#[track_caller]
pub fn terminal_tab(id: impl Into<ElementId>, ui: UiColors, tab: TerminalTab) -> Stateful<Div> {
    let TerminalTab { tile, name, detail, tone, detail_font, selected, hint, fixed } = tab;
    let tone_color = match tone {
        RowTone::Plain if selected => ui.selected_muted,
        RowTone::Plain => ui.muted,
        RowTone::Attention => ui.attention_text,
        RowTone::Done => ui.done_text,
        RowTone::Failed => ui.danger_text,
    };
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap(px(TILE_GAP))
        .min_h(px(TAB_MIN_H))
        .pl(px(TAB_PAD_LEAD))
        .pr(px(TAB_PAD_END))
        .rounded(px(CARD_ROW_RADIUS))
        .overflow_hidden()
        .map(|d| {
            if fixed {
                d.flex_none().w(px(TAB_W))
            } else {
                d.flex_shrink_1().flex_basis(px(TAB_MAX_W)).max_w(px(TAB_MAX_W)).min_w(px(TAB_MIN_W))
            }
        })
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.selected_row))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(tile)
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .h(rems(NAME_LINE))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_body()
                                .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                                .text_color(if selected { ui.selected_text } else { ui.text })
                                .child(name),
                        )
                        .children(hint.map(|hint| keycap(ui, hint))),
                )
                .children(detail.map(|detail| {
                    div()
                        .h(rems(META_LINE))
                        .min_w_0()
                        .truncate()
                        .text_caption()
                        .text_color(tone_color)
                        .when_some(detail_font, |d, font| d.font_family(font))
                        .child(detail)
                })),
        )
}

/// A tab's least height (its tile and room round it), the most a shared one grows to, and its
/// padding: less before the tile than after the text.
const TAB_MIN_H: f32 = 44.0;
const TAB_MAX_W: f32 = 220.0;
const TAB_PAD_LEAD: f32 = 6.0;
const TAB_PAD_END: f32 = 10.0;

/// The parts of a [`group_chip`].
pub struct GroupChip {
    /// The set's colour; `None` draws a faint swatch.
    pub color: Option<Hsla>,
    pub name: SharedString,
    pub count: usize,
    /// It is the selected terminal's set.
    pub selected: bool,
    /// A terminal in it needs you (amber) or failed (danger): a dot on its corner while it isn't
    /// the selected set.
    pub alert: Option<Hsla>,
}

/// A set on the first row of the two-row tab bar: its colour's swatch, its name and how many
/// terminals it has; the selected set on a tint of its colour. Add `.on_click(…)`.
#[track_caller]
pub fn group_chip(id: impl Into<ElementId>, ui: UiColors, chip: GroupChip) -> Stateful<Div> {
    let GroupChip { color, name, count, selected, alert } = chip;
    let swatch = color.unwrap_or(ui.faint);
    div()
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .gap_2()
        .h(px(CHIP_H))
        .px(px(CHIP_PAD))
        .rounded(ui.radius)
        .text_body()
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .cursor_pointer()
        .text_color(if selected { ui.text } else { ui.muted })
        .when(selected, |d| d.bg(swatch.opacity(CHIP_TINT)).outlined(ui, swatch.opacity(CHIP_OUTLINE)))
        .when(!selected, |d| d.hover(|s| s.bg(ui.hover)))
        .child(div().flex_none().size(px(SWATCH)).rounded(ui.radius_sm).bg(swatch))
        .child(name)
        .child(div().text_caption().text_color(ui.faint).child(count.to_string()))
        .children(alert.filter(|_| !selected).map(|alert| {
            div()
                .absolute()
                .top(px(ALERT_INSET))
                .right(px(ALERT_INSET))
                .size(px(ALERT_DOT))
                .rounded_full()
                .bg(alert)
        }))
}

const CHIP_H: f32 = 28.0;
const CHIP_PAD: f32 = 10.0;
const SWATCH: f32 = 10.0;
const CHIP_TINT: f32 = 0.16;
const CHIP_OUTLINE: f32 = 0.4;
const ALERT_DOT: f32 = 7.0;
const ALERT_INSET: f32 = 3.0;

/// The name of the selected set before its tabs (stacks), on a pill of its colour.
#[track_caller]
pub fn set_label(ui: UiColors, color: Option<Hsla>, name: impl Into<SharedString>) -> Div {
    let color = color.unwrap_or(ui.faint);
    div()
        .flex_none()
        .px_2()
        .py_1()
        .rounded(ui.radius_chip)
        .bg(color.opacity(PILL_TINT))
        .text_caption()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ui.text)
        .whitespace_nowrap()
        .child(name.into())
}

const PILL_TINT: f32 = 0.28;

/// How a [`tab_pile`]'s cards behind are moving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PileFan {
    /// Stacked close.
    Rest,
    /// Fanning out: the pointer just came onto it.
    Opening,
    /// Closing back up: the pointer just left it.
    Closing,
}

/// The parts of a [`tab_pile`].
pub struct TabPile {
    pub color: Option<Hsla>,
    pub name: SharedString,
    /// "3 · 1 waiting": how many, and the loudest state among them.
    pub summary: SharedString,
    pub summary_color: Hsla,
    /// The status tile of the terminal that most wants attention in it.
    pub lead: AnyElement,
    pub fan: PileFan,
}

/// A set that isn't the selected one, in the stacks layout: its tabs piled up, the front card
/// showing the tile of the terminal that most wants attention, its name and a summary. Under the
/// pointer the cards behind it fan out (and under Reduce Motion simply sit fanned). Add
/// `.on_click(…)` (it opens the set).
#[track_caller]
pub fn tab_pile(id: impl Into<ElementId>, ui: UiColors, pile: TabPile) -> Stateful<Div> {
    let TabPile { color, name, summary, summary_color, lead, fan } = pile;
    let id: ElementId = id.into();
    let color = color.unwrap_or(ui.faint);
    // A card behind, `depth` back, at `spread` (0 stacked, 1 fanned out).
    let card = move |depth: f32, spread: f32| {
        let (x, y) = (PILE_STEP_X + spread * PILE_FAN_X, PILE_STEP_Y + spread * PILE_FAN_Y);
        div()
            .absolute()
            .top(px(-y * depth))
            .left(px(x * depth))
            .size_full()
            .rounded(px(PILE_RADIUS))
            .bg(ui.band_solid)
            .child(
                div()
                    .size_full()
                    .rounded(px(PILE_RADIUS))
                    .bg(color.opacity((PILE_TINT + spread * PILE_HOVER) * (1.0 - depth * 0.25)))
                    .outlined(ui, color.opacity(PILE_OUTLINE)),
            )
    };
    let behind = |depth: f32| -> AnyElement {
        match fan {
            PileFan::Rest => card(depth, 0.0).into_any_element(),
            PileFan::Opening | PileFan::Closing => {
                let opening = fan == PileFan::Opening;
                let (name, curve, length) = if opening {
                    ("fan-out", EASE_OUT, MOTION_BASE)
                } else {
                    ("fan-in", EASE_IN, MOTION_FAST)
                };
                div()
                    .absolute()
                    .size_full()
                    .with_animation(
                        ElementId::NamedChild(Arc::new(id.clone()), format!("{name}-{depth}").into()),
                        Animation::new(length).with_easing(curve.easing()),
                        move |d, t| d.child(card(depth, if opening { t } else { 1.0 - t })),
                    )
                    .into_any_element()
            }
        }
    };
    let front_tint = if fan == PileFan::Opening { PILE_TINT + PILE_HOVER } else { PILE_TINT };
    div()
        .id(id.clone())
        .relative()
        .flex_none()
        // Room for the cards behind fanned out: above, and as much below so the front card is
        // centred like the tabs beside it.
        .my(px(2.0 * (PILE_STEP_Y + PILE_FAN_Y)))
        .mr(px(2.0 * (PILE_STEP_X + PILE_FAN_X)))
        .cursor_pointer()
        .child(behind(2.0))
        .child(behind(1.0))
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .gap(px(TILE_GAP))
                .min_h(px(TAB_MIN_H))
                .pl(px(TAB_PAD_LEAD))
                .pr_3()
                .rounded(px(PILE_RADIUS))
                .bg(ui.band_solid)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .rounded(px(PILE_RADIUS))
                        .bg(color.opacity(front_tint))
                        .outlined(ui, color.opacity(PILE_OUTLINE)),
                )
                .child(lead)
                .child(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_body()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(ui.text)
                                .whitespace_nowrap()
                                .child(name),
                        )
                        .child(
                            div().text_caption().text_color(summary_color).whitespace_nowrap().child(summary),
                        ),
                ),
        )
}

const PILE_RADIUS: f32 = 10.0;
/// How far each card behind sits from the one in front, and how much further it goes fanned out:
/// mostly sideways, since upwards the two behind have only the tab bar's 8 pt over the tabs.
const PILE_STEP_X: f32 = 4.0;
const PILE_FAN_X: f32 = 6.0;
const PILE_STEP_Y: f32 = 2.0;
const PILE_FAN_Y: f32 = 2.0;
const PILE_TINT: f32 = 0.14;
const PILE_HOVER: f32 = 0.08;
const PILE_OUTLINE: f32 = 0.35;

/// A set on the bottom bar: a rounded segment with its colour along its left edge and its name;
/// the caller adds its tabs (the selected set) or its lead tile and count (the others).
#[track_caller]
pub fn tab_segment(
    id: impl Into<ElementId>,
    ui: UiColors,
    color: Option<Hsla>,
    name: impl Into<SharedString>,
) -> Stateful<Div> {
    let color = color.unwrap_or(ui.faint);
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        .h(px(SEGMENT_H))
        .pr_1()
        .rounded(px(PILE_RADIUS))
        .overflow_hidden()
        .bg(ui.band)
        .outlined(ui, color.opacity(PILE_OUTLINE))
        // A stripe inset from the edge: `overflow_hidden` clips square, so a bar along the edge
        // itself would cut off the segment's rounded corners.
        .child(
            div()
                .absolute()
                .top(px(SEGMENT_EDGE_INSET))
                .bottom(px(SEGMENT_EDGE_INSET))
                .left(px(SEGMENT_EDGE_LEFT))
                .w(px(SEGMENT_EDGE))
                .rounded_full()
                .bg(color),
        )
        .child(
            div()
                .flex_none()
                .pl_3()
                .pr_2()
                .text_caption()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ui.muted)
                .whitespace_nowrap()
                .child(name.into()),
        )
}

const SEGMENT_H: f32 = 50.0;
const SEGMENT_EDGE: f32 = 3.0;
/// The stripe's distance from the segment's top and bottom, past its corners, and from its left.
const SEGMENT_EDGE_INSET: f32 = PILE_RADIUS;
const SEGMENT_EDGE_LEFT: f32 = 4.0;
