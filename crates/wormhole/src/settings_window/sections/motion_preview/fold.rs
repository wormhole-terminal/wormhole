//! A group folding and opening: the group's head, its two terminals and a terminal under the group,
//! drawn from the poses of the chosen `FoldMotion` (core's `pose::fold`), the same ones the sidebar draws.
//! The stage is as tall as the group open: folded, the rows' room is left empty for as long as it lasts,
//! which is the point of the scene (a stage that shrank with it would move the page under it).

use gpui_kit::*;
use wormhole_core::view::motion::FoldMotion;
use wormhole_core::view::motion::pose::{Phase, Pose, fold, fold::Slot};

use super::{ASKING, IDLE, Spot, WORKING, metrics, progress, row, since};
use crate::settings::{UiColors, settings};
use crate::settings_window::sections::samples::SampleHead;
use crate::style::{SummaryFade, preview_stage};

/// When the group folds and when it opens, as shares of the loop.
const FOLDS: f32 = 0.15;
const OPENS: f32 = 0.6;
/// The rows of the group.
const ROWS: usize = 2;

/// The group at `t` of the loop: how each row stands, how much of the rows' room is open to what is
/// under them (0.0..=1.0), whether the head is folded and, then, how far its summary has faded in.
struct Placed {
    rows: [Pose; ROWS],
    open: f32,
    folded: bool,
    summary: Option<f32>,
}

fn place(variant: FoldMotion, t: f32) -> Placed {
    // Before it folds the rows stand; then each is as its slot says, to the end of the motion it plays.
    let (phase, elapsed) =
        if t < OPENS { (Phase::Exit, since(t, FOLDS)) } else { (Phase::Enter, since(t, OPENS)) };
    let still = t < FOLDS;
    let pose_of = |index: usize| {
        let progress = progress(elapsed, fold::duration(variant, phase, ROWS));
        fold::pose(variant, phase, progress, Slot { index, of: ROWS })
    };
    let folded = (FOLDS..OPENS).contains(&t);
    Placed {
        rows: std::array::from_fn(|index| if still { Pose::REST } else { pose_of(index) }),
        open: if still { 1.0 } else { fold::room_at(variant, phase, ROWS, elapsed) },
        folded,
        summary: folded.then(|| fold::summary_opacity(variant, ROWS, since(t, FOLDS))),
    }
}

pub(super) fn stage(ui: UiColors, t: f32, cx: &App) -> Div {
    let metrics = metrics();
    let (head_h, row_h) = (metrics.head_h, metrics.row_h);
    let Placed { rows: poses, open, folded, summary } = place(settings(cx).motion.fold, t);
    // The head shows its chevron open until the fold starts, and again once it opens; folded it also
    // shows what the group holds, which some folds wait for the rows to be gone to show.
    let head = Spot::row(0.0, head_h).put(|| {
        SampleHead {
            id: format!("motion-fold-head-{folded}").into(),
            name: "api", // text: sample (a made-up group of the preview)
            folder: None,
            color: None,
            count: ROWS,
            folded,
            summary: summary.map(SummaryFade::Opacity),
        }
        .draw(ui, metrics)
    });
    // Each row follows the one above it as far as that is open; the terminal under the group follows them all.
    let mut top = head_h;
    let rows: Vec<_> = [WORKING, ASKING]
        .into_iter()
        .zip(&poses)
        .enumerate()
        .filter_map(|(at, (sample, pose))| {
            let spot = Spot::row(top, row_h).moved(pose.dx, pose.dy).showing(pose.opacity).clipped(pose.clip);
            top += row_h * pose.clip;
            spot.put(|| row(&sample, &format!("motion-fold-{at}"), ui, cx))
        })
        .collect();
    let under =
        Spot::row(head_h + ROWS as f32 * row_h * open, row_h).put(|| row(&IDLE, "motion-fold-under", ui, cx));
    preview_stage(ui, head_h + 3.0 * row_h).children(head).children(rows).children(under)
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::pose::Pose;
    use wormhole_core::view::motion::{FoldMotion, MotionChoice};

    use super::{OPENS, place};

    #[test]
    fn every_variant_starts_open_folds_and_opens_again() {
        for &variant in FoldMotion::ALL {
            let start = place(variant, 0.0);
            assert_eq!(
                (start.rows, start.open, start.folded),
                ([Pose::REST; 2], 1.0, false),
                "{variant:?}: the first picture is the open group"
            );
            let folded = place(variant, OPENS - 0.001);
            assert!(
                folded.folded && folded.open < 0.01,
                "{variant:?}: folded, the rows' room is closed: {}",
                folded.open
            );
            assert!(
                folded.rows.iter().all(|row| row.opacity == 0.0 || row.clip == 0.0),
                "{variant:?}: and the rows are gone"
            );
            assert_eq!(folded.summary, Some(1.0), "{variant:?}: the head shows what it holds");
            let end = place(variant, 1.0);
            assert_eq!((end.rows, end.folded), ([Pose::REST; 2], false), "{variant:?}: open again");
            assert!((end.open - 1.0).abs() < 0.01, "{variant:?}");
        }
    }

    #[test]
    fn the_drawer_waits_for_the_rows_to_be_gone_to_show_the_count() {
        let during = place(FoldMotion::Drawer, 0.15 + 0.025);
        assert_eq!(during.summary, Some(0.0), "the rows are closing: 100 ms of the 160");
        assert_eq!(place(FoldMotion::Classic, 0.15 + 0.01).summary, Some(1.0), "Classic shows it at once");
    }
}
