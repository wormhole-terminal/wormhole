//! A number that rolls: the status bar's counts turn over like a counter's digits. Which places of
//! the number change, which way they roll and where each digit is `t` of the way through are plain
//! arithmetic, here; the app draws a digit in a clip of its own and moves it by the offsets.
//!
//! A number grows or shrinks as a whole: when it grows its digits roll up (the old one leaves through
//! the top, the new one comes from below) and when it shrinks they roll down, so 19 → 20 turns both
//! its places up and 20 → 19 both down, and 11 → 12 turns only the last. A place a longer number has
//! over a shorter one comes in (or goes) as a digit that was never there.

use std::time::Duration;

use super::ease::EASE_OUT;

/// How long a roll takes.
pub const DURATION: Duration = Duration::from_millis(160);

/// Which way the digits roll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The number grew: the old digit leaves through the top.
    Up,
    /// The number shrank: the old digit leaves through the bottom.
    Down,
}

/// One place of the number, the units place last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    /// The digit it showed; `None` where the old number was shorter.
    pub was: Option<u8>,
    /// The digit it shows; `None` where the new number is shorter.
    pub now: Option<u8>,
}

impl Place {
    /// It shows another digit than it did, so it rolls.
    #[must_use]
    pub fn turns(self) -> bool {
        self.was != self.now
    }
}

/// A number turning over: its places from the most significant down, and the way they roll.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roll {
    pub direction: Direction,
    pub places: Vec<Place>,
}

impl Roll {
    /// How `from` turns into `to`; `None` when they are the same number.
    #[must_use]
    pub fn of(from: usize, to: usize) -> Option<Self> {
        if from == to {
            return None;
        }
        let (was, now) = (digits(from), digits(to));
        let longest = was.len().max(now.len());
        // Both numbers' units places line up, so a shorter number is short of its most significant places.
        let at =
            |digits: &[u8], place: usize| (place < digits.len()).then(|| digits[digits.len() - 1 - place]);
        let places =
            (0..longest).rev().map(|place| Place { was: at(&was, place), now: at(&now, place) }).collect();
        let direction = if to > from { Direction::Up } else { Direction::Down };
        Some(Self { direction, places })
    }

    /// Where the old digit and the new one are, `t` (0.0..=1.0) of the way through [`DURATION`], as
    /// shares of a digit's height above (negative) or below (positive) its place: the old one starts in
    /// its place and leaves, the new one starts a digit away and arrives.
    #[must_use]
    pub fn offsets(&self, t: f32) -> (f32, f32) {
        let travelled = EASE_OUT.ease(t);
        let sign = match self.direction {
            Direction::Up => 1.0,
            Direction::Down => -1.0,
        };
        (-sign * travelled, sign * (1.0 - travelled))
    }
}

/// `n`'s decimal digits, the most significant first (`0` is one digit).
fn digits(n: usize) -> Vec<u8> {
    n.to_string().bytes().map(|byte| byte - b'0').collect()
}

#[cfg(test)]
mod tests {
    use super::{DURATION, Direction, Place, Roll};

    fn place(was: Option<u8>, now: Option<u8>) -> Place {
        Place { was, now }
    }

    #[test]
    fn the_same_number_does_not_roll() {
        assert_eq!(Roll::of(7, 7), None);
        assert_eq!(Roll::of(0, 0), None);
    }

    #[test]
    fn a_number_that_grows_rolls_up_and_one_that_shrinks_down() {
        let up = Roll::of(2, 3).expect("a roll");
        assert_eq!((up.direction, up.places), (Direction::Up, vec![place(Some(2), Some(3))]));
        let down = Roll::of(3, 0).expect("a roll");
        assert_eq!((down.direction, down.places), (Direction::Down, vec![place(Some(3), Some(0))]));
    }

    #[test]
    fn only_the_places_that_change_turn() {
        let roll = Roll::of(11, 12).expect("a roll");
        assert_eq!(roll.places, vec![place(Some(1), Some(1)), place(Some(1), Some(2))]);
        assert_eq!(roll.places.iter().map(|p| p.turns()).collect::<Vec<_>>(), [false, true]);
        let carry = Roll::of(19, 20).expect("a roll");
        assert!(carry.places.iter().all(|p| p.turns()), "19 → 20 turns both places");
        assert_eq!(carry.direction, Direction::Up);
        assert_eq!(Roll::of(20, 19).expect("a roll").direction, Direction::Down, "and 20 → 19 both down");
    }

    #[test]
    fn a_longer_number_has_a_place_the_shorter_one_did_not() {
        let more = Roll::of(9, 10).expect("a roll");
        assert_eq!(more.places, vec![place(None, Some(1)), place(Some(9), Some(0))]);
        let fewer = Roll::of(100, 99).expect("a roll");
        assert_eq!(
            fewer.places,
            vec![place(Some(1), None), place(Some(0), Some(9)), place(Some(0), Some(9))]
        );
    }

    #[test]
    fn the_digits_start_in_place_and_end_a_digit_away_in_the_direction_of_the_roll() {
        let up = Roll::of(1, 2).expect("a roll");
        assert_eq!(up.offsets(0.0), (0.0, 1.0), "the new digit waits below");
        assert_eq!(up.offsets(1.0), (-1.0, 0.0), "the old one left through the top");
        let down = Roll::of(2, 1).expect("a roll");
        assert_eq!(down.offsets(0.0), (0.0, -1.0));
        assert_eq!(down.offsets(1.0), (1.0, 0.0));
        let (old, new) = up.offsets(0.5);
        assert!(old < -0.5 && new < 0.5, "eased out: past half way at half the time: {old} {new}");
        assert!((new - (old + 1.0)).abs() < 1e-5, "the two digits are always one digit apart");
    }

    #[test]
    fn a_roll_takes_160_ms() {
        assert_eq!(DURATION.as_millis(), 160);
    }
}
