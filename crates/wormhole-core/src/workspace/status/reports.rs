//! What programs report about themselves (OSC 7501) as the terminal's status.
//!
//! The terminal's PTY thread keeps the records and says when they changed; the shell reads the one
//! that needs input most (`Summary`) and hands it over. While any program holds a record, the status
//! is the program's: it is exact, where the output heuristics only guess, so those stand aside. An
//! agent's hooks report the same way and whichever spoke last wins.

use wormhole_term::ProgressState;
use wormhole_term::program_status::{Kind, State, Summary};
use wormhole_text::{keys, text};

use crate::view::sidebar::Status;

use super::super::facts::{Millis, TerminalFacts};
use super::StatusMachine;

impl StatusMachine {
    /// Derive the status from what the programs report. `by_report`: the records changed because a
    /// program said something, which makes the status theirs even where an agent's hooks spoke before.
    pub(super) fn apply_reported(
        &mut self,
        facts: &mut TerminalFacts,
        summary: Option<Summary>,
        by_report: bool,
        at: Millis,
    ) {
        let derived = summary.as_ref().map(|summary| (status_of(summary), summary.agent()));
        // A progress bar (OSC 9;4) says how far along work is, not what the agent in front is
        // doing: a hook's word on it (a question waiting, say) is not taken over by a later bar.
        let bar_under_hooks =
            self.hook_driven && summary.as_ref().is_some_and(|summary| summary.bar.is_some());
        self.program = summary;
        if facts.status == Status::Exited {
            return;
        }
        match derived {
            Some(_) if bar_under_hooks => {}
            Some(((status, message), agent)) => {
                self.program_driven |= by_report;
                if self.program_driven {
                    facts.message = message;
                    self.set_status(facts, status, at);
                    // The agent it says it is, when nothing detected one (it may run over ssh).
                    if facts.agent.is_none() {
                        facts.agent = agent;
                    }
                }
            }
            // Nothing is reported any more (cleared, or dropped with the program or its prompt): the
            // status the reports set goes with it, and the heuristics take over. A status that hooks
            // set is not theirs to reset, so an empty set of records that never drove it changes nothing.
            None if self.program_driven => {
                self.program_driven = false;
                facts.message = None;
                self.set_status(facts, Status::Idle, at);
            }
            None => {}
        }
    }
}

/// The status and the words a program's record amounts to. A blocked program needs input, with its
/// own words or a short default for what it waits for; working is busy, done is done, and an error
/// is failed, with what went wrong as its words. How far along a working or blocked program says it
/// is follows its words ("Installing · 40%"); a result has no progress to show. An OSC 9;4 bar is
/// work whatever its state: paused and error say so in their words and are never a result.
pub fn status_of(summary: &Summary) -> (Status, Option<String>) {
    let message = summary.message();
    match summary.state {
        State::Blocked => {
            let words = message.unwrap_or_else(|| waiting_for(summary.kind).to_string());
            (Status::Attention, Some(with_progress(words, summary.progress)))
        }
        // A bar in its error state (OSC 9;4 state 2) is a state the program still holds, not an
        // outcome: busy, never failed, so nothing is announced and nothing waits to be looked at.
        State::Error if summary.bar.is_some() => {
            let words = text(keys::TERMINAL_PROGRESS_ERROR).to_string();
            (Status::Busy, Some(with_progress(words, summary.progress)))
        }
        State::Error => (Status::Failed, message),
        State::Done => (Status::Done, message),
        // A program that holds its progress (OSC 9;4 state 4) says so, since it isn't moving.
        State::Working if summary.bar == Some(ProgressState::Paused) => {
            let words = text(keys::TERMINAL_PROGRESS_PAUSED).to_string();
            (Status::Busy, Some(with_progress(words, summary.progress)))
        }
        // Without words of its own, only the percentage is worth saying: the row's "Working · 3m"
        // can't carry it.
        State::Working => match (message, summary.progress) {
            (None, Some(_)) => (Status::Busy, Some(with_progress("Working".to_string(), summary.progress))),
            (message, progress) => (Status::Busy, message.map(|words| with_progress(words, progress))),
        },
        State::Idle => (Status::Idle, None),
    }
}

/// `words` with how far along it is, if it says: "Installing updates · 40%".
fn with_progress(words: String, progress: Option<u8>) -> String {
    match progress {
        Some(percent) => format!("{words} · {percent}%"),
        None => words,
    }
}

