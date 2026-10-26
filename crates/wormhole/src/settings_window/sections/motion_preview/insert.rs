//! A terminal opening and closing: a stage of three terminals, a fourth coming in at the top and
//! going again. Classic: the new row fades in where it will stand while the rows under it slide down
//! from where they were (and back up when it goes), in the list's own move time and easing.

use gpui_kit::*;
use wormhole_core::view::motion::presence;

use super::{ASKING, IDLE, RUNNING, Spot, WORKING, metrics, row, span};
use crate::settings::UiColors;
use crate::style::motion::{EASE_OUT, MOTION_MOVE};
use crate::style::preview_stage;

pub(super) fn stage(ui: UiColors, t: f32, cx: &App) -> Div {
    let row_h = metrics().row_h;
    let ease = EASE_OUT.easing();
    let (comes, goes) = (span(0.15, MOTION_MOVE), span(0.6, MOTION_MOVE));
    // The rows under the new one are a row lower while it is there.
    let slide = row_h * presence(t, comes, goes, &ease);
    // The new row is there from its first moment, fades in, and is gone at once when it closes.
    let opacity = if goes.start <= t { 0.0 } else { comes.progress(t) };
    let existing = [WORKING, ASKING, RUNNING].into_iter().enumerate().filter_map(|(at, sample)| {
        Spot::row(at as f32 * row_h, row_h)
            .moved(0.0, slide)
            .put(|| row(&sample, &format!("motion-insert-{at}"), ui, cx))
    });
    let new = Spot::row(0.0, row_h).showing(opacity).put(|| row(&IDLE, "motion-insert-new", ui, cx));
    preview_stage(ui, 4.0 * row_h).children(existing).children(new)
}
