//! When the chip shows what. One chip per window: a message while nothing shows drops it in, a different
//! one while it shows swaps the words and starts the chip's life over, and the same one again soon after is
//! not said twice. The clock is the caller's (`now` is passed in), so the rules are tested without waiting
//! and the app's timer decides when to ask again (`due`).

use std::time::{Duration, Instant};

use super::message::Notice;
use crate::view::motion::notice::{DEDUPE, LIFE};

/// Where the chip is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Hidden,
    /// Shown, or on its way down; it rests until its life is over.
    Showing,
    /// Going back up (or fading out): the panel stays until it is out of sight.
    Leaving,
}

/// What the caller does about a message, or about time passing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Bring the chip in with this message.
    Show(Notice),
    /// The chip is up: put these words in it, without dropping again.
    Swap(Notice),
    /// Life is over: take the chip away.
    Hide,
    /// The chip is out of sight: let the panel go.
    Remove,
    /// Nothing changes.
    Ignore,
}

#[derive(Clone, Copy, Debug)]
enum State {
    Hidden,
    Showing { notice: Notice, until: Instant },
    Leaving { until: Instant },
}

/// One window's chip.
#[derive(Debug)]
pub struct Notices {
    state: State,
    /// The last message that was said, and when: what the same message again is held against.
    last: Option<(Notice, Instant)>,
}

impl Default for Notices {
    fn default() -> Self {
        Self { state: State::Hidden, last: None }
    }
}

impl Notices {
    /// A message arrives at `now`.
    pub fn post(&mut self, notice: Notice, now: Instant) -> Effect {
        if self.last.is_some_and(|(last, at)| last == notice && now.saturating_duration_since(at) < DEDUPE) {
            return Effect::Ignore;
        }
        self.last = Some((notice, now));
        let until = now + LIFE;
        match self.state {
            State::Showing { .. } => {
                self.state = State::Showing { notice, until };
                Effect::Swap(notice)
            }
            // Coming back while it leaves drops it in anew: the panel is still there.
            State::Hidden | State::Leaving { .. } => {
                self.state = State::Showing { notice, until };
                Effect::Show(notice)
            }
        }
    }

    /// Time has come to `now`. `leave` is how long the chip takes to go, which depends on whether Reduce
    /// Motion is on, so it is the caller's to say.
    pub fn advance(&mut self, now: Instant, leave: Duration) -> Effect {
        match self.state {
            State::Showing { until, .. } if now >= until => {
                self.state = State::Leaving { until: now + leave };
                Effect::Hide
            }
            State::Leaving { until } if now >= until => {
                self.state = State::Hidden;
                Effect::Remove
            }
            _ => Effect::Ignore,
        }
    }

