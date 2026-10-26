//! A group folding and opening: the group's head, its three terminals and a terminal under the
//! group. Today the rows leave at once when it folds, and the terminal under the group slides up to
//! where they were; opening, the rows fade in and the one under slides back down.

use gpui_kit::*;
use wormhole_core::view::motion::presence;
use wormhole_text::keys::SETTINGS_MOTION_FOLD_PREVIEW_LABEL;
use wormhole_text::text;

use super::{ASKING, IDLE, Pose, RUNNING, Scene, WORKING, metrics, show, span};
use crate::settings::settings;
use crate::settings_window::sections::preview::sample_head;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::motion::{EASE_OUT, MOTION_MOVE};
use crate::style::preview_stage;

pub(in crate::settings_window::sections) fn fold(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> Option<AnyElement> {
    let ui = ui(cx);
    let scene = Scene::of("motion-fold", settings(cx).motion.fold);
    let metrics = metrics();
    let (head_h, row_h) = (metrics.head_h, metrics.row_h);
    let (folds, opens) = (span(0.15, MOTION_MOVE), span(0.6, MOTION_MOVE));
    let group = |folded: bool| {
        sample_head(
            format!("motion-fold-head-{folded}").into(),
            ui,
            metrics,
            "api", // text: sample (a made-up group of the preview)
            None,
            None,
            3,
            folded,
        )
    };
    let mut stage = preview_stage(ui, head_h + 4.0 * row_h);
    // The head shows its chevron open until the fold starts, and again once it opens.
    stage =
        stage
            .child(scene.lane(0, 0.0, head_h, group(false), move |t| {
                Pose::when(t < folds.start || t >= opens.start)
            }));
    stage = stage.child(
        scene.lane(1, 0.0, head_h, group(true), move |t| Pose::when(folds.start <= t && t < opens.start)),
    );
    for (lane, sample) in [WORKING, ASKING, RUNNING].into_iter().enumerate() {
        stage = stage.child(scene.lane(
            2 + lane,
            head_h + lane as f32 * row_h,
            row_h,
            sample.row("motion-fold", lane, ui, cx),
            move |t| {
                let opacity = if t < folds.start {
                    1.0
                } else if t < opens.start {
                    0.0
                } else {
                    opens.progress(t)
                };
                Pose { opacity, ..Pose::REST }
            },
        ));
    }
    // The terminal under the group rides up into the room the rows leave, and back down.
    stage = stage.child(scene.lane(
        5,
        head_h + 3.0 * row_h,
        row_h,
        IDLE.row("motion-fold", 3, ui, cx),
        move |t| Pose { dy: -3.0 * row_h * presence(t, folds, opens, EASE_OUT.easing()), ..Pose::REST },
    ));
    Some(show(ui, "motion-fold-preview", text(SETTINGS_MOTION_FOLD_PREVIEW_LABEL), stage))
}
