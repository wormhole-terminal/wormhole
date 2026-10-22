//! What programs report about themselves (OSC 7501, `wormhole_term::program_status`) as
//! the terminal's status.
//!
//! The terminal's PTY thread keeps the records and says when they changed
//! (`PtyEvent::ProgramStatus`); this reads the one record that needs you most (`Reported`) and
//! turns it into a status. While any program holds a record, the status is the program's: it is
//! exact, where the output heuristics only guess, so those stand aside ([`TerminalView::reported`]).
//! An agent's hooks report the same way and whichever spoke last wins. Any program may report, not
//! only the coding agents: one that is none (`brew`, a deploy script) is a `Source::Program` and
//! shows in the row, the bar and the notifications like an agent does, under its own name.

#[cfg(test)]
use gpui_kit::*;
use wormhole_term::program_status::Summary as Reported;
use wormhole_term::program_status::{Kind, State};
#[cfg(test)]
use wormhole_term::{Mark, PtyEvent};

use super::TerminalView;
use super::status::Status;

impl TerminalView {
    /// Whether the status is told, not guessed: an agent's hooks or a program's own reports say
    /// what it is, so the output heuristics (activity, the bell, notifications, prompt marks)
    /// leave it alone.
    pub(super) fn reported(&self) -> bool {
        self.hook_driven || self.program.is_some()
    }

    /// The status is what a program reported last (not an agent's hook, not a guess). Anyone who
    /// can write to the terminal can say it, so what it causes outside the window is rate-limited.
    pub fn reported_by_program(&self) -> bool {
        self.program_driven
    }

    /// The program is blocked on you, and the user's looking doesn't change that: it stays until
    /// the program moves on, like an agent's question.
    pub(super) fn blocked_by_program(&self) -> bool {
        self.program.as_ref().is_some_and(|program| program.state == State::Blocked)
    }

    /// Read what the programs report again and derive the status from it. `by_report`: the
    /// records changed because a program said something (rather than a prompt, the program's exit
    /// or the user looking dropping some), which makes the status theirs even where an agent's hooks
    /// spoke before. Returns whether the status, its words or the agent changed.
    pub(super) fn sync_reported(&mut self, by_report: bool) -> bool {
        let summary = self.pty.as_ref().and_then(|pty| pty.program_status().summary());
        self.apply_reported(summary, by_report)
    }

    fn apply_reported(&mut self, summary: Option<Reported>, by_report: bool) -> bool {
        let before = (self.status, self.agent_message.clone(), self.agent);
        let derived = summary.as_ref().map(|summary| (status_of(summary), summary.agent()));
        self.program = summary;
        if self.status != Status::Exited {
            match derived {
                Some(((status, message), agent)) => {
                    self.program_driven |= by_report;
                    if self.program_driven {
                        self.agent_message = message;
                        self.set_status(status);
                        // The agent it says it is, when nothing detected one (it may run over ssh).
                        if self.agent.is_none() {
                            self.agent = agent;
                        }
                    }
                }
                // Nothing is reported any more (cleared, or dropped with the program or its
                // prompt): the status the reports set goes with it, and the heuristics take over.
                // A status that hooks set is not theirs to reset, so an empty set of records that
                // never drove the status changes nothing.
                None if self.program_driven => {
                    self.program_driven = false;
                    self.agent_message = None;
                    self.set_status(Status::Idle);
                }
                None => {}
            }
        }
        (self.status, &self.agent_message, self.agent) != (before.0, &before.1, before.2)
    }

    /// The program that was in front has left the terminal (the process exit of the protocol):
    /// working and blocked are not true any more, even for a program that crashed without saying
    /// so. Returns whether the status changed.
    pub(super) fn program_left(&mut self) -> bool {
        if self.program.is_none() {
            return false;
        }
        let Some(pty) = &self.pty else { return false };
        pty.program_status().on_process_exit();
        self.sync_reported(false)
    }

    /// The user looked: a program's finished or failed records have done their job. Dropped, so
    /// the next derivation doesn't bring "done" back; what is still going on stays.
    pub(super) fn acknowledge_reported(&mut self) {
        if let Some(pty) = &self.pty {
            pty.program_status().acknowledge();
        }
        self.sync_reported(false);
    }
}