    /// When [`Self::advance`] next has something to do, if it does: the caller sets a timer for it.
    #[must_use]
    pub fn due(&self) -> Option<Instant> {
        match self.state {
            State::Hidden => None,
            State::Showing { until, .. } | State::Leaving { until } => Some(until),
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

    /// The message the chip shows now.
    #[must_use]
    pub fn current(&self) -> Option<Notice> {
        match self.state {
            State::Showing { notice, .. } => Some(notice),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::notice::Copied;

    const LEAVE: Duration = Duration::from_millis(180);
    const MS: fn(u64) -> Duration = Duration::from_millis;

    fn path() -> Notice {
        Copied::Path.into()
    }

    fn lines(n: usize) -> Notice {
        Copied::Text { lines: n, chars: n * 10 }.into()
    }

    #[test]
    fn a_message_while_nothing_shows_drops_the_chip_in() {
        let (mut chip, t0) = (Notices::default(), Instant::now());
        assert_eq!(chip.phase(), Phase::Hidden);
        assert_eq!(chip.post(path(), t0), Effect::Show(path()));
        assert_eq!((chip.phase(), chip.current()), (Phase::Showing, Some(path())));
        assert_eq!(chip.due(), Some(t0 + LIFE));
    }

    #[test]
    fn a_different_message_swaps_the_words_and_starts_the_life_over() {
        let (mut chip, t0) = (Notices::default(), Instant::now());
        chip.post(lines(3), t0);
        assert_eq!(chip.post(path(), t0 + MS(1500)), Effect::Swap(path()));
        assert_eq!(chip.current(), Some(path()));
        assert_eq!(chip.due(), Some(t0 + MS(1500) + LIFE), "two seconds more from the new message");
        assert_eq!(
            chip.advance(t0 + LIFE, LEAVE),
            Effect::Ignore,
            "the first message's time no longer counts"
        );
    }

    #[test]
    fn the_same_message_within_two_seconds_is_dropped_and_does_not_extend_the_life() {
        let (mut chip, t0) = (Notices::default(), Instant::now());
        chip.post(path(), t0);
        assert_eq!(chip.post(path(), t0 + MS(1900)), Effect::Ignore);
        assert_eq!(chip.due(), Some(t0 + LIFE));
    }

    #[test]
    fn the_same_message_is_held_against_the_last_one_said_even_after_another_came_between() {
        let (mut chip, t0) = (Notices::default(), Instant::now());
        chip.post(path(), t0);
        chip.post(lines(2), t0 + MS(500));
        // `path` was said 1.5 s ago, but the last said was `lines`: this is a different message.
        assert_eq!(chip.post(path(), t0 + MS(900)), Effect::Swap(path()));
        assert_eq!(chip.post(path(), t0 + MS(1200)), Effect::Ignore);
    }

    #[test]
    fn the_same_message_after_two_seconds_is_said_again() {
        let (mut chip, t0) = (Notices::default(), Instant::now());
        chip.post(path(), t0);
        assert_eq!(chip.advance(t0 + LIFE, LEAVE), Effect::Hide);
        assert_eq!(chip.post(path(), t0 + LIFE), Effect::Show(path()), "it left, and was last said 2 s ago");
    }

    #[test]
    fn the_chip_leaves_when_its_life_is_over_and_the_panel_goes_when_it_is_out_of_sight() {
        let (mut chip, t0) = (Notices::default(), Instant::now());
        chip.post(path(), t0);
        assert_eq!(chip.advance(t0 + MS(1999), LEAVE), Effect::Ignore);
        assert_eq!(chip.advance(t0 + LIFE, LEAVE), Effect::Hide);
        assert_eq!((chip.phase(), chip.current()), (Phase::Leaving, None));
        assert_eq!(chip.due(), Some(t0 + LIFE + LEAVE));
        assert_eq!(chip.advance(t0 + LIFE + MS(100), LEAVE), Effect::Ignore);
        assert_eq!(chip.advance(t0 + LIFE + LEAVE, LEAVE), Effect::Remove);
        assert_eq!((chip.phase(), chip.due()), (Phase::Hidden, None));
        assert_eq!(chip.advance(t0 + MS(9000), LEAVE), Effect::Ignore, "nothing more happens by itself");
    }

    #[test]
    fn a_message_while_the_chip_leaves_brings_it_back() {
        let (mut chip, t0) = (Notices::default(), Instant::now());
        chip.post(path(), t0);
        chip.advance(t0 + LIFE, LEAVE);
        assert_eq!(chip.post(lines(4), t0 + LIFE + MS(50)), Effect::Show(lines(4)));
        assert_eq!(chip.phase(), Phase::Showing);
        assert_eq!(chip.advance(t0 + LIFE + LEAVE, LEAVE), Effect::Ignore, "the old leaving is forgotten");
    }

    #[test]
    fn the_time_a_chip_takes_to_leave_is_the_callers() {
        let (mut chip, t0) = (Notices::default(), Instant::now());
        chip.post(path(), t0);
        chip.advance(t0 + LIFE, MS(150));
        assert_eq!(chip.due(), Some(t0 + LIFE + MS(150)));
    }
}
