//! The rows of a group folding under its head and opening again: what each row does, by its place in
//! the group, and when the head shows what the group holds. The stages (milliseconds):
//!
//! | Variant | Folding | Opening |
//! |---|---|---|
//! | Classic | the rows are cut; the lines below slide | the rows fade in, 180, as the lines below slide |
//! | Drawer | the rows' height closes, 160, while they fade; then the head's count fades in, 120 | the reverse: the height opens while the rows fade in |
//! | Tuck | the rows fade and rise 6 pt, 120 each, 30 apart from the bottom; then the height closes, 140 | the reverse: the height opens, then the rows come down into place from the top |
//!
//! Folding is the `Exit` phase and opening the `Enter`: the rows that fold stay in the list for
//! [`duration`] of their exit, drawn as poses, and are dropped when it is over.

use std::time::Duration;

use super::{CLASSIC_FADE, LIST_MOVE, Phase, Pose, Room, millis, progress};
use crate::view::motion::FoldMotion;
use crate::view::motion::ease::{EASE_IN, EASE_IN_OUT, EASE_OUT};

/// Drawer: the rows' height and fade.
const DRAWER: f32 = 160.0;
/// Tuck: each row's fade and rise, the gap between rows and the height closing after them.
const TUCK_ROW: f32 = 120.0;
const TUCK_STAGGER: f32 = 30.0;
const TUCK_HEIGHT: f32 = 140.0;
/// How far a tucking row rises, in points.
const TUCK_RISE: f32 = 6.0;
/// The head's count and tiles fading in once the rows are gone.
const SUMMARY_FADE: f32 = 120.0;

/// A row's place among the rows of its group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// 0 is the group's first (top) row.
    pub index: usize,
    /// How many rows the group has.
    pub of: usize,
}

impl Slot {
    /// How many rows after this one: the one at the bottom has none.
    fn below(self) -> usize {
        self.of.saturating_sub(self.index + 1)
    }
}

/// The time the stagger of `rows` rows runs over, in milliseconds (Tuck).
fn tuck_stagger(rows: usize) -> f32 {
    rows.saturating_sub(1) as f32 * TUCK_STAGGER
}

/// How long the motion of a group of `rows` rows takes: the time the rows take to come in, or stay
/// in the list while they fold.
#[must_use]
pub fn duration(variant: FoldMotion, phase: Phase, rows: usize) -> Duration {
    millis(match (variant, phase) {
        (FoldMotion::Classic, Phase::Enter) => CLASSIC_FADE,
        (FoldMotion::Classic, Phase::Exit) => 0.0,
        (FoldMotion::Drawer, _) => DRAWER,
        (FoldMotion::Tuck, _) => tuck_stagger(rows) + TUCK_ROW + TUCK_HEIGHT,
    })
}

/// The row in `slot` at `t` (0.0..=1.0) of the group's [`duration`]. A fold of no duration (a cut) has
/// every row gone at any time.
#[must_use]
pub fn pose(variant: FoldMotion, phase: Phase, t: f32, slot: Slot) -> Pose {
    let at = t.clamp(0.0, 1.0) * duration(variant, phase, slot.of).as_secs_f32() * 1000.0;
    match (variant, phase) {
        (FoldMotion::Classic, Phase::Enter) => {
            Pose { opacity: EASE_OUT.ease(progress(at, 0.0, CLASSIC_FADE)), ..Pose::REST }
        }
        (FoldMotion::Classic, Phase::Exit) => Pose { opacity: 0.0, clip: 0.0, ..Pose::REST },
        (FoldMotion::Drawer, Phase::Enter) => {
            let open = EASE_IN_OUT.ease(progress(at, 0.0, DRAWER));
            Pose { opacity: open, clip: open, ..Pose::REST }
        }
        (FoldMotion::Drawer, Phase::Exit) => {
            let closed = EASE_IN_OUT.ease(progress(at, 0.0, DRAWER));
            Pose { opacity: 1.0 - closed, clip: 1.0 - closed, ..Pose::REST }
        }
        (FoldMotion::Tuck, Phase::Exit) => {
            // The bottom row goes first, the top one last, and only then does the height close.
            let tucked = EASE_IN.ease(progress(at, slot.below() as f32 * TUCK_STAGGER, TUCK_ROW));
            let closed = EASE_IN_OUT.ease(progress(at, tuck_stagger(slot.of) + TUCK_ROW, TUCK_HEIGHT));
            Pose { opacity: 1.0 - tucked, dy: -TUCK_RISE * tucked, clip: 1.0 - closed, ..Pose::REST }
        }
        (FoldMotion::Tuck, Phase::Enter) => {
            // The same stages backwards: the height opens, then the rows come down from the top.
            let open = EASE_IN_OUT.ease(progress(at, 0.0, TUCK_HEIGHT));
            let placed =
                EASE_OUT.ease(progress(at, TUCK_HEIGHT + slot.index as f32 * TUCK_STAGGER, TUCK_ROW));
            Pose { opacity: placed, dy: -TUCK_RISE * (1.0 - placed), clip: open, ..Pose::REST }
        }
    }
}

