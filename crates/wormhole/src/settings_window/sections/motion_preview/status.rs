//! A terminal's status changing: one row between two quiet ones goes from idle to working, to asking
//! for input, to done. Today the tile changes at once: the light starts, the amber edge and the badge
//! are there from the first frame of the new status.

use crate::style::{RowTone, StatusGlyph};
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::view::motion::Span;
use wormhole_text::keys::SETTINGS_MOTION_STATUS_PREVIEW_LABEL;
use wormhole_text::text;

use super::{IDLE, Pose, RUNNING, Sample, Scene, fixed, metrics, show};
use crate::settings::settings;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::preview_stage;

/// The terminal whose status changes, in each status it goes through.
const STATES: [Sample; 4] = [
    state("Rate limits", "~", StatusGlyph::Idle, RowTone::Plain), // text: sample (a made-up terminal of the preview)
    state("Rate limits", "Working · 4m", StatusGlyph::Working, RowTone::Plain), // text: sample (a made-up status of the preview)
    state("Rate limits", "Needs input · Allow edit?", StatusGlyph::NeedsYou, RowTone::Attention), // text: sample (a made-up status of the preview)
    state("Rate limits", "Finished · 1m", StatusGlyph::Done, RowTone::Done), // text: sample (a made-up status of the preview)
];

const fn state(name: &'static str, detail: &'static str, glyph: StatusGlyph, tone: RowTone) -> Sample {
    Sample { name, detail, glyph, tone, selected: false, agent: Some(AgentId::Claude) }
}

pub(in crate::settings_window::sections) fn status(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> Option<AnyElement> {
    let ui = ui(cx);
    let scene = Scene::of("motion-status", settings(cx).motion.status);
    let row_h = metrics().row_h;
    let mut stage = preview_stage(ui, 3.0 * row_h)
        .child(fixed(0.0, row_h, IDLE.row("motion-status", 10, ui, cx)))
        .child(fixed(2.0 * row_h, row_h, RUNNING.row("motion-status", 11, ui, cx)));
    // Each status is a row of its own on the same place; the loop shows one at a time, so the change
    // is a cut. Its tile keeps its own animations while it is shown.
    for (lane, sample) in STATES.into_iter().enumerate() {
        let shown = Span::new(lane as f32 / 4.0, (lane + 1) as f32 / 4.0);
        stage = stage.child(scene.lane(
            lane,
            row_h,
            row_h,
            sample.row("motion-status", lane, ui, cx),
            move |t| Pose::when(shown.contains(t)),
        ));
    }
    Some(show(ui, "motion-status-preview", text(SETTINGS_MOTION_STATUS_PREVIEW_LABEL), stage))
}
