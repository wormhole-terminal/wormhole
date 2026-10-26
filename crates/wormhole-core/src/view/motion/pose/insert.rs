//! A terminal opening in the list and closing in it, one function per thing a variant decides. The
//! stages (milliseconds, from the start of the motion):
//!
//! | Variant | Opening | Closing |
//! |---|---|---|
//! | Classic | the row fades in, 180, as the rows below slide aside | it is cut; the rows below slide |
//! | Reveal | the row is revealed top to bottom with a fade, 160, the rows below moving with it | it fades, 100; then the gap closes, 160 |
//! | From the side | the gap opens, 160; then the row slides in 12 pt from the left with a fade, 180 | it slides left 12 pt and fades, 120; then the gap closes, 160 |
//! | Soft spring | the row fades in, 120, as the rows around move on the soft spring | it fades, 100; then the rows spring |
//!
//! A row that goes stays in the list for [`duration`] of its closing, drawn as a pose, and is dropped
//! when it is over (for Classic that is no time at all: it is cut).

use std::time::Duration;

use super::{CLASSIC_FADE, LIST_MOVE, Phase, Pose, Room, millis, progress};
use crate::view::motion::ease::{EASE_IN, EASE_IN_OUT, EASE_OUT};
use crate::view::motion::{InsertMotion, SpringSpec};

/// The rows below moving with a row that opens its height.
const REVEAL: f32 = 160.0;
/// A fade on its way out.
const FADE_OUT: f32 = 100.0;
/// The gap a row opens or closes before or after it slides.
const GAP: f32 = 160.0;
/// The row sliding in from the side, and out to it.
const SLIDE_IN: f32 = 180.0;
const SLIDE_OUT: f32 = 120.0;
/// How far from its place the row stands at the side of its slide, in points.
const SLIDE: f32 = 12.0;
/// The fade of a row that appears among rows that move on a spring.
const SPRING_FADE: f32 = 120.0;

/// How long the row's own motion takes: the time it takes to come in, or the time it stays in the
/// list while it goes.
#[must_use]
pub fn duration(variant: InsertMotion, phase: Phase) -> Duration {
    millis(match (variant, phase) {
        (InsertMotion::Classic, Phase::Enter) => CLASSIC_FADE,
        (InsertMotion::Classic, Phase::Exit) => 0.0,
        (InsertMotion::Reveal, Phase::Enter) => REVEAL,
        (InsertMotion::Reveal, Phase::Exit) => FADE_OUT + GAP,
        (InsertMotion::FromSide, Phase::Enter) => GAP + SLIDE_IN,
        (InsertMotion::FromSide, Phase::Exit) => SLIDE_OUT + GAP,
        (InsertMotion::SoftSpring, Phase::Enter) => SPRING_FADE,
        (InsertMotion::SoftSpring, Phase::Exit) => FADE_OUT,
    })
}

/// The row at `t` (0.0..=1.0) of its [`duration`]. An exit of no duration (a cut) is gone at any time.
#[must_use]
pub fn pose(variant: InsertMotion, phase: Phase, t: f32) -> Pose {
    let at = t.clamp(0.0, 1.0) * duration(variant, phase).as_secs_f32() * 1000.0;
    match (variant, phase) {
        (InsertMotion::Classic, Phase::Enter) => {
            Pose { opacity: EASE_OUT.ease(progress(at, 0.0, CLASSIC_FADE)), ..Pose::REST }
        }
        (InsertMotion::Classic, Phase::Exit) => Pose { opacity: 0.0, clip: 0.0, ..Pose::REST },
        (InsertMotion::Reveal, Phase::Enter) => {
            let open = EASE_IN_OUT.ease(progress(at, 0.0, REVEAL));
            Pose { opacity: open, clip: open, ..Pose::REST }
        }
        (InsertMotion::Reveal, Phase::Exit) => {
            let faded = EASE_IN.ease(progress(at, 0.0, FADE_OUT));
            let closed = EASE_IN_OUT.ease(progress(at, FADE_OUT, GAP));
            Pose { opacity: 1.0 - faded, clip: 1.0 - closed, ..Pose::REST }
        }
        (InsertMotion::FromSide, Phase::Enter) => {
            let open = EASE_OUT.ease(progress(at, 0.0, GAP));
            let slid = EASE_OUT.ease(progress(at, GAP, SLIDE_IN));
            Pose { opacity: slid, dx: -SLIDE * (1.0 - slid), clip: open, ..Pose::REST }
        }
        (InsertMotion::FromSide, Phase::Exit) => {
            let slid = EASE_IN.ease(progress(at, 0.0, SLIDE_OUT));
            let closed = EASE_OUT.ease(progress(at, SLIDE_OUT, GAP));
            Pose { opacity: 1.0 - slid, dx: -SLIDE * slid, clip: 1.0 - closed, ..Pose::REST }
        }
        (InsertMotion::SoftSpring, Phase::Enter) => {
            Pose { opacity: EASE_OUT.ease(progress(at, 0.0, SPRING_FADE)), ..Pose::REST }
        }
        (InsertMotion::SoftSpring, Phase::Exit) => {
            Pose { opacity: 1.0 - EASE_IN.ease(progress(at, 0.0, FADE_OUT)), ..Pose::REST }
        }
    }
}