/// What a blocked program with nothing to say for itself waits for.
fn waiting_for(kind: Option<Kind>) -> &'static str {
    // text: the words of a status a program reported without any; the catalogs take the app's side later.
    match kind {
        Some(Kind::Permission) => "Waiting for approval",
        Some(Kind::Question) => "Waiting for an answer",
        Some(Kind::Auth) => "Waiting for a login",
        None => "Waiting for you",
    }
}

#[cfg(test)]
mod tests {
    use wormhole_term::ProgressState;
    use wormhole_term::program_status::{Kind, State, Summary};

    use crate::view::sidebar::Status;

    use super::{status_of, waiting_for};

    fn summary(state: State, kind: Option<Kind>, msg: Option<&str>) -> Summary {
        Summary {
            state,
            kind,
            progress: None,
            app: None,
            title: None,
            msg: msg.map(str::to_string),
            bar: None,
        }
    }

    fn with_progress(state: State, progress: u8, msg: Option<&str>) -> Summary {
        Summary { progress: Some(progress), ..summary(state, None, msg) }
    }

    #[test]
    fn a_blocked_program_needs_you_in_its_own_words_or_a_default() {
        let blocked = |kind, msg| status_of(&summary(State::Blocked, kind, msg));
        assert_eq!(
            blocked(Some(Kind::Permission), Some("Apply 3 to add?")),
            (Status::Attention, Some("Apply 3 to add?".to_string()))
        );
        for (kind, words) in [
            (Some(Kind::Permission), "Waiting for approval"),
            (Some(Kind::Question), "Waiting for an answer"),
            (Some(Kind::Auth), "Waiting for a login"),
            (None, "Waiting for you"),
        ] {
            assert_eq!(blocked(kind, None), (Status::Attention, Some(words.to_string())), "{kind:?}");
            assert_eq!(waiting_for(kind), words);
        }
        // A message with nothing left to show once cleaned is no message.
        assert_eq!(blocked(None, Some("\u{202e}  ")), (Status::Attention, Some("Waiting for you".into())));
    }

    #[test]
    fn the_other_states_map_to_the_status_that_says_them() {
        let of = |state, msg| status_of(&summary(state, None, msg));
        assert_eq!(
            of(State::Working, Some("Installing updates")),
            (Status::Busy, Some("Installing updates".into()))
        );
        assert_eq!(of(State::Working, None), (Status::Busy, None));
        assert_eq!(
            of(State::Done, Some("Upgraded 12 packages")),
            (Status::Done, Some("Upgraded 12 packages".into()))
        );
        assert_eq!(of(State::Idle, Some("ignored")), (Status::Idle, None));
        assert_eq!(of(State::Error, Some("rsync failed")), (Status::Failed, Some("rsync failed".into())));
        assert_eq!(of(State::Error, None), (Status::Failed, None));
    }

    #[test]
    fn progress_follows_the_words_of_a_program_that_works_or_waits() {
        let of = |summary: Summary| status_of(&summary);
        assert_eq!(
            of(with_progress(State::Working, 40, Some("Installing updates"))),
            (Status::Busy, Some("Installing updates · 40%".into()))
        );
        assert_eq!(of(with_progress(State::Working, 7, None)), (Status::Busy, Some("Working · 7%".into())));
        assert_eq!(
            of(with_progress(State::Blocked, 90, None)),
            (Status::Attention, Some("Waiting for you · 90%".into()))
        );
        // A result has finished: no progress to show, even if the report carried one.
        assert_eq!(of(with_progress(State::Done, 100, Some("Done"))), (Status::Done, Some("Done".into())));
        assert_eq!(of(with_progress(State::Idle, 5, None)), (Status::Idle, None));
    }

    #[test]
    fn a_paused_or_errored_bar_is_work_that_says_so_never_a_result() {
        let bar = |state, progress, bar| Summary { progress, bar: Some(bar), ..summary(state, None, None) };
        assert_eq!(
            status_of(&bar(State::Working, Some(40), ProgressState::Paused)),
            (Status::Busy, Some("Paused · 40%".into()))
        );
        assert_eq!(
            status_of(&bar(State::Error, None, ProgressState::Error)),
            (Status::Busy, Some("Error".into()))
        );
    }

    #[test]
    fn what_a_program_says_is_cleaned_before_it_is_shown() {
        let (_, message) =
            status_of(&summary(State::Working, None, Some("Pushing\u{202e}   the image\u{200b}")));
        assert_eq!(message.as_deref(), Some("Pushing the image"));
        let (_, message) = status_of(&summary(State::Working, None, Some(&"x".repeat(500))));
        assert_eq!(message.map(|m| m.chars().count()), Some(201));
    }
}
