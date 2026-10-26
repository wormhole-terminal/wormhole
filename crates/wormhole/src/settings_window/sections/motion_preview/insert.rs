//! A terminal opening and closing: a stage of three terminals, a fourth coming in at the top and
//! going again. Today the new row fades in where it will stand while the rows under it slide down
//! from where they were (and back up when it goes), in the list's own move time and easing.

use gpui_kit::*;
use wormhole_core::view::motion::presence;
use wormhole_text::keys::SETTINGS_MOTION_INSERT_PREVIEW_LABEL;
use wormhole_text::text;

use super::{ASKING, IDLE, Pose, RUNNING, Scene, WORKING, metrics, show, span};
use crate::settings::settings;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::motion::{EASE_OUT, MOTION_MOVE};
use crate::style::preview_stage;

pub(in crate::settings_window::sections) fn insert(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> Option<AnyElement> {
    let ui = ui(cx);
    let scene = Scene::of("motion-insert", settings(cx).motion.insert);
    let row_h = metrics().row_h;
    let (comes, goes) = (span(0.15, MOTION_MOVE), span(0.6, MOTION_MOVE));
    let mut stage = preview_stage(ui, 4.0 * row_h);
    for (lane, sample) in [WORKING, ASKING, RUNNING].into_iter().enumerate() {
        // The rows under the new one are a row lower while it is there.
        stage = stage.child(scene.lane(
            lane,
            lane as f32 * row_h,
            row_h,
            sample.row("motion-insert", lane, ui, cx),
            move |t| Pose { dy: row_h * presence(t, comes, goes, EASE_OUT.easing()), ..Pose::REST },
        ));
    }
    // The new row is drawn from its first moment, fades in, and is gone at once when it closes.
    stage = stage.child(scene.lane(3, 0.0, row_h, IDLE.row("motion-insert", 3, ui, cx), move |t| Pose {
        opacity: if goes.start <= t { 0.0 } else { comes.progress(t) },
        ..Pose::REST
    }));
    Some(show(ui, "motion-insert-preview", text(SETTINGS_MOTION_INSERT_PREVIEW_LABEL), stage))
}
