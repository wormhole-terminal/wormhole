//! The two motions of dragging a terminal in the list, which the design fixed (the picks of the motion
//! plan: no choice of variant, so no `MotionChoices` field): rows make room for a row that is moved,
//! and a row dropped on the middle of another snaps onto it and previews the split. The numbers are
//! here so a test can pin them and the app only draws them.
//!
//! | Part | Motion |
//! |---|---|
//! | The row lifts | its picture gets a shadow, 120 |
//! | The rows it passes | step aside on the quick spring (400/30) to open the gap, and back when it moves on |
//! | It drops | the rows and the row settle on the quick spring |
//! | The chip over the middle of a row | snaps onto it on the quick spring and comes off it the same way |
//! | That row | tints and shows a split mark, 120; both fade out, 160, when the chip leaves or lets go |
//! | The split's bracket | draws down the two rows, 160 |
//! | The rows after a join | settle on the soft spring (170/26) |

use std::time::Duration;

use super::SpringSpec;
use super::ease::EASE_OUT;

/// The rows stepping aside, and back: quick, a hint of overshoot, and the same spring the list moves
/// on, so a row that was pushed aside and a row that is dropped carry on from where they are.
pub const ROOM: SpringSpec = SpringSpec::SNAP;
/// The chip snapping onto a row and coming off it.
pub const SNAP: SpringSpec = SpringSpec::SNAP;
/// The rows settling after a drop that made a split.
pub const JOIN_SETTLE: SpringSpec = SpringSpec::SOFT;

/// The lift of the dragged row's picture, and the target's tint and split mark coming in.
pub const LIFT: Duration = Duration::from_millis(120);
/// The target's tint and split mark going out again, the bracket drawing in.
pub const FADE_OUT: Duration = Duration::from_millis(160);
pub const BRACKET: Duration = Duration::from_millis(160);

/// How much of a split's bracket line shows `t` (0.0..=1.0) of the way through [`BRACKET`], for the
/// stretch of the line that is drawn from `from` to `to` of that time: the bracket draws down the
/// rows, so the first row's stretch (0.0 to 0.5) is there before the last one's (0.5 to 1.0). It
/// fades in as it is drawn, easing out.
#[must_use]
pub fn bracket(t: f32, from: f32, to: f32) -> f32 {
    let drawn = if to > from { ((t - from) / (to - from)).clamp(0.0, 1.0) } else { f32::from(t >= from) };
    EASE_OUT.ease(drawn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picks_are_the_numbers_the_design_names() {
        assert_eq!((ROOM, SNAP), (SpringSpec::SNAP, SpringSpec::SNAP), "400/30");
        assert_eq!(JOIN_SETTLE, SpringSpec::SOFT, "170/26");
        assert_eq!((LIFT.as_millis(), FADE_OUT.as_millis(), BRACKET.as_millis()), (120, 160, 160));
    }

    #[test]
    fn a_bracket_is_drawn_down_its_rows_in_the_time_it_takes() {
        // The first stretch is there at half the time, the last one only starts then.
        assert_eq!(bracket(0.0, 0.0, 0.5), 0.0);
        assert!(bracket(0.25, 0.0, 0.5) > 0.0 && bracket(0.25, 0.0, 0.5) < 1.0);
        assert_eq!(bracket(0.5, 0.0, 0.5), 1.0);
        assert_eq!(bracket(0.5, 0.5, 1.0), 0.0);
        assert_eq!(bracket(1.0, 0.5, 1.0), 1.0);
        // Never outside 0..=1, at any time, and a stretch of no time is a cut.
        for t in [-1.0, 0.0, 0.3, 0.9, 1.0, 2.0] {
            assert!((0.0..=1.0).contains(&bracket(t, 0.25, 0.75)));
        }
        assert_eq!((bracket(0.4, 0.5, 0.5), bracket(0.5, 0.5, 0.5)), (0.0, 1.0));
    }
}
