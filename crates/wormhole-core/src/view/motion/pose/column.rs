//! The sidebar or the right panel hiding and showing, one function of the variant, which way the column
//! goes and how far into it the motion is. A column's motion is two numbers: how much of it is in
//! ([`ColumnPose::shown`], what the clip, the terminal that moves with it and the title row follow) and how
//! much of what is in it shows ([`ColumnPose::content`]). The stages (milliseconds):
//!
//! | Variant | Showing | Hiding |
//! |---|---|---|
//! | Classic | the clip opens, 250, easing out | the clip closes, 200, easing in |
//! | Soft spring | the clip opens on the soft spring (170/26) | the clip closes on it |
//! | Content after | the clip opens, 250, easing out, and the content fades in, 150, from 100 | the content fades out, 80; then the clip closes, 200, easing in |
//!
//! Which part of the layout moves when is not decided here, and is the same for every variant: a column
//! that comes in lies over the terminal and joins the layout when it is in (the terminal resizes once, at
//! the end), and one that goes leaves the layout at once and is covered up as it goes. Nothing here may be
//! sampled to resize a terminal: the terminal is moved or cut, never sized, until the column has landed.
//!
//! A motion that is begun while another is under way starts from where the column is (`from`), so a second
//! toggle turns it round instead of jumping. Classic is today's motion and the app starts it from rest
//! every time, as it always did.

use std::time::Duration;

use super::status::SETTLED;
use super::{millis, progress};
use crate::view::motion::ease::{EASE_IN, EASE_OUT};
use crate::view::motion::{ColumnMotion, SpringSpec};

/// A column coming in under Classic and Content after, and going out under Classic.
const IN: f32 = 250.0;
const OUT: f32 = 200.0;
/// Content after: the content's fade out before the clip closes, and its fade in, which starts a little
/// after the clip does.
const FADE_OUT: f32 = 80.0;
const FADE_IN: f32 = 150.0;
const FADE_IN_AFTER: f32 = 100.0;
/// The spring the soft variant moves the clip on.
const SOFT: SpringSpec = SpringSpec::SOFT;

/// How long a column takes to come in, under Classic: what the app's `MOTION_PANEL` is.
pub const CLASSIC_IN: Duration = Duration::from_millis(IN as u64);
/// And to go out: `MOTION_PANEL_EXIT`.
pub const CLASSIC_OUT: Duration = Duration::from_millis(OUT as u64);

/// How a column stands at a moment of its motion. At rest it is [`ColumnPose::IN`] or [`ColumnPose::OUT`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnPose {
    /// How much of the column is in: 0.0 none of it, 1.0 all of its width.
    pub shown: f32,
    /// How much of the column's content shows through (0.0 nothing, 1.0 all of it).
    pub content: f32,
}

impl ColumnPose {
    /// A column that is in.
    pub const IN: Self = Self { shown: 1.0, content: 1.0 };
    /// A column that is out.
    pub const OUT: Self = Self { shown: 0.0, content: 1.0 };

    /// The rest of a column that is `opening` or not, once its motion is over.
    #[must_use]
    pub fn rest(opening: bool) -> Self {
        if opening { Self::IN } else { Self::OUT }
    }
}

/// How long the column's motion takes, `opening` it or not.
#[must_use]
pub fn duration(variant: ColumnMotion, opening: bool) -> Duration {
    match (variant, opening) {
        (ColumnMotion::Classic, true) | (ColumnMotion::ContentAfter, true) => CLASSIC_IN,
        (ColumnMotion::Classic, false) => CLASSIC_OUT,
        (ColumnMotion::SoftSpring, _) => SOFT.settle(SETTLED),
        (ColumnMotion::ContentAfter, false) => millis(FADE_OUT + OUT),
    }
}