/// How much of the head's summary (the tiles and the count a folded group shows) shows `elapsed` after
/// the group started to fold: it waits for the rows to be gone and fades in after them. Classic shows
/// it at once, as it always did.
#[must_use]
pub fn summary_opacity(variant: FoldMotion, rows: usize, elapsed: Duration) -> f32 {
    match variant {
        FoldMotion::Classic => 1.0,
        FoldMotion::Drawer | FoldMotion::Tuck => {
            let gone = duration(variant, Phase::Exit, rows).as_secs_f32() * 1000.0;
            EASE_OUT.ease(progress(elapsed.as_secs_f32() * 1000.0, gone, SUMMARY_FADE))
        }
    }
}

/// How long after a group started to fold its head's summary has finished fading in: the rows' time
/// and the fade after it. No time at all where it shows at once (Classic).
#[must_use]
pub fn summary_duration(variant: FoldMotion, rows: usize) -> Duration {
    match variant {
        FoldMotion::Classic => Duration::ZERO,
        FoldMotion::Drawer | FoldMotion::Tuck => duration(variant, Phase::Exit, rows) + millis(SUMMARY_FADE),
    }
}

/// What makes room for the rows as the group opens, and closes the gap as it folds.
#[must_use]
pub fn room(variant: FoldMotion) -> Room {
    match variant {
        FoldMotion::Classic => Room::Spring(LIST_MOVE),
        FoldMotion::Drawer | FoldMotion::Tuck => Room::Layout,
    }
}

