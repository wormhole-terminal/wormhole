//! OSC 9;4 (ConEmu's progress, which Windows Terminal and Ghostty show too) as a fallback source
//! for the same [`Summary`] OSC 7501 gives the UI, so one progress model feeds the row and the bar.
//!
//! Which one speaks when both do: OSC 7501 wins for as long as the terminal holds any of its
//! records. It is the richer protocol (a state, words, what a program waits for), a program that
//! sends it means it, and a 9;4 mapped over it would replace a real report's kind and message.
//! Nothing is lost by it: the 9;4 progress is kept beside the records and shows again the moment
//! the last record goes (a clear, the prompt, the program's exit), if the program still has it set.
//! "Most recent wins" was the other choice; it makes the row flicker for a tool that writes both,
//! as `cargo` and package managers do, so it was not taken.
//!
//! The mapping: normal and paused are working with a percentage (paused says so in its words),
//! indeterminate is working without one, and error is a failure. Like their 7501 counterparts,
//! the working ones are true only while the program runs, so the prompt and the program's exit drop
//! them; a failure stays until the user has looked.

use super::store::{ProgramStatus, Summary};
use super::{Record, State};

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

impl ProgramStatus {
    /// A program's OSC 9;4 report; `None` removes it. Returns whether what is held changed.
    pub fn set_progress(&mut self, progress: Option<Progress>) -> bool {
        // The engine clamps already; this is the type's own promise (0 to 100).
        let progress = progress.map(|p| Progress { percent: p.percent.min(100), ..p });
        std::mem::replace(&mut self.progress, progress) != progress
    }

    /// The OSC 9;4 progress as the program last set it, if it is still set.
    pub fn progress(&self) -> Option<Progress> {
        self.progress
    }

    /// The summary the 9;4 progress amounts to, for when no 7501 record is held.
    pub(super) fn progress_summary(&self) -> Option<Summary> {
        let progress = self.progress?;
        let (state, percent) = match progress.state {
            ProgressState::Normal | ProgressState::Paused => (State::Working, Some(progress.percent)),
            ProgressState::Indeterminate => (State::Working, None),
            ProgressState::Error => (State::Error, None),
        };
        Some(Summary {
            paused: progress.state == ProgressState::Paused,
            ..Summary::of(Record { state, kind: None, progress: percent, app: None, title: None, msg: None })
        })
    }

    /// The program is over (a new prompt, or the process left): what only it could say goes. A
    /// failure stays for the user to see, as a 7501 error does.
    pub(super) fn drop_running_progress(&mut self) -> bool {
        match self.progress {
            Some(Progress { state: ProgressState::Error, .. }) | None => false,
            Some(_) => self.set_progress(None),
        }
    }

    /// The user has looked: a failure has done its job.
    pub(super) fn drop_failed_progress(&mut self) -> bool {
        match self.progress {
            Some(Progress { state: ProgressState::Error, .. }) => self.set_progress(None),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Progress, ProgressState, State};
    use crate::program_status::{ProgramStatus, Record, Report};

    fn p(state: ProgressState, percent: u8) -> Option<Progress> {
        Some(Progress { state, percent })
    }

    fn working() -> Report {
        Report::Set {
            id: None,
            record: Record {
                state: State::Working,
                kind: None,
                progress: None,
                app: None,
                title: None,
                msg: None,
            },
        }
    }

    #[test]
    fn states_map_to_working_with_or_without_a_percentage_and_to_failure() {
        let mut status = ProgramStatus::default();
        assert!(status.summary().is_none());
        assert!(status.set_progress(p(ProgressState::Normal, 40)));
        let s = status.summary().expect("summary");
        assert_eq!((s.state, s.progress, s.paused), (State::Working, Some(40), false));
        status.set_progress(p(ProgressState::Paused, 40));
        let s = status.summary().expect("summary");
        assert_eq!((s.state, s.progress, s.paused), (State::Working, Some(40), true));
        status.set_progress(p(ProgressState::Indeterminate, 0));
        let s = status.summary().expect("summary");
        assert_eq!((s.state, s.progress, s.paused), (State::Working, None, false));
        status.set_progress(p(ProgressState::Error, 60));
        assert_eq!(status.summary().expect("summary").state, State::Error);
    }

    #[test]
    fn a_percentage_over_100_is_held_at_100_and_a_repeat_changes_nothing() {
        let mut status = ProgramStatus::default();
        assert!(status.set_progress(p(ProgressState::Normal, 250)));
        assert_eq!(status.progress(), p(ProgressState::Normal, 100));
        assert!(!status.set_progress(p(ProgressState::Normal, 100)));
        assert!(status.set_progress(None));
        assert!(!status.set_progress(None));
        assert!(status.summary().is_none());
    }

    #[test]
    fn a_7501_record_wins_while_it_is_held_and_progress_shows_again_after() {
        let mut status = ProgramStatus::default();
        status.set_progress(p(ProgressState::Normal, 70));
        status.apply(working());
        let s = status.summary().expect("summary");
        assert_eq!(s.progress, None, "the record's own, not the 9;4 percentage");
        status.apply(Report::Clear { id: None });
        assert_eq!(status.summary().expect("summary").progress, Some(70));
        assert_eq!(status.len(), 0, "the 9;4 progress is not a record");
    }

    #[test]
    fn the_prompt_and_the_exit_drop_running_progress_but_a_failure_waits_to_be_seen() {
        let mut status = ProgramStatus::default();
        status.set_progress(p(ProgressState::Paused, 10));
        assert!(status.on_prompt());
        assert!(status.progress().is_none());
        status.set_progress(p(ProgressState::Error, 10));
        assert!(!status.on_process_exit());
        assert!(status.progress().is_some());
        assert!(status.acknowledge());
        assert!(status.progress().is_none());
        assert!(!status.acknowledge());
    }

    #[test]
    fn a_full_reset_drops_it_but_a_7501_clear_does_not() {
        let mut status = ProgramStatus::default();
        status.set_progress(p(ProgressState::Normal, 5));
        assert!(!status.apply(Report::Clear { id: None }));
        assert!(status.progress().is_some());
        assert!(status.reset());
        assert!(status.progress().is_none());
    }
}
