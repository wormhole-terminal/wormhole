//! A number that turns over like a counter's digits: each place that changes rolls its old digit out of a
//! clip of its own, up or down by whether the number grew or shrank, as the new one rolls in. Which places
//! turn and where each digit is are core's (`view::motion::roll`); this draws them. A number at rest is
//! plain text, which the caller draws itself: this is for the moment it changes, so a bar of counts that
//! do not change costs what it did before.
//!
//! The text is the caller's: size, weight and colour are inherited, and the digits must be tabular (the
//! status bar's are) so a place is as wide whatever it shows.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::roll::{DURATION, Roll};

use super::motion::ANIMATION_FPS;

/// How a number is turning over, and how far along it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RollClock {
    /// An animation of [`DURATION`] that runs once, from the first frame it is drawn. The number tells
    /// this change from the one before it, so a second one runs from its start.
    Playing(u64),
    /// `t` (0.0..=1.0) of the way through the roll.
    At(f32),
}

/// A number turning over from `from`, as `clock` says.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberRoll {
    pub from: usize,
    pub clock: RollClock,
}

/// `to`, turning over from the number `roll` says it was: every place that differs rolls, in a clip as
/// high as its text and as wide as a digit, and the places that do not stay as they are. `id` keys the
/// animation (it must be the same for the same number from one frame to the next). Plain text for a number
/// that is not changing.
#[track_caller]
pub fn rolling_number(id: impl Into<ElementId>, to: usize, roll: Option<NumberRoll>) -> AnyElement {
    let Some((roll, clock)) = roll.and_then(|number| Some((Roll::of(number.from, to)?, number.clock))) else {
        return to.to_string().into_any_element();
    };
    let places = div().flex().items_center();
    match clock {
        RollClock::At(t) => places.children(drawn(&roll, to, t)).into_any_element(),
        RollClock::Playing(turn) => places
            .with_animation(
                (id.into(), turn.to_string()),
                Animation::new(DURATION).with_max_fps(ANIMATION_FPS),
                move |places, t| places.children(drawn(&roll, to, t)),
            )
            .into_any_element(),
    }
}

/// The number `t` of the way through its roll: its places while it rolls, and once it has rolled the number as
/// plain text, so a number that lost a place (10 to 9) does not keep a blank one until it is drawn again.
#[track_caller]
fn drawn(roll: &Roll, to: usize, t: f32) -> Vec<AnyElement> {
    if t >= 1.0 { vec![to.to_string().into_any_element()] } else { cells(roll, t) }
}

/// The places of the number `t` of the way through its roll.
#[track_caller]
fn cells(roll: &Roll, t: f32) -> Vec<AnyElement> {
    let (leaving, arriving) = roll.offsets(t);
    let digit = |digit: u8| SharedString::from(digit.to_string());
    roll.places
        .iter()
        .map(|place| match (place.was, place.now) {
            (Some(was), Some(now)) if !place.turns() => {
                debug_assert_eq!(was, now);
                div().child(digit(now)).into_any_element()
            }
            (was, now) => {
                // The clip's size is its text's, which an invisible digit gives while the two that move lie
                // over it: a place that is leaving keeps its width until the roll is over.
                let size = now.or(was).map(digit);
                div()
                    .relative()
                    .overflow_hidden()
                    .child(div().invisible().children(size))
                    .children(was.map(|was| {
                        div().absolute().left_0().w_full().top(relative(leaving)).child(digit(was))
                    }))
                    .children(now.map(|now| {
                        div().absolute().left_0().w_full().top(relative(arriving)).child(digit(now))
                    }))
                    .into_any_element()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::roll::Roll;

    use super::{cells, drawn};

    #[test]
    fn a_place_is_one_element_whether_it_turns_or_not_and_a_longer_number_has_one_more() {
        for (from, to, places) in [(2, 3, 1), (11, 12, 2), (19, 20, 2), (9, 10, 2), (100, 99, 3)] {
            let roll = Roll::of(from, to).expect("a roll");
            assert_eq!(cells(&roll, 0.5).len(), places, "{from} to {to}");
        }
    }

    #[test]
    fn a_roll_that_is_over_is_the_number_in_one_piece_whatever_places_it_had() {
        let roll = Roll::of(10, 9).expect("a roll");
        assert_eq!(drawn(&roll, 9, 0.99).len(), 2, "while it rolls the tens place is still there");
        assert_eq!(drawn(&roll, 9, 1.0).len(), 1, "after it the number is just its digit");
    }
}
