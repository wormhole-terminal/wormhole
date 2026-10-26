//! The notice chip's motion, which the design fixed (the picks of the motion plan, "Notice chip": no
//! choice of variant, so no `MotionChoices` field). The chip drops from under the title row on a spring,
//! rests a little below it, and goes back up; with Reduce Motion it fades in place instead. A native panel
//! plays all of it (`platform::notice`), so these are the numbers it is handed, not a pose to draw.

use std::time::Duration;

use super::ease::{CubicBezier, EASE_IN, EASE_OUT};
use super::spring::SpringSpec;

/// The drop on its way in.
pub const DROP: SpringSpec = SpringSpec::SNAP;
/// Where the chip rests: this far under the title row's lower edge (points).
pub const REST_GAP: f32 = 8.0;
/// Going back up.
pub const UP: Duration = Duration::from_millis(180);
/// What the way up eases along: a thing that leaves speeds up.
pub const UP_EASE: CubicBezier = EASE_IN;
/// Reduce Motion: the chip fades in place.
pub const FADE_IN: Duration = Duration::from_millis(200);
pub const FADE_OUT: Duration = Duration::from_millis(150);
/// What the fade eases along.
pub const FADE_EASE: CubicBezier = EASE_OUT;
/// A new message while the chip shows: the old words crossfade into the new, the width following.
pub const SWAP: Duration = Duration::from_millis(120);
/// How long a message stays before the chip goes (a message that replaces it starts this over).
pub const LIFE: Duration = Duration::from_secs(2);
/// The same message again within this long is not said twice.
pub const DEDUPE: Duration = Duration::from_secs(2);
/// A spring is within a hundredth of the way it travels (a pixel of the drop, near enough) of its
/// target for good after this long.
const TOLERANCE: f32 = 0.01;

/// How the chip comes and goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entrance {
    /// Drops from under the title row and goes back up.
    Drop,
    /// Fades in place: Reduce Motion.
    Fade,
}

impl Entrance {
    /// The way the chip comes and goes when Reduce Motion is `reduced`.
    #[must_use]
    pub fn of(reduced: bool) -> Self {
        if reduced { Self::Fade } else { Self::Drop }
    }

    /// How long the chip takes to arrive.
    #[must_use]
    pub fn arrive(self) -> Duration {
        match self {
            Self::Drop => DROP.settle(TOLERANCE),
            Self::Fade => FADE_IN,
        }
    }

    /// How long the chip takes to leave: until then the panel must stay.
    #[must_use]
    pub fn leave(self) -> Duration {
        match self {
            Self::Drop => UP,
            Self::Fade => FADE_OUT,
        }
    }
}

/// How far a chip `height` points tall travels to go from rest to out of sight above the title row's
/// edge: its own height and the gap it rests at.
#[must_use]
pub fn travel(height: f32) -> f32 {
    height + REST_GAP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_are_the_ones_the_design_decided() {
        assert_eq!((DROP.stiffness, DROP.damping), (400.0, 30.0));
        assert_eq!(REST_GAP, 8.0);
        assert_eq!(UP, Duration::from_millis(180));
        assert_eq!((FADE_IN, FADE_OUT), (Duration::from_millis(200), Duration::from_millis(150)));
        assert_eq!(SWAP, Duration::from_millis(120));
        assert_eq!((LIFE, DEDUPE), (Duration::from_secs(2), Duration::from_secs(2)));
        assert_eq!(UP_EASE, EASE_IN);
    }

    #[test]
    fn reduce_motion_fades_and_otherwise_the_chip_drops() {
        assert_eq!(Entrance::of(true), Entrance::Fade);
        assert_eq!(Entrance::of(false), Entrance::Drop);
        assert_eq!(Entrance::Fade.arrive(), FADE_IN);
        assert_eq!(Entrance::Fade.leave(), FADE_OUT);
        assert_eq!(Entrance::Drop.leave(), UP);
    }

    #[test]
    fn the_drop_settles_in_well_under_a_second_and_after_it_has_started() {
        let arrive = Entrance::Drop.arrive();
        assert!(arrive > Duration::from_millis(100) && arrive < Duration::from_millis(700), "{arrive:?}");
        assert!(arrive < LIFE, "the chip is at rest before it starts to leave");
    }

    #[test]
    fn the_chip_travels_its_height_and_its_gap() {
        assert_eq!(travel(32.0), 40.0);
    }
}
