//! When the chip shows what. One chip per window: a message while nothing shows drops it in, a different
//! one while it shows swaps the words and starts the chip's life over, and the same one again soon after is
//! not said twice. A sticky message (a warning) has no life: it stays until it is dismissed, a copy that
//! arrives meanwhile takes the words for its own life and the sticky words come back after it, and once
//! dismissed the same warning is not said again. The clock is the caller's (`now` is passed in), so the
//! rules are tested without waiting and the app's timer decides when to ask again (`due`).

use std::time::{Duration, Instant};

use super::message::Notice;
use crate::view::motion::notice::{DEDUPE, LIFE};

/// Where the chip is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Hidden,
    /// Shown, or on its way down; it rests until its life is over (or, for a sticky message, until it is
    /// dismissed).
    Showing,
    /// Going back up (or fading out): the panel stays until it is out of sight.
    Leaving,
}

/// What the caller does about a message, or about time passing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Bring the chip in with the message just posted.
    Show,
    /// The chip is up: put the message's words in it, without dropping again.
    Swap,
    /// A message's time is over while a sticky one waits behind it: put the sticky words back
    /// ([`ChipRules::held`] says which).
    Restore,
    /// Life is over, or the chip was dismissed: take the chip away.
    Hide,
    /// The chip is out of sight: let the panel go.
    Remove,
    /// Nothing changes.
    Ignore,
}

#[derive(Clone, Copy, Debug)]
enum State {
    Hidden,
    /// `until`: when its life is over; `None` while a sticky message is on the chip.
    Showing {
        until: Option<Instant>,
    },
    Leaving {
        until: Instant,
    },
}

/// One window's chip.
#[derive(Debug)]
pub struct ChipRules {
    state: State,
    /// The last message that was said, and when: what the same message again is held against.
    last: Option<(Notice, Instant)>,
    /// The sticky message the chip holds, while one is up (it may be under a copy's words for a moment).
    sticky: Option<Notice>,
    /// Sticky messages the person dismissed: said once, not again.
    dismissed: Vec<Notice>,
}

impl Default for ChipRules {
    fn default() -> Self {
        Self { state: State::Hidden, last: None, sticky: None, dismissed: Vec::new() }
    }
}

impl ChipRules {
    /// A message arrives at `now`.
    pub fn post(&mut self, notice: Notice, now: Instant) -> Effect {
        if notice.is_sticky() {
            return self.post_sticky(notice);
        }
        if self.last.is_some_and(|(last, at)| last == notice && now.saturating_duration_since(at) < DEDUPE) {
            return Effect::Ignore;
        }
        self.last = Some((notice, now));
        self.show(Some(now + LIFE))
    }

    /// A message that stays. It is not held against `last` (that is for copies said twice); it is said
    /// once while it is up, and not again once it was dismissed.
    fn post_sticky(&mut self, notice: Notice) -> Effect {
        if self.sticky == Some(notice) || self.dismissed.contains(&notice) {
            return Effect::Ignore;
        }
        self.sticky = Some(notice);
        self.show(None)
    }

    fn show(&mut self, until: Option<Instant>) -> Effect {
        let effect = match self.state {
            State::Showing { .. } => Effect::Swap,
            // Coming back while it leaves drops it in anew: the panel is still there.
            State::Hidden | State::Leaving { .. } => Effect::Show,
        };
        self.state = State::Showing { until };
        effect
    }

    /// Time has come to `now`. `leave` is how long the chip takes to go, which depends on whether Reduce
    /// Motion is on, so it is the caller's to say.
    pub fn advance(&mut self, now: Instant, leave: Duration) -> Effect {
        match self.state {
            State::Showing { until: Some(until) } if now >= until => {
                if self.sticky.is_some() {
                    self.state = State::Showing { until: None };
                    Effect::Restore
                } else {
                    self.state = State::Leaving { until: now + leave };
                    Effect::Hide
                }
            }
            State::Leaving { until } if now >= until => {
                self.state = State::Hidden;
                Effect::Remove
            }
            _ => Effect::Ignore,
        }
    }

    /// The person clicked the chip at `now`: a sticky message on it goes, for good. A click on a chip that
    /// shows a copy (or none) does nothing: it is not [`Self::clickable`].
    pub fn dismiss(&mut self, now: Instant, leave: Duration) -> Effect {
        if !self.clickable() {
            return Effect::Ignore;
        }
        if let Some(notice) = self.sticky.take() {
            self.dismissed.push(notice);
        }
        self.state = State::Leaving { until: now + leave };
        Effect::Hide
    }

    /// When [`Self::advance`] next has something to do, if it does: the caller sets a timer for it. A
    /// sticky message has nothing to wait for.
    #[must_use]
    pub fn due(&self) -> Option<Instant> {
        match self.state {
            State::Hidden | State::Showing { until: None } => None,
            State::Showing { until: Some(until) } | State::Leaving { until } => Some(until),
        }
    }