/// How much of the group's height is open to the lines below it, `elapsed` after the motion began.
#[must_use]
pub fn room_at(variant: FoldMotion, phase: Phase, rows: usize, elapsed: Duration) -> f32 {
    let stays = duration(variant, phase, rows);
    let t = if stays.is_zero() { 1.0 } else { elapsed.as_secs_f32() / stays.as_secs_f32() };
    // Every row closes its height together, so any slot says how much is open.
    let clip = pose(variant, phase, t, Slot { index: 0, of: rows.max(1) }).clip;
    room(variant).open_at(phase, stays, elapsed, clip)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        FoldMotion, Phase, Pose, Slot, duration, pose, room, room_at, summary_duration, summary_opacity,
    };
    use crate::view::motion::MotionChoice;
    use crate::view::motion::ease::EASE_OUT;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn slot(index: usize, of: usize) -> Slot {
        Slot { index, of }
    }

    fn at(variant: FoldMotion, phase: Phase, ms: f32, slot: Slot) -> Pose {
        pose(variant, phase, ms / (duration(variant, phase, slot.of).as_secs_f32() * 1000.0), slot)
    }

    #[test]
    fn rows_open_from_nothing_to_rest_and_fold_from_rest_to_nothing() {
        for &variant in FoldMotion::ALL {
            for of in 1..=4 {
                for index in 0..of {
                    let slot = slot(index, of);
                    let start = pose(variant, Phase::Enter, 0.0, slot);
                    assert!(start.opacity == 0.0 || start.clip == 0.0, "{variant:?} {slot:?}: {start:?}");
                    assert_eq!(pose(variant, Phase::Enter, 1.0, slot), Pose::REST, "{variant:?} {slot:?}");
                    if !duration(variant, Phase::Exit, of).is_zero() {
                        assert_eq!(pose(variant, Phase::Exit, 0.0, slot), Pose::REST, "{variant:?} {slot:?}");
                    }
                    let end = pose(variant, Phase::Exit, 1.0, slot);
                    assert!(end.opacity == 0.0 || end.clip == 0.0, "{variant:?} {slot:?}: {end:?}");
                }
            }
        }
    }

    #[test]
    fn classic_cuts_the_rows_that_fold_and_fades_in_the_rows_that_open() {
        let classic = FoldMotion::Classic;
        assert!(duration(classic, Phase::Exit, 3).is_zero());
        assert_eq!(duration(classic, Phase::Enter, 3), ms(180));
        let opened = pose(classic, Phase::Enter, 0.25, slot(1, 3));
        assert_eq!((opened.opacity, opened.clip, opened.dy), (EASE_OUT.ease(0.25), 1.0, 0.0));
        assert_eq!(summary_opacity(classic, 3, Duration::ZERO), 1.0, "the head shows its tiles at once");
    }

    #[test]
    fn the_drawer_closes_every_rows_height_while_they_fade_and_then_shows_the_count() {
        let drawer = FoldMotion::Drawer;
        assert_eq!(duration(drawer, Phase::Exit, 5), ms(160));
        let half = at(drawer, Phase::Exit, 80.0, slot(0, 2));
        assert!(half.clip > 0.1 && half.clip < 0.4, "{half:?}: half way through, a good part has closed");
        assert_eq!(half.opacity, half.clip, "the rows fade as the height closes");
        assert_eq!(at(drawer, Phase::Exit, 80.0, slot(1, 2)), half, "all rows close together");
        assert_eq!(summary_opacity(drawer, 2, ms(100)), 0.0, "not while the rows are there");
        assert_eq!(summary_opacity(drawer, 2, ms(160)), 0.0);
        let coming = summary_opacity(drawer, 2, ms(220));
        assert!(coming > 0.0 && coming < 1.0, "{coming}");
        assert_eq!(summary_opacity(drawer, 2, ms(280)), 1.0, "after the rows, over 120 ms");
        assert_eq!(summary_duration(drawer, 2), ms(280));
        assert!(summary_duration(FoldMotion::Classic, 2).is_zero());
        assert_eq!(room(drawer), super::Room::Layout);
    }

    #[test]
    fn tuck_takes_the_bottom_row_first_30_ms_apart_and_closes_the_height_after() {
        let tuck = FoldMotion::Tuck;
        // Three rows: 2 x 30 apart + 120 each + 140 for the height.
        assert_eq!(duration(tuck, Phase::Exit, 3), ms(320));
        assert_eq!(duration(tuck, Phase::Exit, 1), ms(260));
        let (top, middle, bottom) = (slot(0, 3), slot(1, 3), slot(2, 3));
        assert!(at(tuck, Phase::Exit, 30.0, bottom).opacity < 1.0, "the bottom row has started");
        assert_eq!(at(tuck, Phase::Exit, 30.0, middle), Pose::REST, "the one above has not");
        assert_eq!(at(tuck, Phase::Exit, 30.0, top), Pose::REST);
        assert!(at(tuck, Phase::Exit, 60.0, middle).opacity < 1.0);
        assert_eq!(at(tuck, Phase::Exit, 60.0, top), Pose::REST);
        // Each rises 6 pt as it fades and none moves before its time.
        let done = at(tuck, Phase::Exit, 120.0, bottom);
        assert_eq!((done.opacity, done.dy, done.clip), (0.0, -6.0, 1.0));
        // The height closes only after the top row is gone (60 + 120 = 180).
        assert_eq!(at(tuck, Phase::Exit, 179.0, top).clip, 1.0);
        let closing = at(tuck, Phase::Exit, 250.0, middle);
        assert!(closing.clip > 0.0 && closing.clip < 1.0 && closing.opacity == 0.0, "{closing:?}");
        assert_eq!(room(tuck), super::Room::Layout);
    }

    #[test]
    fn tuck_opens_as_it_folds_played_backwards() {
        let tuck = FoldMotion::Tuck;
        let (top, bottom) = (slot(0, 3), slot(2, 3));
        // The height opens first, with nothing in it.
        let opening = at(tuck, Phase::Enter, 70.0, top);
        assert!(opening.clip > 0.0 && opening.clip < 1.0 && opening.opacity == 0.0, "{opening:?}");
        assert_eq!(at(tuck, Phase::Enter, 140.0, top).clip, 1.0);
        // Then the rows come down from the top, one after another.
        assert!(at(tuck, Phase::Enter, 200.0, top).opacity > 0.0);
        assert_eq!(at(tuck, Phase::Enter, 170.0, bottom).opacity, 0.0, "the bottom row waits 60 ms more");
        let placed = at(tuck, Phase::Enter, 200.0, top);
        assert!(placed.dy < 0.0, "it is still above its place, coming down: {placed:?}");
    }

    #[test]
    fn the_group_makes_room_as_far_as_its_height_is_open() {
        assert_eq!(room_at(FoldMotion::Drawer, Phase::Exit, 2, ms(0)), 1.0);
        assert_eq!(room_at(FoldMotion::Drawer, Phase::Exit, 2, ms(160)), 0.0);
        assert_eq!(room_at(FoldMotion::Tuck, Phase::Exit, 3, ms(100)), 1.0, "still while the rows tuck");
        assert!(room_at(FoldMotion::Tuck, Phase::Exit, 3, ms(250)) < 1.0);
        assert_eq!(room_at(FoldMotion::Classic, Phase::Exit, 2, ms(0)), 1.0, "a spring starts after the cut");
        assert!(room_at(FoldMotion::Classic, Phase::Exit, 2, ms(2000)) < 0.001);
    }
}
