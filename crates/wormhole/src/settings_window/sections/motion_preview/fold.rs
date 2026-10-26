//! A group folding and opening: the group's head, its two terminals and a terminal under the group. The
//! stage is as tall as the group open: folded, the rows' room is left empty for as long as it lasts, which
//! is the point of the scene (a stage that shrank with it would move the page under it).
//! Classic: the rows leave at once when it folds, and the terminal under the group slides up to where
//! they were; opening, the rows fade in and the one under slides back down.

use gpui_kit::*;
use wormhole_core::view::motion::presence;

use super::{ASKING, IDLE, Spot, WORKING, metrics, row, span};
use crate::settings::UiColors;
use crate::settings_window::sections::samples::SampleHead;
use crate::style::motion::{EASE_OUT, MOTION_MOVE};
use crate::style::preview_stage;

pub(super) fn stage(ui: UiColors, t: f32, cx: &App) -> Div {
    let metrics = metrics();
    let (head_h, row_h) = (metrics.head_h, metrics.row_h);
    let ease = EASE_OUT.easing();
    let (folds, opens) = (span(0.15, MOTION_MOVE), span(0.6, MOTION_MOVE));
    // The head shows its chevron open until the fold starts, and again once it opens.
    let folded = folds.start <= t && t < opens.start;
    let head = Spot::row(0.0, head_h).put(|| {
        SampleHead {
            id: format!("motion-fold-head-{folded}").into(),
            name: "api", // text: sample (a made-up group of the preview)
            folder: None,
            color: None,
            count: 2,
            folded,
        }
        .draw(ui, metrics)
    });
    // The rows go at once and come back fading in.
    let opacity = if t < folds.start {
        1.0
    } else if t < opens.start {
        0.0
    } else {
        opens.progress(t)
    };
    let rows = [WORKING, ASKING].into_iter().enumerate().filter_map(|(at, sample)| {
        Spot::row(head_h + at as f32 * row_h, row_h)
            .showing(opacity)
            .put(|| row(&sample, &format!("motion-fold-{at}"), ui, cx))
    });
    // The terminal under the group rides up into the room the rows leave, and back down.
    let rise = -2.0 * row_h * presence(t, folds, opens, &ease);
    let under = Spot::row(head_h + 2.0 * row_h, row_h)
        .moved(0.0, rise)
        .put(|| row(&IDLE, "motion-fold-under", ui, cx));
    preview_stage(ui, head_h + 3.0 * row_h).children(head).children(rows).children(under)
}