    #[must_use]
    pub fn phase(&self) -> Phase {
        match self.state {
            State::Hidden => Phase::Hidden,
            State::Showing { .. } => Phase::Showing,
            State::Leaving { .. } => Phase::Leaving,
        }
    }

    /// The sticky message the chip holds, whether its words show now or a copy's do.
    #[must_use]
    pub fn held(&self) -> Option<Notice> {
        self.sticky
    }

    /// The chip shows a sticky message's own words, so it takes a click (and only then: a copy's chip
    /// ignores the mouse, even with a sticky message waiting under it).
    #[must_use]
    pub fn clickable(&self) -> bool {
        matches!(self.state, State::Showing { until: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::notice::{Copied, Warning};

    const LEAVE: Duration = Duration::from_millis(180);
    const MS: fn(u64) -> Duration = Duration::from_millis;

    fn path() -> Notice {
        Copied::Path.into()
    }

    fn lines(n: usize) -> Notice {
        Copied::Lines(n).into()
    }

    fn warned() -> Notice {
        Notice::Warned(Warning::HostFallback)
    }

    #[test]
    fn a_message_while_nothing_shows_drops_the_chip_in() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        assert_eq!(chip.phase(), Phase::Hidden);
        assert_eq!(chip.post(path(), t0), Effect::Show);
        assert_eq!(chip.phase(), Phase::Showing);
        assert_eq!(chip.due(), Some(t0 + LIFE));
    }

    #[test]
    fn a_different_message_swaps_the_words_and_starts_the_life_over() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(lines(3), t0);
        assert_eq!(chip.post(path(), t0 + MS(1500)), Effect::Swap);
        assert_eq!(chip.due(), Some(t0 + MS(1500) + LIFE), "two seconds more from the new message");
        assert_eq!(
            chip.advance(t0 + LIFE, LEAVE),
            Effect::Ignore,
            "the first message's time no longer counts"
        );
    }

    #[test]
    fn the_same_message_within_two_seconds_is_dropped_and_does_not_extend_the_life() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        assert_eq!(chip.post(path(), t0 + MS(1900)), Effect::Ignore);
        assert_eq!(chip.due(), Some(t0 + LIFE));
    }

