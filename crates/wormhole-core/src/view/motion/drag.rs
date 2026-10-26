//! The two motions of dragging a terminal in the list, which the design fixed (the picks of the motion
//! plan: no choice of variant, so no `MotionChoices` field): rows make room for a row that is moved,
//! and a row dropped on the middle of another snaps onto it and previews the split. The numbers are
//! here so a test can pin them and the app only draws them.
//!
//! | Part | Motion |
//! |---|---|
//! | The row lifts | the row's own picture comes in at the pointer with a shadow, 120 |
//! | The rows it passes | step aside on the quick spring (400/30) to open the gap, and back when it moves on |
//! | It drops | the rows and the row settle on the quick spring |
//! | The picture over the middle of a row | crossfades, 120, to a compact chip that snaps below that row's name on the quick spring, and back the same way |
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

/// The dragged row's picture coming in (and crossfading to the chip and back), and the target's tint and
/// split mark coming in.
pub const LIFT: Duration = Duration::from_millis(120);
/// The target's tint and split mark going out again.
pub const FADE_OUT: Duration = Duration::from_millis(160);
/// A split's bracket drawing down its rows once the drop has made it.
pub const BRACKET: Duration = Duration::from_millis(160);

/// How much of the compact chip shows `t` (0.0..=1.0) of the way through [`LIFT`] after the picture of a
/// dragged row crossfaded: coming `onto` a row's middle the chip fades in, leaving it the chip fades out,
/// easing out. The row's own picture shows the rest (1.0 minus this).
#[must_use]
pub fn chip_share(onto: bool, t: f32) -> f32 {
    let eased = EASE_OUT.ease(t.clamp(0.0, 1.0));
    if onto { eased } else { 1.0 - eased }
}

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
    fn the_picture_crossfades_to_the_chip_onto_a_row_and_back_off_it() {
        assert_eq!((chip_share(true, 0.0), chip_share(true, 1.0)), (0.0, 1.0));
        assert_eq!((chip_share(false, 0.0), chip_share(false, 1.0)), (1.0, 0.0));
        for t in [-1.0, 0.2, 0.5, 0.9, 3.0] {
            let (on, off) = (chip_share(true, t), chip_share(false, t));
            assert!((0.0..=1.0).contains(&on) && (0.0..=1.0).contains(&off), "{t}");
            assert!((on + off - 1.0).abs() < 1e-6, "{t}: going on and coming off mirror each other");
        }
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
