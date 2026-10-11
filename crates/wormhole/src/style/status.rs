//! Status at a glance: the glyph in a row's lead column and the
//! items of the status bar.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::motion::{ANIMATION_FPS, PULSE_RANGE, SPIN_PERIOD, STATUS_PULSE};
use super::scale::{
    CONTROL_H_SM, Edges, GLYPH, ITEM_LEAD_W, STATUS_BAR_H, STATUS_DOT_SIZE, STATUS_HALO, TITLE_GLYPH,
    TextRole,
};
use crate::icons::glyph;
use crate::settings::UiColors;

/// What a terminal is doing, as its lead glyph shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusGlyph {
    /// An agent works: a 12 pt primary spinner, a turn every `SPIN_PERIOD`; three still
    /// dots under Reduce Motion.
    Working,
    /// A command runs in a terminal without an agent: the same spinner, neutral (muted).
    Running,
    /// An agent needs you: an 8 pt amber dot in a 3 pt halo that breathes (steady under
    /// Reduce Motion); the dot itself never fades.
    NeedsYou,
    /// An agent finished: a 14 pt check in `agent.done` (shape, not only color, says it).
    Done,
    /// Nothing happening: an 8 pt ring in `text.faint`.
    Idle,
    /// A terminal without an agent rang its bell: an amber bell, until it is seen.
    Bell,
    /// The last command failed: a crossed circle in `status.danger`.
    Failed,
    /// A terminal without an agent; `unread` while it has output you haven't seen.
    Terminal { unread: bool },
    /// A bare shell at its prompt (nothing running): the terminal symbol, faint.
    Shell,
}

/// `glyph` centered in the [`ITEM_LEAD_W`] lead slot. `id` keys its animation (a
/// terminal's id), so the spinner keeps turning when the list around it changes.
#[track_caller]
pub fn status_glyph(id: impl Into<ElementId>, ui: UiColors, glyph: StatusGlyph, cx: &App) -> AnyElement {
    let still = cx.reduce_motion();
    let id = id.into();
    let drawing = match glyph {
        StatusGlyph::Working => spinner(id, ui.agent_working, still),
        StatusGlyph::Running => spinner(id, ui.muted, still),
        StatusGlyph::NeedsYou => needs_you(id, ui, still),
        StatusGlyph::Done => mark(glyph::CHECK, GLYPH, ui.agent_done),
        StatusGlyph::Idle => idle_ring(ui).into_any_element(),
        StatusGlyph::Bell => mark(glyph::BELL, GLYPH, ui.agent_attention),
        StatusGlyph::Failed => mark(glyph::FAILED, TITLE_GLYPH, ui.danger),
        StatusGlyph::Terminal { unread } => {
            mark(glyph::TERMINAL, GLYPH, if unread { ui.text } else { ui.muted })
        }
        StatusGlyph::Shell => mark(glyph::TERMINAL, GLYPH, ui.faint),
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(ITEM_LEAD_W))
        .child(drawing)
        .into_any_element()
}

/// A coding agent's logo in a row's lead column, tinted like the text beside it, or a
/// two-letter monogram in a rounded square for an agent without a bundled logo; `size` square
/// either way. `logo` is the agent's icon path (`Agent::icon`).
#[track_caller]
pub fn agent_logo(
    logo: Option<&'static str>,
    monogram: impl Into<SharedString>,
    size: f32,
    color: Hsla,
) -> AnyElement {
    match logo {
        Some(path) => svg().path(path).size(px(size)).flex_none().text_color(color).into_any_element(),
        None => div()
            .size(px(size))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(size * MONOGRAM_CORNER))
            .border_1()
            .border_color(color)
            .text_color(color)
            .text_size(px(size * MONOGRAM_TEXT))
            .font_weight(FontWeight::BOLD)
            .child(monogram.into())
            .into_any_element(),
    }
}

/// A monogram's corner and letters, as shares of its size (drawn at 13 pt: 3 and 7).
const MONOGRAM_CORNER: f32 = 3.0 / 13.0;
const MONOGRAM_TEXT: f32 = 7.0 / 13.0;

/// The working spinner's size.
const SPINNER: f32 = 12.0;
/// The idle ring's line.
const RING_STROKE: f32 = 1.5;

/// The working spinner, a turn every [`SPIN_PERIOD`]; three still dots while `still`.
fn spinner(id: ElementId, color: Hsla, still: bool) -> AnyElement {
    if still {
        return still_dots(color).into_any_element();
    }
    svg()
        .path(glyph::SPINNER)
        .size(px(SPINNER))
        .text_color(color)
        .with_animation(
            (id, "spin"),
            Animation::new(SPIN_PERIOD).repeat().with_max_fps(ANIMATION_FPS),
            |svg, turn| svg.with_transformation(Transformation::rotate(percentage(turn))),
        )
        .into_any_element()
}

fn mark(path: &'static str, size: f32, color: Hsla) -> AnyElement {
    svg().path(path).flex_none().size(px(size)).text_color(color).into_any_element()
}