/// What a test does in place of the shell and the PTY thread: they have no program to print
/// the sequences.
#[cfg(test)]
impl TerminalView {
    /// Feed one OSC 7501 body to the terminal's records as the PTY thread would, and handle the
    /// event that tells the UI.
    pub(crate) fn report_for_test(&mut self, body: &str, cx: &mut Context<Self>) {
        use wormhole_term::program_status::{Parsed, parse};
        if let (Some(Parsed::Report(report)), Some(pty)) = (parse(body.as_bytes(), 2), &self.pty) {
            pty.program_status().apply(report);
        }
        self.handle_event(PtyEvent::ProgramStatus, cx);
    }

    /// A shell prompt began (OSC 133 `A`): the PTY thread drops what only the running program could
    /// say, the UI is told, and then the mark itself reaches the UI.
    pub(crate) fn prompt_for_test(&mut self, cx: &mut Context<Self>) {
        use wormhole_term::PromptKind;
        if let Some(pty) = &self.pty {
            pty.program_status().on_prompt();
        }
        self.handle_event(PtyEvent::ProgramStatus, cx);
        self.handle_event(PtyEvent::Mark(Mark::PromptStart(PromptKind::Initial)), cx);
    }

    pub(crate) fn event_for_test(&mut self, event: PtyEvent, cx: &mut Context<Self>) {
        self.handle_event(event, cx);
    }

    /// The program in front is found to have left the terminal.
    pub(crate) fn left_for_test(&mut self) -> bool {
        self.program_left()
    }

    pub(crate) fn records_for_test(&self) -> usize {
        self.pty.as_ref().map_or(0, |pty| pty.program_status().len())
    }
}

/// The status and the words a program's record amounts to. A blocked program needs you, with its
/// own words or a short default for what it waits for; working is busy, done is done, and an error
/// is failed, with what went wrong as its words. How far along a working or blocked program says it
/// is follows its words ("Installing · 40%"); a result has no progress to show.
pub(super) fn status_of(summary: &Reported) -> (Status, Option<String>) {
    let message = summary.message();
    match summary.state {
        State::Blocked => {
            let words = message.unwrap_or_else(|| waiting_for(summary.kind).to_string());
            (Status::Attention, Some(with_progress(words, summary.progress)))
        }
        State::Error => (Status::Failed, message),
        State::Done => (Status::Done, message),
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
    match kind {
        Some(Kind::Permission) => "Waiting for approval",
        Some(Kind::Question) => "Waiting for an answer",
        Some(Kind::Auth) => "Waiting for a login",
        None => "Waiting for you",
    }
}

#[cfg(test)]
mod tests {
    use wormhole_term::program_status::{Kind, State, Summary};

    use super::super::Status;
    use super::{status_of, waiting_for};

    fn summary(state: State, kind: Option<Kind>, msg: Option<&str>) -> Summary {
        Summary { state, kind, progress: None, app: None, title: None, msg: msg.map(str::to_string) }
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
        // An error is a failure of its own, with its words.
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
        // No words: the percentage is all there is to say, and the row's own "Working" goes with it.
        assert_eq!(of(with_progress(State::Working, 7, None)), (Status::Busy, Some("Working · 7%".into())));
        assert_eq!(
            of(with_progress(State::Blocked, 90, Some("Apply 3 to add?"))),
            (Status::Attention, Some("Apply 3 to add? · 90%".into()))
        );
        assert_eq!(
            of(with_progress(State::Blocked, 90, None)),
            (Status::Attention, Some("Waiting for you · 90%".into()))
        );
        // A result has finished: no progress to show, even if the report carried one.
        assert_eq!(of(with_progress(State::Done, 100, Some("Done"))), (Status::Done, Some("Done".into())));
        assert_eq!(
            of(with_progress(State::Error, 60, Some("Failed"))),
            (Status::Failed, Some("Failed".into()))
        );
        assert_eq!(of(with_progress(State::Idle, 5, None)), (Status::Idle, None));
        assert_eq!(
            of(with_progress(State::Working, 0, Some("Starting"))).1.as_deref(),
            Some("Starting · 0%")
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
