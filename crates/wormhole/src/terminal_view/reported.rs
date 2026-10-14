//! What programs report about themselves (OSC 7501, `wormhole_core::terminal::program_status`) as
//! the terminal's status.
//!
//! The terminal's PTY thread keeps the records and says when they changed
//! (`PtyEvent::ProgramStatus`); this reads the one record that needs you most (`Reported`) and
//! turns it into a status. While any program holds a record, the status is the program's: it is
//! exact, where the output heuristics only guess, so those stand aside ([`TerminalView::reported`]).
//! An agent's hooks report the same way and whichever spoke last wins.

use super::*;
use wormhole_core::terminal::program_status::{Kind, State};

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
        use wormhole_core::terminal::program_status::{Parsed, parse};
        if let (Some(Parsed::Report(report)), Some(pty)) = (parse(body.as_bytes(), 2), &self.pty) {
            pty.program_status().apply(report);
        }
        self.handle_event(PtyEvent::ProgramStatus, cx);
    }

    /// A shell prompt began (OSC 133 `A`): the PTY thread drops what only the running program could
    /// say, the UI is told, and then the mark itself reaches the UI.
    pub(crate) fn prompt_for_test(&mut self, cx: &mut Context<Self>) {
        use wormhole_core::terminal::PromptKind;
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
/// own words or a short default for what it waits for; working is busy, and done is done. There is
/// no status for a failure (yet): an error is a finished result for the user to look at, like done,
/// with what went wrong as its words.
pub(super) fn status_of(summary: &Reported) -> (Status, Option<String>) {
    let message = summary.message();
    match summary.state {
        State::Blocked => {
            (Status::Attention, Some(message.unwrap_or_else(|| waiting_for(summary.kind).to_string())))
        }
        State::Error | State::Done => (Status::Done, message),
        State::Working => (Status::Busy, message),
        State::Idle => (Status::Idle, None),
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
    use wormhole_core::terminal::program_status::{Kind, State, Summary};

    use super::super::Status;
    use super::{status_of, waiting_for};

    fn summary(state: State, kind: Option<Kind>, msg: Option<&str>) -> Summary {
        Summary { state, kind, progress: None, app: None, title: None, msg: msg.map(str::to_string) }
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
        // No Error status: a failure is a finished result, with its words.
        assert_eq!(of(State::Error, Some("rsync failed")), (Status::Done, Some("rsync failed".into())));
        assert_eq!(of(State::Error, None), (Status::Done, None));
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