    #[test]
    fn the_same_message_is_held_against_the_last_one_said_even_after_another_came_between() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        chip.post(lines(2), t0 + MS(500));
        // `path` was said 1.5 s ago, but the last said was `lines`: this is a different message.
        assert_eq!(chip.post(path(), t0 + MS(900)), Effect::Swap);
        assert_eq!(chip.post(path(), t0 + MS(1200)), Effect::Ignore);
    }

    #[test]
    fn the_same_message_after_two_seconds_is_said_again() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        assert_eq!(chip.advance(t0 + LIFE, LEAVE), Effect::Hide);
        assert_eq!(chip.post(path(), t0 + LIFE), Effect::Show, "it left, and was last said 2 s ago");
    }

    #[test]
    fn the_chip_leaves_when_its_life_is_over_and_the_panel_goes_when_it_is_out_of_sight() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        assert_eq!(chip.advance(t0 + MS(1999), LEAVE), Effect::Ignore);
        assert_eq!(chip.advance(t0 + LIFE, LEAVE), Effect::Hide);
        assert_eq!(chip.phase(), Phase::Leaving);
        assert_eq!(chip.due(), Some(t0 + LIFE + LEAVE));
        assert_eq!(chip.advance(t0 + LIFE + MS(100), LEAVE), Effect::Ignore);
        assert_eq!(chip.advance(t0 + LIFE + LEAVE, LEAVE), Effect::Remove);
        assert_eq!((chip.phase(), chip.due()), (Phase::Hidden, None));
        assert_eq!(chip.advance(t0 + MS(9000), LEAVE), Effect::Ignore, "nothing more happens by itself");
    }

    #[test]
    fn a_message_while_the_chip_leaves_brings_it_back() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        chip.advance(t0 + LIFE, LEAVE);
        assert_eq!(chip.post(lines(4), t0 + LIFE + MS(50)), Effect::Show);
        assert_eq!(chip.phase(), Phase::Showing);
        assert_eq!(chip.advance(t0 + LIFE + LEAVE, LEAVE), Effect::Ignore, "the old leaving is forgotten");
    }

    #[test]
    fn the_time_a_chip_takes_to_leave_is_the_callers() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        chip.advance(t0 + LIFE, MS(150));
        assert_eq!(chip.due(), Some(t0 + LIFE + MS(150)));
    }

    #[test]
    fn a_sticky_message_shows_and_has_no_life() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        assert_eq!(chip.post(warned(), t0), Effect::Show);
        assert_eq!(chip.phase(), Phase::Showing);
        assert_eq!(chip.due(), None, "nothing to wait for");
        assert!(chip.clickable());
        assert_eq!(chip.held(), Some(warned()));
        assert_eq!(chip.advance(t0 + MS(60_000), LEAVE), Effect::Ignore, "a minute on, it is still there");
        assert_eq!(chip.phase(), Phase::Showing);
    }

    #[test]
    fn a_copy_is_never_clickable() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        assert!(!chip.clickable());
        assert_eq!(chip.dismiss(t0, LEAVE), Effect::Ignore);
        assert_eq!(chip.phase(), Phase::Showing, "a click on a copy's chip takes nothing away");
    }

    #[test]
    fn the_same_sticky_message_again_while_it_shows_is_dropped() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(warned(), t0);
        assert_eq!(chip.post(warned(), t0 + MS(5000)), Effect::Ignore);
        assert_eq!(chip.due(), None);
    }

    #[test]
    fn a_copy_while_a_sticky_message_shows_takes_the_words_for_its_life_and_the_sticky_ones_come_back() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(warned(), t0);
        let t1 = t0 + MS(10_000);
        assert_eq!(
            chip.post(path(), t1),
            Effect::Swap,
            "the copy's words go in, the chip does not drop again"
        );
        assert!(!chip.clickable(), "a copy's words ignore the mouse");
        assert_eq!(chip.held(), Some(warned()), "the warning waits under it");
        assert_eq!(chip.due(), Some(t1 + LIFE));
        assert_eq!(chip.advance(t1 + MS(1999), LEAVE), Effect::Ignore);
        assert_eq!(chip.advance(t1 + LIFE, LEAVE), Effect::Restore, "the chip stays and the words return");
        assert_eq!(chip.phase(), Phase::Showing);
        assert!(chip.clickable());
        assert_eq!(chip.due(), None);
    }

    #[test]
    fn a_second_copy_while_the_first_shows_over_a_sticky_message_starts_the_life_over() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(warned(), t0);
        chip.post(lines(2), t0 + MS(100));
        assert_eq!(chip.post(path(), t0 + MS(1000)), Effect::Swap);
        assert_eq!(chip.advance(t0 + MS(2100), LEAVE), Effect::Ignore);
        assert_eq!(chip.advance(t0 + MS(3000), LEAVE), Effect::Restore);
    }

    #[test]
    fn a_sticky_message_arriving_over_a_copy_replaces_it_and_stays() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        assert_eq!(chip.post(warned(), t0 + MS(500)), Effect::Swap);
        assert_eq!(chip.due(), None, "the copy's time no longer counts");
        assert_eq!(chip.advance(t0 + LIFE, LEAVE), Effect::Ignore);
        assert!(chip.clickable());
    }

    #[test]
    fn dismissing_takes_the_chip_away_and_the_panel_goes_when_it_is_out_of_sight() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(warned(), t0);
        let t1 = t0 + MS(30_000);
        assert_eq!(chip.dismiss(t1, LEAVE), Effect::Hide);
        assert_eq!(chip.phase(), Phase::Leaving);
        assert_eq!((chip.clickable(), chip.held()), (false, None));
        assert_eq!(chip.due(), Some(t1 + LEAVE));
        assert_eq!(chip.dismiss(t1, LEAVE), Effect::Ignore, "a second click on the way out does nothing");
        assert_eq!(chip.advance(t1 + LEAVE, LEAVE), Effect::Remove);
        assert_eq!(chip.phase(), Phase::Hidden);
    }

    #[test]
    fn a_dismissed_sticky_message_is_not_said_again_but_a_copy_still_is() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(warned(), t0);
        chip.dismiss(t0, LEAVE);
        chip.advance(t0 + LEAVE, LEAVE);
        let later = t0 + MS(3_600_000);
        assert_eq!(chip.post(warned(), later), Effect::Ignore, "an hour on, still not");
        assert_eq!(chip.phase(), Phase::Hidden);
        assert_eq!(chip.post(path(), later), Effect::Show);
    }

    #[test]
    fn dismissing_while_it_leaves_after_a_copy_is_not_possible() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        chip.advance(t0 + LIFE, LEAVE);
        assert_eq!(chip.dismiss(t0 + LIFE, LEAVE), Effect::Ignore);
    }

    #[test]
    fn a_sticky_message_while_the_chip_leaves_brings_it_back() {
        let (mut chip, t0) = (ChipRules::default(), Instant::now());
        chip.post(path(), t0);
        chip.advance(t0 + LIFE, LEAVE);
        assert_eq!(chip.post(warned(), t0 + LIFE + MS(50)), Effect::Show);
        assert_eq!(chip.advance(t0 + LIFE + LEAVE, LEAVE), Effect::Ignore);
        assert!(chip.clickable());
    }
}
