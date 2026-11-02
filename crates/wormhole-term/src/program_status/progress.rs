//! OSC 9;4 (ConEmu's progress, which Windows Terminal and Ghostty show too) as one more candidate
//! for the [`Summary`] OSC 7501 gives the UI, so one progress model feeds the row and the bar.
//!
//! Which one speaks when both do: the candidate that needs the user most, by the same
//! order that ranks 7501's own records (blocked, error, done, working, idle), and 7501 on a tie. So a program that
//! says it is blocked keeps the row whatever a 9;4 bar says, a lingering 7501 `done` does not
//! hide a later 9;4 error or build, and a program that writes both at the same weight (`cargo`
//! and package managers do) shows its 7501 words with no flicker between the two.
//!
//! The mapping: normal and paused are working with a percentage (paused says so in its words),
//! indeterminate is working without one. Error is a bar state, not an outcome: it keeps its
//! percentage, outranks `done` like a 7501 error does, and is *not* a failure the user has to
//! acknowledge (the program still holds it until it sends `9;4;0`), so it never raises the
//! failure notification; the row shows it as work whose words say "Error".
//!
//! Every state is true only while the program runs: the prompt and the program's exit drop it,
//! and so does silence. A bar nobody updates is a program that died without removing it, so, like
//! Ghostty, it goes after [`PROGRESS_TIMEOUT`] without a report.

use std::time::{Duration, Instant};

use super::store::{ProgramStatus, Summary};
use super::{Record, State};

/// How long a 9;4 report stands without another one. Ghostty removes a progress report after 15
/// seconds too, so that a program that crashed (or was killed) mid-bar doesn't leave it up for good;
/// programs that run longer re-send theirs, as Windows Terminal's documentation asks.
pub const PROGRESS_TIMEOUT: Duration = Duration::from_secs(15);

/// A program's progress (OSC 9;4, ConEmu / Windows Terminal / Ghostty).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub state: ProgressState,
    /// 0–100; 0 when the state has no percentage (indeterminate).
    pub percent: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressState {
    Normal,
    Error,
    Indeterminate,
    Paused,
}

/// What a program last reported, and when (the store has no clock of its own).
#[derive(Clone, Copy, Debug)]
pub(super) struct Held {
    progress: Progress,
    at: Instant,
}

impl ProgramStatus {
    /// A program's OSC 9;4 report at `now`; `None` removes it. Any report starts the timeout over.
    /// Returns whether what is shown of it changed (a repeat does not).
    pub fn set_progress(&mut self, progress: Option<Progress>, now: Instant) -> bool {
        // The engine clamps already; this is the type's own promise (0 to 100).
        let held =
            progress.map(|p| Held { progress: Progress { percent: p.percent.min(100), ..p }, at: now });
        let changed = self.progress() != held.map(|h| h.progress);
        self.progress = held;
        changed
    }

    /// Drop the report if it has gone [`PROGRESS_TIMEOUT`] without another. Returns whether it did.
    pub fn expire_progress(&mut self, now: Instant) -> bool {
        match self.progress {
            Some(held) if now.saturating_duration_since(held.at) >= PROGRESS_TIMEOUT => {
                self.progress = None;
                true
            }
            _ => false,
        }
    }

    /// The OSC 9;4 progress as the program last set it, if it still stands.
    pub fn progress(&self) -> Option<Progress> {
        self.progress.map(|held| held.progress)
    }

    /// The program is over, or the terminal was reset: nothing it reported stands. Returns whether
    /// anything did.
    pub(super) fn forget_progress(&mut self) -> bool {
        self.progress.take().is_some()
    }

