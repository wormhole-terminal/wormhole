//! A terminal opening and closing: a stage of three terminals, a fourth coming in at the top and
//! going again, drawn from the poses of the chosen `InsertMotion` (core's `pose::insert`), the same
//! ones the sidebar draws. The rows under the new one make room as the motion says (`room_at`): they
//! follow its height, or slide on the motion's spring.

use gpui_kit::*;
use wormhole_core::view::motion::InsertMotion;
use wormhole_core::view::motion::pose::{Phase, Pose, insert};

use super::{ASKING, IDLE, RUNNING, Spot, WORKING, metrics, progress, row, since};
use crate::settings::{UiColors, settings};
use crate::style::preview_stage;

/// When the new terminal opens and when it closes, as shares of the loop.
const OPENS: f32 = 0.15;
const CLOSES: f32 = 0.6;

/// The new terminal at `t` of the loop, and how far down the rows under it are (in points, for rows
/// `row_h` tall) as far as its place is open to them.
fn place(variant: InsertMotion, t: f32, row_h: f32) -> (Pose, f32) {
    let (phase, elapsed) =
        if t < CLOSES { (Phase::Enter, since(t, OPENS)) } else { (Phase::Exit, since(t, CLOSES)) };
    let pose = insert::pose(variant, phase, progress(elapsed, insert::duration(variant, phase)));
    (pose, row_h * insert::room_at(variant, phase, elapsed))
}

pub(super) fn stage(ui: UiColors, t: f32, cx: &App) -> Div {
    let row_h = metrics().row_h;
    let (pose, slide) = place(settings(cx).motion.insert, t, row_h);
    // The new row first: the rows under it cover it as they slide, as they do in the list.
    let new = Spot::row(0.0, row_h)
        .moved(pose.dx, pose.dy)
        .showing(pose.opacity)
        .clipped(pose.clip)
        .put(|| row(&IDLE, "motion-insert-new", ui, cx));
    let existing = [WORKING, ASKING, RUNNING].into_iter().enumerate().filter_map(|(at, sample)| {
        Spot::row(at as f32 * row_h, row_h)
            .moved(0.0, slide)
            .put(|| row(&sample, &format!("motion-insert-{at}"), ui, cx))
    });
    preview_stage(ui, 4.0 * row_h).children(new).children(existing)
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::pose::Phase;
    use wormhole_core::view::motion::pose::insert;
    use wormhole_core::view::motion::{InsertMotion, MotionChoice};

    use super::{CLOSES, OPENS, place};
    use crate::settings_window::sections::motion_preview::LOOP;

    const ROW: f32 = 48.0;

    #[test]
    fn every_variant_starts_without_the_new_row_and_rests_with_it_and_ends_without_it() {
        for &variant in InsertMotion::ALL {
            let (before, slide) = place(variant, 0.0, ROW);
            assert!(
                before.opacity == 0.0 || before.clip == 0.0,
                "{variant:?}: the first picture is the one before"
            );
            assert_eq!(slide, 0.0, "{variant:?}: the rows are where they were");
            let (there, slide) = place(variant, CLOSES - 0.001, ROW);
            assert_eq!(
                (there.opacity, there.clip, there.dx),
                (1.0, 1.0, 0.0),
                "{variant:?}: whole in its place"
            );
            assert!((slide - ROW).abs() < 0.5, "{variant:?}: the rows are a row lower: {slide}");
            let (after, slide) = place(variant, 1.0, ROW);
            assert!(after.opacity == 0.0, "{variant:?}: gone");
            assert!(slide < 0.5, "{variant:?}: and the gap closed: {slide}");
        }
    }

    #[test]
    fn the_new_row_is_the_pose_core_gives_at_the_time_the_loop_says() {
        for &variant in InsertMotion::ALL {
            let at =
                OPENS + 0.25 * insert::duration(variant, Phase::Enter).as_secs_f32() / LOOP.as_secs_f32();
            let (pose, _) = place(variant, at, ROW);
            let want = insert::pose(variant, Phase::Enter, 0.25);
            assert!(
                (pose.opacity - want.opacity).abs() < 1e-3 && (pose.clip - want.clip).abs() < 1e-3,
                "{variant:?}"
            );
            assert!((pose.dx - want.dx).abs() < 1e-2, "{variant:?}");
        }
    }
}