/// The column `elapsed` after its motion began, `opening` or not, from `from` of it being in (0.0..=1.0:
/// the rest it left, or where it was when another motion turned round). Over, it is exactly at rest.
#[must_use]
pub fn pose(variant: ColumnMotion, opening: bool, from: f32, elapsed: Duration) -> ColumnPose {
    if elapsed >= duration(variant, opening) {
        return ColumnPose::rest(opening);
    }
    let at = elapsed.as_secs_f32() * 1000.0;
    let target = if opening { 1.0 } else { 0.0 };
    let travel = |share: f32| from + (target - from) * share;
    match (variant, opening) {
        (ColumnMotion::Classic, true) => {
            ColumnPose { shown: travel(EASE_OUT.ease(progress(at, 0.0, IN))), content: 1.0 }
        }
        (ColumnMotion::Classic, false) => {
            ColumnPose { shown: travel(EASE_IN.ease(progress(at, 0.0, OUT))), content: 1.0 }
        }
        (ColumnMotion::SoftSpring, _) => {
            ColumnPose { shown: travel(SOFT.step(elapsed.as_secs_f32())), content: 1.0 }
        }
        (ColumnMotion::ContentAfter, true) => ColumnPose {
            shown: travel(EASE_OUT.ease(progress(at, 0.0, IN))),
            content: EASE_OUT.ease(progress(at, FADE_IN_AFTER, FADE_IN)),
        },
        (ColumnMotion::ContentAfter, false) => ColumnPose {
            shown: travel(EASE_IN.ease(progress(at, FADE_OUT, OUT))),
            content: 1.0 - progress(at, 0.0, FADE_OUT),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::motion::MotionChoice;

    fn at(variant: ColumnMotion, opening: bool, from: f32, ms: u64) -> ColumnPose {
        pose(variant, opening, from, Duration::from_millis(ms))
    }

    fn rest(opening: bool) -> f32 {
        f32::from(opening)
    }

    #[test]
    fn the_durations_are_the_ones_the_design_picked() {
        let ms = |variant, opening| duration(variant, opening).as_millis();
        assert_eq!((ms(ColumnMotion::Classic, true), ms(ColumnMotion::Classic, false)), (250, 200));
        assert_eq!(CLASSIC_IN, Duration::from_millis(250));
        assert_eq!(CLASSIC_OUT, Duration::from_millis(200));
        assert_eq!(
            (ms(ColumnMotion::ContentAfter, true), ms(ColumnMotion::ContentAfter, false)),
            (250, 280),
            "in: the clip and the fade overlap; out: the fade, then the clip"
        );
        let soft = duration(ColumnMotion::SoftSpring, true);
        assert_eq!(soft, duration(ColumnMotion::SoftSpring, false), "the spring is the same both ways");
        assert!((0.3..0.9).contains(&soft.as_secs_f32()), "{soft:?}");
    }

    #[test]
    fn every_variant_starts_at_the_old_rest_and_ends_at_the_new_one() {
        for &variant in ColumnMotion::ALL {
            for opening in [true, false] {
                let (start, end) = (rest(!opening), rest(opening));
                let first = at(variant, opening, start, 0);
                assert!((first.shown - start).abs() < 1e-6, "{variant:?} {opening}: {first:?}");
                let last = pose(variant, opening, start, duration(variant, opening));
                assert_eq!(last, ColumnPose::rest(opening), "{variant:?} {opening}");
                assert_eq!(last.shown, end);
                let long_after = pose(variant, opening, start, Duration::from_secs(9));
                assert_eq!(long_after, ColumnPose::rest(opening), "{variant:?} {opening}: it stays at rest");
            }
        }
    }

    #[test]
    fn the_column_only_ever_travels_toward_where_it_is_going_and_stays_inside_its_width() {
        for &variant in ColumnMotion::ALL {
            for opening in [true, false] {
                let (mut before, target) = (rest(!opening), rest(opening));
                for ms in (0..=700).step_by(10) {
                    let shown = at(variant, opening, rest(!opening), ms).shown;
                    assert!((0.0..=1.0).contains(&shown), "{variant:?} {opening} at {ms}: {shown}");
                    assert!(
                        (target - shown).abs() <= (target - before).abs() + 1e-4,
                        "{variant:?} {opening} at {ms}: {shown} after {before}"
                    );
                    before = shown;
                }
            }
        }
    }

    #[test]
    fn classic_is_the_timed_curves_it_always_was_and_its_content_never_fades() {
        let (mid_in, mid_out) =
            (at(ColumnMotion::Classic, true, 0.0, 125), at(ColumnMotion::Classic, false, 1.0, 100));
        assert!((mid_in.shown - EASE_OUT.ease(0.5)).abs() < 1e-5);
        assert!((mid_out.shown - (1.0 - EASE_IN.ease(0.5))).abs() < 1e-5);
        assert!(mid_in.shown > 0.5, "coming in eases out");
        assert!(mid_out.shown > 0.5, "going out eases in: it is slow to start");
        assert_eq!((mid_in.content, mid_out.content), (1.0, 1.0));
    }

    #[test]
    fn the_soft_spring_is_the_soft_spring_and_never_passes_its_rest() {
        let ms = 120;
        let shown = at(ColumnMotion::SoftSpring, true, 0.0, ms).shown;
        assert!((shown - SpringSpec::SOFT.step(0.120)).abs() < 1e-6);
        let going = at(ColumnMotion::SoftSpring, false, 1.0, ms).shown;
        assert!((going - (1.0 - SpringSpec::SOFT.step(0.120))).abs() < 1e-6, "out is the same spring, back");
        for ms in 0..700 {
            assert!(at(ColumnMotion::SoftSpring, true, 0.0, ms).shown <= 1.0);
            assert!(at(ColumnMotion::SoftSpring, false, 1.0, ms).shown >= 0.0);
        }
    }

    #[test]
    fn content_after_fades_the_content_in_once_the_column_is_on_its_way() {
        let early = at(ColumnMotion::ContentAfter, true, 0.0, 90);
        assert_eq!(early.content, 0.0, "nothing shows for the first 100");
        assert!(early.shown > 0.0, "while the column already opens");
        let late = at(ColumnMotion::ContentAfter, true, 0.0, 200);
        assert!(late.content > 0.0 && late.content < 1.0, "{late:?}");
        assert_eq!(at(ColumnMotion::ContentAfter, true, 0.0, 249).content.round(), 1.0);
    }

    #[test]
    fn content_after_takes_the_content_away_first_and_then_the_column() {
        let fading = at(ColumnMotion::ContentAfter, false, 1.0, 40);
        assert!(fading.content > 0.0 && fading.content < 1.0, "{fading:?}");
        assert_eq!(fading.shown, 1.0, "the column has not moved yet");
        let gone = at(ColumnMotion::ContentAfter, false, 1.0, 80);
        assert_eq!((gone.content, gone.shown), (0.0, 1.0));
        let closing = at(ColumnMotion::ContentAfter, false, 1.0, 180);
        assert_eq!(closing.content, 0.0, "the content stays gone");
        assert!(closing.shown < 1.0 && closing.shown > 0.0, "{closing:?}");
    }

    #[test]
    fn a_motion_that_turns_round_starts_from_where_the_column_is() {
        // Hiding a column that was two fifths in does not jump to all of it first.
        for variant in [ColumnMotion::SoftSpring, ColumnMotion::ContentAfter] {
            let begun = pose(variant, false, 0.4, Duration::ZERO);
            assert!((begun.shown - 0.4).abs() < 1e-6, "{variant:?}: {begun:?}");
        }
        // And one that was nearly in, coming in again, stays near the end.
        let nearly = pose(ColumnMotion::SoftSpring, true, 0.9, Duration::from_millis(40));
        assert!(nearly.shown > 0.9 && nearly.shown <= 1.0, "{nearly:?}");
    }
}
