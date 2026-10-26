//! A terminal's status changing: one row between two quiet ones goes from idle to working, to asking
//! for input, to done. Classic: the tile changes at once: the light starts, the amber edge and the
//! badge are there from the first frame of the new status.

use gpui_kit::*;

use super::{IDLE, RUNNING, Spot, metrics, row};
use crate::settings::UiColors;
use crate::settings_window::sections::samples::{FLAKY_LOGIN, RATE_LIMITS, RELEASE_NOTES, Sample};
use crate::style::preview_stage;

/// The terminal whose status changes, in each status it goes through: what the agents' sample list
/// shows of its three agents, as one agent's.
const STATES: [Sample; 4] = [
    Sample { glyph: IDLE.glyph, detail: IDLE.detail, tone: IDLE.tone, selected: false, ..RATE_LIMITS },
    Sample { selected: false, ..RATE_LIMITS },
    Sample {
        glyph: FLAKY_LOGIN.glyph,
        detail: FLAKY_LOGIN.detail,
        tone: FLAKY_LOGIN.tone,
        selected: false,
        ..RATE_LIMITS
    },
    Sample {
        glyph: RELEASE_NOTES.glyph,
        detail: RELEASE_NOTES.detail,
        tone: RELEASE_NOTES.tone,
        selected: false,
        ..RATE_LIMITS
    },
];

pub(super) fn stage(ui: UiColors, t: f32, cx: &App) -> Div {
    let row_h = metrics().row_h;
    // The loop is four equal stretches, one status each: the change is a cut, and only the status
    // shown is built, so the others have no tile running a light or an edge.
    let at = ((t * STATES.len() as f32) as usize).min(STATES.len() - 1);
    let changing = Spot::row(row_h, row_h).put(|| row(&STATES[at], &format!("motion-status-{at}"), ui, cx));
    preview_stage(ui, 3.0 * row_h)
        .children(Spot::row(0.0, row_h).put(|| row(&IDLE, "motion-status-above", ui, cx)))
        .children(changing)
        .children(Spot::row(2.0 * row_h, row_h).put(|| row(&RUNNING, "motion-status-below", ui, cx)))
}