    /// The summary the 9;4 progress amounts to, if it is set.
    pub(super) fn progress_summary(&self) -> Option<Summary> {
        let progress = self.progress()?;
        let (state, percent) = match progress.state {
            ProgressState::Normal | ProgressState::Paused => (State::Working, Some(progress.percent)),
            ProgressState::Indeterminate => (State::Working, None),
            // 9;4;2 without a percentage reads as 0, which says nothing here.
            ProgressState::Error => (State::Error, Some(progress.percent).filter(|p| *p > 0)),
        };
        Some(Summary {
            bar: Some(progress.state),
            ..Summary::of(Record { state, kind: None, progress: percent, app: None, title: None, msg: None })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{PROGRESS_TIMEOUT, Progress, ProgressState, State};
    use crate::program_status::{ProgramStatus, Record, Report};
    use std::time::{Duration, Instant};

    fn p(state: ProgressState, percent: u8) -> Option<Progress> {
        Some(Progress { state, percent })
    }

    fn record(state: State) -> Report {
        Report::Set {
            id: None,
            record: Record { state, kind: None, progress: None, app: None, title: None, msg: None },
        }
    }

    #[test]
    fn states_map_to_working_with_or_without_a_percentage_and_to_error() {
        let now = Instant::now();
        let mut status = ProgramStatus::default();
        assert!(status.summary().is_none());
        assert!(status.set_progress(p(ProgressState::Normal, 40), now));
        let s = status.summary().expect("summary");
        assert_eq!((s.state, s.progress, s.bar), (State::Working, Some(40), Some(ProgressState::Normal)));
        status.set_progress(p(ProgressState::Paused, 40), now);
        let s = status.summary().expect("summary");
        assert_eq!((s.state, s.progress, s.bar), (State::Working, Some(40), Some(ProgressState::Paused)));
        status.set_progress(p(ProgressState::Indeterminate, 0), now);
        let s = status.summary().expect("summary");
        assert_eq!((s.state, s.progress), (State::Working, None));
        status.set_progress(p(ProgressState::Error, 60), now);
        let s = status.summary().expect("summary");
        assert_eq!((s.state, s.progress, s.bar), (State::Error, Some(60), Some(ProgressState::Error)));
        status.set_progress(p(ProgressState::Error, 0), now);
        assert_eq!(status.summary().expect("summary").progress, None, "an error that names no percentage");
    }

    #[test]
    fn a_percentage_over_100_is_held_at_100_and_a_repeat_changes_nothing() {
        let now = Instant::now();
        let mut status = ProgramStatus::default();
        assert!(status.set_progress(p(ProgressState::Normal, 250), now));
        assert_eq!(status.progress(), p(ProgressState::Normal, 100));
        assert!(!status.set_progress(p(ProgressState::Normal, 100), now));
        assert!(status.set_progress(None, now));
        assert!(!status.set_progress(None, now));
        assert!(status.summary().is_none());
    }

    #[test]
    fn the_candidate_that_needs_the_user_most_wins_and_7501_wins_a_tie() {
        let now = Instant::now();
        let mut status = ProgramStatus::default();
        status.set_progress(p(ProgressState::Normal, 70), now);
        status.apply(record(State::Working));
        assert_eq!(status.summary().expect("summary").bar, None, "equal weight: the 7501 record");
        status.apply(record(State::Done));
        assert_eq!(status.summary().expect("summary").state, State::Done, "done outranks work");
        status.set_progress(p(ProgressState::Error, 0), now);
        let s = status.summary().expect("summary");
        assert_eq!((s.state, s.bar), (State::Error, Some(ProgressState::Error)), "error outranks done");
        status.apply(record(State::Error));
        assert_eq!(status.summary().expect("summary").bar, None, "equal weight again: the 7501 record");
        status.apply(record(State::Blocked));
        assert_eq!(status.summary().expect("summary").state, State::Blocked);
        status.apply(Report::Clear { id: None });
        assert_eq!(status.summary().expect("summary").bar, Some(ProgressState::Error));
        assert_eq!(status.len(), 0, "the 9;4 progress is not a record");
    }

    #[test]
    fn the_prompt_and_the_exit_drop_every_state_but_looking_does_not_drop_an_error() {
        let now = Instant::now();
        let mut status = ProgramStatus::default();
        status.set_progress(p(ProgressState::Paused, 10), now);
        assert!(status.on_prompt());
        assert!(status.progress().is_none());
        status.set_progress(p(ProgressState::Error, 10), now);
        assert!(!status.acknowledge(), "an error bar is a state the program still holds");
        assert!(status.progress().is_some());
        assert!(status.on_process_exit());
        assert!(status.progress().is_none());
    }

    #[test]
    fn a_full_reset_drops_it_but_a_7501_clear_does_not() {
        let mut status = ProgramStatus::default();
        status.set_progress(p(ProgressState::Normal, 5), Instant::now());
        assert!(!status.apply(Report::Clear { id: None }));
        assert!(status.progress().is_some());
        assert!(status.reset());
        assert!(status.progress().is_none());
    }

    #[test]
    fn silence_ends_a_report_and_any_report_starts_the_clock_over() {
        let start = Instant::now();
        let mut status = ProgramStatus::default();
        status.set_progress(p(ProgressState::Normal, 5), start);
        let nearly = start + PROGRESS_TIMEOUT - Duration::from_millis(1);
        assert!(!status.expire_progress(nearly));
        // The same report again changes nothing shown, but it is a sign of life.
        assert!(!status.set_progress(p(ProgressState::Normal, 5), nearly));
        assert!(!status.expire_progress(start + PROGRESS_TIMEOUT));
        assert!(status.expire_progress(nearly + PROGRESS_TIMEOUT));
        assert!(status.progress().is_none());
        assert!(!status.expire_progress(nearly + PROGRESS_TIMEOUT), "nothing left to expire");
        status.set_progress(p(ProgressState::Error, 5), start);
        assert!(status.expire_progress(start + PROGRESS_TIMEOUT), "every state expires");
    }
}
