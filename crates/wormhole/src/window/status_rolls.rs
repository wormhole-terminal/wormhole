//! The status bar's counts turning over: which counts changed since the bar was last drawn, from what,
//! and for how long each is still rolling. The bar is a cached region, built when the window says a fact
//! of it changed, so the bar remembers a roll itself until its time is up (a build in the middle of one,
//! by the tick, carries it on instead of cutting it), and draws it as an animation of the region that
//! holds it, which draws that region and nothing else. At rest it holds nothing: no roll, no work.

use std::time::Instant;

use wormhole_core::view::layout::status_bar::Activity;
use wormhole_core::view::motion::roll::DURATION;

use crate::style::motion::LAST_FRAME_GRACE;
use crate::style::{NumberRoll, RollClock};

/// One count that is rolling.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rolling {
    activity: Activity,
    from: usize,
    to: usize,
    /// Which change of the bar's counts it was: tells its animation from the next one's.
    turn: u64,
    began: Instant,
}

/// The counts that are rolling now.
#[derive(Debug, Default)]
pub(super) struct StatusRolls {
    rolling: Vec<Rolling>,
    turns: u64,
}

impl StatusRolls {
    /// The bar is built with the counts `after`, having been drawn with `before`: every count of an item
    /// that was there and has another number now starts a roll. A roll that is over is dropped. With
    /// `on` false (Reduce Motion, or no Motion feature) nothing rolls.
    pub(super) fn note(
        &mut self,
        before: impl Iterator<Item = (Activity, usize)> + Clone,
        after: impl Iterator<Item = (Activity, usize)>,
        now: Instant,
        on: bool,
    ) {
        self.rolling.retain(|roll| on && now.duration_since(roll.began) < DURATION + LAST_FRAME_GRACE);
        if !on {
            return;
        }
        for (activity, to) in after {
            let Some((_, from)) = before.clone().find(|(was, _)| *was == activity) else { continue };
            if from != to {
                self.turns += 1;
                self.rolling.retain(|roll| roll.activity != activity);
                self.rolling.push(Rolling { activity, from, to, turn: self.turns, began: now });
            }
        }
    }

    /// How the count of `activity`, which is `to` now, is turning over, if it is. A roll to another number
    /// than the bar shows is a stale one, and not drawn.
    pub(super) fn of(&self, activity: Activity, to: usize, now: Instant) -> Option<NumberRoll> {
        self.rolling
            .iter()
            .find(|roll| roll.activity == activity && roll.to == to)
            .filter(|roll| now.duration_since(roll.began) < DURATION + LAST_FRAME_GRACE)
            .map(|roll| NumberRoll { from: roll.from, clock: RollClock::Playing(roll.turn) })
    }

    /// Whether any count is rolling.
    #[cfg(test)]
    pub(super) fn is_rolling(&self) -> bool {
        !self.rolling.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use wormhole_core::view::layout::status_bar::Activity;

    use super::StatusRolls;
    use crate::style::RollClock;

    const WORKING: Activity = Activity::Working;
    const DONE: Activity = Activity::Done;

    fn at(now: Instant, ms: u64) -> Instant {
        now + Duration::from_millis(ms)
    }

    #[test]
    fn a_count_that_changed_rolls_from_the_number_it_was() {
        let (mut rolls, now) = (StatusRolls::default(), Instant::now());
        rolls.note([(WORKING, 2), (DONE, 1)].into_iter(), [(WORKING, 3), (DONE, 1)].into_iter(), now, true);
        let roll = rolls.of(WORKING, 3, now).expect("working rolls");
        assert_eq!(roll.from, 2);
        assert!(matches!(roll.clock, RollClock::Playing(_)));
        assert!(rolls.of(DONE, 1, now).is_none(), "a count that is the same does not");
    }

    #[test]
    fn an_item_that_comes_or_goes_does_not_roll() {
        let (mut rolls, now) = (StatusRolls::default(), Instant::now());
        rolls.note([(WORKING, 2)].into_iter(), [(WORKING, 2), (DONE, 1)].into_iter(), now, true);
        rolls.note([(DONE, 1)].into_iter(), [].into_iter(), now, true);
        assert!(!rolls.is_rolling());
    }

    #[test]
    fn a_roll_is_over_after_its_160_ms_and_a_build_in_the_middle_of_one_carries_it_on() {
        let (mut rolls, now) = (StatusRolls::default(), Instant::now());
        rolls.note([(WORKING, 2)].into_iter(), [(WORKING, 3)].into_iter(), now, true);
        let first = rolls.of(WORKING, 3, now).expect("rolling");
        // The tick builds the bar again with nothing new: the same roll, the same animation.
        rolls.note([(WORKING, 3)].into_iter(), [(WORKING, 3)].into_iter(), at(now, 80), true);
        assert_eq!(rolls.of(WORKING, 3, at(now, 80)), Some(first));
        assert!(rolls.of(WORKING, 3, at(now, 400)).is_none(), "over");
        rolls.note([(WORKING, 3)].into_iter(), [(WORKING, 3)].into_iter(), at(now, 400), true);
        assert!(!rolls.is_rolling(), "and forgotten: nothing is held at rest");
    }

    #[test]
    fn another_change_in_the_middle_starts_a_roll_of_its_own_from_where_the_count_was() {
        let (mut rolls, now) = (StatusRolls::default(), Instant::now());
        rolls.note([(WORKING, 2)].into_iter(), [(WORKING, 3)].into_iter(), now, true);
        let first = rolls.of(WORKING, 3, now).expect("rolling");
        rolls.note([(WORKING, 3)].into_iter(), [(WORKING, 4)].into_iter(), at(now, 50), true);
        let second = rolls.of(WORKING, 4, at(now, 50)).expect("rolling");
        assert_eq!(second.from, 3);
        assert_ne!(first.clock, second.clock, "an animation of its own, which runs from its start");
        assert!(rolls.of(WORKING, 3, at(now, 50)).is_none(), "the first is stale");
    }

    #[test]
    fn nothing_rolls_while_motion_is_off() {
        let (mut rolls, now) = (StatusRolls::default(), Instant::now());
        rolls.note([(WORKING, 2)].into_iter(), [(WORKING, 3)].into_iter(), now, false);
        assert!(!rolls.is_rolling());
    }
}