/// The amber dot and its halo. The halo breathes on its own 28 % tint; under Reduce
/// Motion it holds at full strength instead of GPUI's first frame.
#[track_caller]
fn needs_you(id: ElementId, ui: UiColors, still: bool) -> AnyElement {
    let halo = div().absolute().size_full().rounded_full().bg(ui.attention_ring);
    let halo = if still {
        halo.into_any_element()
    } else {
        let (low, high) = PULSE_RANGE;
        halo.with_animation(
            (id, "pulse"),
            Animation::new(STATUS_PULSE)
                .repeat()
                .with_max_fps(ANIMATION_FPS)
                .with_easing(pulsating_between(low, high)),
            |halo, opacity| halo.opacity(opacity),
        )
        .into_any_element()
    };
    div()
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(STATUS_DOT_SIZE + 2.0 * STATUS_HALO))
        .child(halo)
        .child(div().size(px(STATUS_DOT_SIZE)).rounded_full().bg(ui.agent_attention))
        .into_any_element()
}

#[track_caller]
fn idle_ring(ui: UiColors) -> Div {
    let mut ring = div().flex_none().size(px(STATUS_DOT_SIZE)).rounded_full().border_color(ui.faint);
    let stroke = Some(px(RING_STROKE).into());
    let b = &mut ring.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (stroke, stroke, stroke, stroke);
    ring
}

/// The spinner under Reduce Motion: three dots that say "busy" without moving.
#[track_caller]
fn still_dots(color: Hsla) -> Div {
    let dot = || div().size(px(STILL_DOT)).rounded_full().bg(color);
    div().flex().items_center().gap(px(STILL_DOT)).child(dot()).child(dot()).child(dot())
}

const STILL_DOT: f32 = 2.0;

/// One item of the status bar: a [`CONTROL_H_SM`] face with the control
/// corners, caption text in 500, muted unless it is the one loud item (`loud`: who needs
/// you, in `agent.attention.text`). It takes clicks across the bar's whole height (22
/// drawn, 28 hit), and with `ring` wears the keyboard focus ring round its face. Add a
/// glyph and the words with `.child(…)`, and `.on_click(…)`: each item opens what it counts.
#[track_caller]
pub fn status_item(id: impl Into<ElementId>, ui: UiColors, loud: bool, ring: bool) -> Stateful<Div> {
    let id: ElementId = id.into();
    let group = SharedString::from(format!("status-item-{id}"));
    // The face is drawn behind the words, inset from the hit area by the difference.
    let inset = px((STATUS_BAR_H - CONTROL_H_SM) / 2.0);
    let face = div().absolute().left_0().right_0().top(inset).bottom(inset).rounded(ui.radius);
    div()
        .id(id)
        .group(group.clone())
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .gap_1()
        .h(px(STATUS_BAR_H))
        .px_1()
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .tabular_nums()
        .text_color(if loud { ui.attention_text } else { ui.muted })
        .cursor_pointer()
        .child(face.group_hover(group, |s| s.bg(ui.band_hover)).when(ring, |d| {
            d.child(super::controls::focus_ring(ui, super::controls::RingPlacement::Outside, ui.radius))
        }))
}

/// How tall the hairline between groups of status bar items is.
const STATUS_SEPARATOR_H: f32 = 12.0;

/// The hairline between groups of status bar items: the branch, the agents, the terminal's
/// size.
#[track_caller]
pub fn status_separator(ui: UiColors) -> Div {
    div().flex_none().w(ui.border_width).h(px(STATUS_SEPARATOR_H)).mx_1().bg(ui.border)
}

/// A status bar item that is read, not clicked (the terminal's size): an item's metrics
/// without its hover.
#[track_caller]
pub fn status_text(ui: UiColors) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap_1()
        .h(px(CONTROL_H_SM))
        .px_1()
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .tabular_nums()
        .text_color(ui.muted)
}

/// The status bar's context meter for the selected terminal's agent: "Context" in muted type
/// (dropped with the other items' words in a narrow window), a `width` meter and the share used,
/// 12 / 600 in the text color.
#[track_caller]
pub fn status_context(
    ui: UiColors,
    width: f32,
    words: bool,
    share: f32,
    warn: bool,
    percent: impl Into<SharedString>,
) -> Div {
    status_text(ui)
        .gap_2()
        .when(words, |d| d.child("Context"))
        .child(super::context_meter(ui, width, share, warn))
        .child(div().text_color(ui.text).font_weight(FontWeight::SEMIBOLD).child(percent.into()))
}

/// The optional status bar along the bottom of a window: [`STATUS_BAR_H`]
/// tall on the sidebar's surface with a hairline on top; its items start and end at the
/// 16 pt inset (12 here, 4 inside each item).
#[track_caller]
pub fn status_bar(ui: UiColors) -> Div {
    div().flex_none().flex().items_center().gap_1().h(px(STATUS_BAR_H)).px_3().bg(ui.panel).rule_top(ui)
}
