//! The pieces of the title row's extras: the branch's status, the agent's
//! context meter and the small chips the ports and the pull request are. They are drawn at the
//! row's own height, in its caption type, and each knows its own fixed widths so the row can
//! measure them (`workspace::title_extras`) before it draws them.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{Edges, MARK_DOT, TextRole};
use crate::settings::UiColors;

/// Height of a chip in the title row: the branch chip's.
pub const EXTRA_H: f32 = 20.0;
/// A chip's side padding (`px_2`), and the gap between its icon and its text (`gap_1`).
pub const EXTRA_PAD: f32 = 8.0;
pub const EXTRA_GAP: f32 = 4.0;
/// The small icons inside the extras (`Icon::xsmall`).
pub const EXTRA_ICON: f32 = 12.0;
/// The context meter's track in the title row.
pub const METER_W: f32 = 28.0;
const METER_H: f32 = 4.0;
/// The dot of a branch that has uncommitted changes.
pub const STATUS_DOT: f32 = MARK_DOT;

/// A chip in the title row: an optional icon (in `icon_color`) and a short text, 20 tall. A
/// `filled` one sits on the neutral fill and reads in the text color (a port, a pull request);
/// without it only the muted text is left (the "+2" after the ports). Its width is
/// `2 * EXTRA_PAD` plus the text, plus `EXTRA_ICON + EXTRA_GAP` with an icon.
#[track_caller]
pub fn extra_chip(
    ui: UiColors,
    filled: bool,
    icon: Option<(IconName, Hsla)>,
    text: impl Into<SharedString>,
) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .h(px(EXTRA_H))
        .px_2()
        .rounded(ui.radius_chip)
        .when(filled, |d| {
            // band.hover (8%) vanishes on the title row in several themes and over blur, so
            // the chip takes the stronger active fill and an outline.
            d.bg(ui.active).outlined(ui, ui.border).text_color(ui.text)
        })
        .when(!filled, |d| d.text_color(ui.muted))
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .children(
            icon.map(|(icon, color)| div().flex_none().text_color(color).child(Icon::new(icon).xsmall())),
        )
        .child(text.into())
}

/// A branch's status after its chip: an amber dot while the tree has uncommitted changes, then
/// the commits ahead of its upstream (↑) and behind it (↓) in muted digits. A count of none is
/// left out. Its width is the dot (`STATUS_DOT`), `EXTRA_GAP` after it, and each count's arrow
/// (`EXTRA_ICON`) and digits, with `EXTRA_GAP` between the counts.
#[track_caller]
pub fn branch_status(ui: UiColors, dirty: bool, ahead: u32, behind: u32) -> Div {
    let count = |icon: IconName, n: u32| {
        div()
            .flex_none()
            .flex()
            .items_center()
            .child(Icon::new(icon).xsmall())
            .child(div().tabular_nums().child(n.to_string()))
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .h(px(EXTRA_H))
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ui.muted)
        .when(dirty, |d| d.child(div().flex_none().size(px(STATUS_DOT)).rounded_full().bg(ui.warning)))
        .when(ahead > 0, |d| d.child(count(IconName::ArrowUp, ahead)))
        .when(behind > 0, |d| d.child(count(IconName::ArrowDown, behind)))
}

/// The most room a model's name takes in the gauge; a longer one is cut at its end.
pub const GAUGE_MODEL_MAX: f32 = 96.0;

/// How much of its context window an agent has used, as the gauge draws it.
#[derive(Clone, Debug)]
pub struct ContextUse {
    /// 0 to 1.
    pub share: f32,
    /// Nearly full: the meter turns to the warning color.
    pub warn: bool,
    /// "41%".
    pub percent: SharedString,
}

/// What a coding agent runs and how full its context window is: the model's name and a meter with
/// the share used, whichever the agent reported, in muted caption type. Its width is the sum of
/// what is there (the name at most [`GAUGE_MODEL_MAX`], [`METER_W`], the share's text) with
/// `EXTRA_GAP` between them and `EXTRA_PAD` at each end.
#[track_caller]
pub fn agent_gauge(ui: UiColors, model: Option<SharedString>, context: Option<ContextUse>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .h(px(EXTRA_H))
        .px_2()
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ui.muted)
        .whitespace_nowrap()
        .children(model.map(|name| div().max_w(px(GAUGE_MODEL_MAX)).truncate().child(name)))
        .children(context.map(|used| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(context_meter(ui, METER_W, used.share, used.warn))
                .child(div().tabular_nums().child(used.percent))
        }))
}

/// How full a coding agent's context window is: a `width` track ([`METER_W`] in the title row,
/// wider in the status bar) with the share used in the primary, or the warning color from `warn`
/// on, which is early enough to compact.
#[track_caller]
pub fn context_meter(ui: UiColors, width: f32, used: f32, warn: bool) -> Div {
    div().flex_none().w(px(width)).h(px(METER_H)).rounded(ui.radius_sm).bg(ui.border).overflow_hidden().child(
        div().h_full().w(relative(used.clamp(0.0, 1.0))).rounded(ui.radius_sm).bg(if warn {
            ui.warning
        } else {
            ui.primary
        }),
    )
}