/// What makes room for the row, and what closes the gap when it is gone.
#[must_use]
pub fn room(variant: InsertMotion) -> Room {
    match variant {
        InsertMotion::Classic => Room::Spring(LIST_MOVE),
        InsertMotion::Reveal | InsertMotion::FromSide => Room::Layout,
        InsertMotion::SoftSpring => Room::Spring(SpringSpec::SOFT),
    }
}

/// How much of the row's height is open to the rows around, `elapsed` after the motion began.
#[must_use]
pub fn room_at(variant: InsertMotion, phase: Phase, elapsed: Duration) -> f32 {
    let stays = duration(variant, phase);
    let t = if stays.is_zero() { 1.0 } else { elapsed.as_secs_f32() / stays.as_secs_f32() };
    room(variant).open_at(phase, stays, elapsed, pose(variant, phase, t).clip)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{InsertMotion, Phase, Pose, Room, duration, pose, room, room_at};
    use crate::view::motion::ease::EASE_OUT;
    use crate::view::motion::{MotionChoice, SpringSpec};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn at(variant: InsertMotion, phase: Phase, ms: f32) -> Pose {
        pose(variant, phase, ms / (duration(variant, phase).as_secs_f32() * 1000.0))
    }

    #[test]
    fn a_row_comes_in_from_nothing_to_rest_and_goes_from_rest_to_nothing() {
        for &variant in InsertMotion::ALL {
            let start = pose(variant, Phase::Enter, 0.0);
            assert!(start.opacity == 0.0 || start.clip == 0.0, "{variant:?} shows at its first moment");
            assert_eq!(pose(variant, Phase::Enter, 1.0), Pose::REST, "{variant:?}");
            if !duration(variant, Phase::Exit).is_zero() {
                assert_eq!(pose(variant, Phase::Exit, 0.0), Pose::REST, "{variant:?} moves at once");
            }
            let end = pose(variant, Phase::Exit, 1.0);
            assert!(end.opacity == 0.0 && (end.clip == 0.0 || end.clip == 1.0), "{variant:?} stays: {end:?}");
        }
    }

    #[test]
    fn classic_is_what_the_list_did_before_a_fade_of_180_ms_easing_out_and_a_cut() {
        let classic = InsertMotion::Classic;
        assert_eq!(duration(classic, Phase::Enter), ms(180));
        assert!(duration(classic, Phase::Exit).is_zero());
        for t in [0.1, 0.25, 0.5, 0.9] {
            let opened = pose(classic, Phase::Enter, t);
            assert!((opened.opacity - EASE_OUT.ease(t)).abs() < 1e-4, "{t}: {opened:?}");
            assert_eq!((opened.dx, opened.dy, opened.clip), (0.0, 0.0, 1.0), "it only fades");
        }
        assert_eq!(
            room(classic),
            Room::Spring(super::LIST_MOVE),
            "the rows below slide on the list's spring"
        );
    }

    #[test]
    fn reveal_opens_its_height_with_a_fade_and_closes_it_after_the_fade() {
        let reveal = InsertMotion::Reveal;
        assert_eq!((duration(reveal, Phase::Enter), duration(reveal, Phase::Exit)), (ms(160), ms(260)));
        let half = at(reveal, Phase::Enter, 80.0);
        // The curve starts a little slowly and arrives softly: well on the way at half the time.
        assert!(half.clip > 0.6 && half.clip < 0.9, "{half:?}");
        assert_eq!(half.opacity, half.clip, "the fade goes with the reveal");
        // Closing: the fade first (the whole height stays), then the gap.
        assert_eq!(at(reveal, Phase::Exit, 100.0).clip, 1.0);
        assert_eq!(at(reveal, Phase::Exit, 100.0).opacity, 0.0);
        let closing = at(reveal, Phase::Exit, 180.0);
        assert!(closing.clip > 0.0 && closing.clip < 1.0, "{closing:?}");
        assert_eq!(room(reveal), Room::Layout);
    }

    #[test]
    fn from_the_side_opens_the_gap_and_then_slides_the_row_in() {
        let from_side = InsertMotion::FromSide;
        assert_eq!((duration(from_side, Phase::Enter), duration(from_side, Phase::Exit)), (ms(340), ms(280)));
        // During the gap the row is not there yet, 12 pt to the left of its place.
        let gap = at(from_side, Phase::Enter, 80.0);
        assert_eq!((gap.opacity, gap.dx), (0.0, -12.0));
        assert!(gap.clip > 0.5, "ease-out: most of it is open at half the time: {gap:?}");
        assert_eq!(at(from_side, Phase::Enter, 160.0).clip, 1.0);
        // Then it slides in with its fade.
        let sliding = at(from_side, Phase::Enter, 250.0);
        assert!(sliding.opacity > 0.0 && sliding.opacity < 1.0 && sliding.dx < 0.0 && sliding.dx > -12.0);
        assert_eq!(sliding.clip, 1.0, "the gap stays open");
        // Closing: slides left and fades first, keeping its height, then the gap closes.
        let leaving = at(from_side, Phase::Exit, 60.0);
        assert!(leaving.dx < 0.0 && leaving.opacity < 1.0 && leaving.clip == 1.0, "{leaving:?}");
        let end_of_slide = at(from_side, Phase::Exit, 120.0);
        assert_eq!((end_of_slide.opacity, end_of_slide.dx, end_of_slide.clip), (0.0, -12.0, 1.0));
        let closing = at(from_side, Phase::Exit, 200.0);
        assert!(closing.clip > 0.0 && closing.clip < 1.0 && closing.opacity == 0.0, "{closing:?}");
    }

    #[test]
    fn soft_spring_fades_the_row_and_leaves_the_gap_to_the_soft_spring() {
        let soft = InsertMotion::SoftSpring;
        assert_eq!((duration(soft, Phase::Enter), duration(soft, Phase::Exit)), (ms(120), ms(100)));
        assert_eq!(room(soft), Room::Spring(SpringSpec::SOFT));
        // The row keeps its whole height; only the rows around move.
        for t in [0.0, 0.5, 1.0] {
            assert_eq!(pose(soft, Phase::Enter, t).clip, 1.0);
            assert_eq!(pose(soft, Phase::Exit, t).clip, 1.0);
        }
        assert_eq!(room_at(soft, Phase::Enter, Duration::ZERO), 0.0);
        assert_eq!(room_at(soft, Phase::Exit, ms(100)), 1.0, "the gap stays while the row fades");
        assert!(room_at(soft, Phase::Exit, ms(900)) < 0.01);
    }

    #[test]
    fn a_layout_motion_makes_room_as_far_as_its_height_is_open() {
        let reveal = InsertMotion::Reveal;
        for elapsed in [0, 40, 80, 120, 160] {
            assert_eq!(
                room_at(reveal, Phase::Enter, ms(elapsed)),
                at(reveal, Phase::Enter, elapsed as f32).clip
            );
        }
        assert_eq!(room_at(reveal, Phase::Exit, ms(0)), 1.0);
        assert_eq!(room_at(reveal, Phase::Exit, ms(260)), 0.0);
    }

    #[test]
    fn a_pose_outside_the_time_holds_the_end() {
        for &variant in InsertMotion::ALL {
            assert_eq!(pose(variant, Phase::Enter, 7.0), pose(variant, Phase::Enter, 1.0));
            assert_eq!(pose(variant, Phase::Exit, -1.0), pose(variant, Phase::Exit, 0.0));
        }
    }
}
