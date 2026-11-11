//! A terminal's status (idle, working, needs input, finished, failed, exited), derived from what
//! happens to it: its output, the shell's marks, the bell, its agent's hooks, what programs report
//! about themselves, who holds its process and how it ended. These are the rules of the app's
//! `terminal_view/status.rs`, `settle.rs`, `reported.rs` and `failed.rs`, as a step function over
//! events with the clock passed in: [`StatusMachine::step`] takes a [`StatusEvent`] and the moment it
//! happened and changes the terminal's [`TerminalFacts`]. No clock, thread, process or I/O: what only a
//! shell can do comes back as an [`Ask`], and a deadline the machine needs (a busy terminal's output
//! going quiet, a title waiting its turn) as [`StatusMachine::next_due`], for the shell to arm one
//! timer for. The model keeps one machine per terminal (`WorkspaceModel::feed`).
//!
//! Layers inside: `rules` (pure decisions), `event`, then one file per concern as an `impl` of the
//! machine: `hooks`, `marks`, `reports`, `front` and `settle`.

mod book;
mod event;
mod front;
mod hooks;
mod marks;
mod reports;
mod rules;
mod settle;

use wormhole_term::program_status::{State, Summary};

use crate::agents::integrations::Wakes;
use crate::view::sidebar::command::{OutputFacts, has_new_output};
use crate::view::sidebar::{Status, names};

pub(in crate::workspace) use book::StatusBook;
pub use event::{Ask, Context, ForegroundLook, Front, StatusEvent};
pub use reports::status_of;

use super::facts::{Millis, ProgramNotice, TerminalFacts};
use rules::{Answered, TitleGate, on_bell};

/// Keep `title` and `body` as the terminal's latest notification, numbered after the last one.
fn note_notice(facts: &mut TerminalFacts, title: String, body: String) {
    let serial = facts.notice.as_ref().map_or(0, |notice| notice.serial) + 1;
    facts.notice = Some(ProgramNotice { serial, title, body });
}

/// One terminal's status rules and what they remember between events. The status itself, its
/// message, agent and the other things a row shows live in the terminal's `TerminalFacts`; this holds
/// what only the rules need.
#[derive(Debug)]
pub struct StatusMachine {
    /// An agent's hooks tell the status (until the agent is gone): the output heuristics stand aside.
    hook_driven: bool,
    /// The status is what a program reported last (not a hook, not a guess).
    program_driven: bool,
    /// The record of the program that reports, if one does.
    program: Option<Summary>,
    /// Background work that wakes the agent again: a turn that ends while it runs isn't finished.
    wakes: Wakes,
    /// The shell's own mark: at its prompt, or not; `None` before it has marked anything.
    at_prompt: Option<bool>,
    /// The shell said a command finished: the status settles at once.
    finished_mark: bool,
    last_output: Millis,
    bell_counted: Option<Millis>,
    answered: Answered,
    /// A program other than the shell held the terminal when last looked at.
    program_in_front: bool,
    foreground: Option<Front>,
    /// What was in front before the shell got the terminal back.
    ended_command: Option<String>,
    /// How many commands have failed here: each failure's serial.
    failures: u64,
    title: TitleGate,
}

impl StatusMachine {
    /// A terminal as it is when it opens at `now`. A shell tells it so with a `Resized` (the terminal was
    /// sized): what it prints in its first moments is the shell starting, not work.
    pub fn new(now: Millis) -> Self {
        Self {
            hook_driven: false,
            program_driven: false,
            program: None,
            wakes: Wakes::default(),
            at_prompt: None,
            finished_mark: false,
            last_output: now,
            bell_counted: None,
            answered: Answered::default(),
            program_in_front: false,
            foreground: None,
            ended_command: None,
            failures: 0,
            title: TitleGate::default(),
        }
    }

    /// What `event`, which happened at `at`, does to the terminal. The facts change in place; what the
    /// shell must do besides comes back.
    pub fn step(
        &mut self,
        facts: &mut TerminalFacts,
        event: StatusEvent,
        at: Millis,
        cx: Context,
    ) -> Option<Ask> {
        let mut ask = None;
        match event {
            StatusEvent::Output => self.on_output(facts, at, cx),
            StatusEvent::Title(raw) => self.offer_title(facts, names::title_name(&raw), at),
            StatusEvent::ResetTitle => self.offer_title(facts, None, at),
            StatusEvent::Bell => self.on_bell(facts, at),
            StatusEvent::Notice { title, body } => self.on_notice(facts, title, body, at),
            StatusEvent::ControlNotice { title, body } => note_notice(facts, title, body),
            StatusEvent::Mark { mark, ended } => self.on_mark(facts, mark, ended, at),
            StatusEvent::Reported { summary, by_report } => {
                self.apply_reported(facts, summary, by_report, at)
            }
            StatusEvent::Hook { agent, event, message } => {
                ask = self.on_hook(facts, agent, &event, message, at, cx)
            }
            StatusEvent::Foreground(look) => ask = self.on_foreground(facts, look, at),
            StatusEvent::Exited => self.on_exit(facts, at),
            StatusEvent::Typed => self.answered.input = Some(at),
            StatusEvent::Key { key, control } => {
                self.answered.input = Some(at);
                self.answer(facts, &key, control, at);
            }
            StatusEvent::Resized => self.answered.resize = Some(at),
            StatusEvent::FocusReported => self.answered.focus = Some(at),
            StatusEvent::Seen => ask = self.acknowledge(facts, at),
            StatusEvent::DoneUnseen => {
                if facts.status == Status::Idle {
                    self.set_status(facts, Status::Done, at);
                }
            }
            StatusEvent::Tick => {
                self.settle(facts, at, cx);
                if let Some(title) = self.title.take_due(at) {
                    facts.title = title;
                }
            }
        }
        // A command's end settles at once, whatever timer the output has still to wait for.
        if self.finished_mark {
            self.settle(facts, at, cx);
        }
        self.sync(facts);
        ask
    }

    /// The moment the machine wants a [`StatusEvent::Tick`]: a busy terminal's output has been quiet
    /// long enough, or a title is due. `None` when nothing waits, so an idle window arms no timer.
    pub fn next_due(&self, facts: &TerminalFacts) -> Option<Millis> {
        [self.settle_at(facts), self.title.due_at()].into_iter().flatten().min()
    }

    /// The name of the program that reports its status (`facts.source()` is `Program`): what it calls itself,
    /// else the command it runs. `None` for any other terminal, and where neither is known.
    pub fn program_name(&self, facts: &TerminalFacts) -> Option<String> {
        if facts.source() != crate::notify::Source::Program {
            return None;
        }
        if let Some(app) = self.program.as_ref().and_then(|program| program.app.clone()) {
            return Some(app);
        }
        let command = facts.command.as_deref().or(facts.finished_command.as_deref())?;
        Some(crate::view::sidebar::names::command_name(command)).filter(|name| !name.is_empty())
    }

    /// Whether the status is told, not guessed: an agent's hooks or a program's own reports say what
    /// it is, so the output heuristics (activity, the bell, notifications, prompt marks) leave it alone.
    pub(super) fn reported(&self) -> bool {
        self.hook_driven || self.program.is_some()
    }

    /// The program is blocked on you, and looking doesn't change that: it stays until the program
    /// moves on, like an agent's question.
    pub(super) fn blocked_by_program(&self) -> bool {
        self.program.as_ref().is_some_and(|program| program.state == State::Blocked)
    }

    pub(super) fn set_status(&mut self, facts: &mut TerminalFacts, status: Status, at: Millis) {
        if status != facts.status {
            facts.status = status;
            facts.since = at;
            // Seen, or at work again: the finished command is history.
            if !matches!(status, Status::Done | Status::Failed) {
                self.ended_command = None;
            }
        }
    }

    /// Facts that are the machine's own reading of what it remembers.
    fn sync(&self, facts: &mut TerminalFacts) {
        facts.reports = self.program_driven;
        facts.command = self.foreground.as_ref().map(|front| front.command.clone());
        facts.running_since = self.foreground.as_ref().map(|front| front.since);
        facts.finished_command = self
            .ended_command
            .clone()
            .filter(|_| self.foreground.is_none() && matches!(facts.status, Status::Done | Status::Failed));
    }

    fn offer_title(&mut self, facts: &mut TerminalFacts, title: Option<String>, at: Millis) {
        self.title.offer(title);
        if let Some(title) = self.title.take_due(at) {
            facts.title = title;
        }
    }

    /// The program printed. Ignore what answers us, and whatever a shell with integration prints at
    /// its prompt.
    fn on_output(&mut self, facts: &mut TerminalFacts, at: Millis, cx: Context) {
        let quiet = self.reported() || self.at_prompt == Some(true) || self.answered.covers(at);
        if !quiet && facts.status != Status::Exited {
            self.last_output = at;
            if facts.status != Status::Attention {
                self.set_status(facts, Status::Busy, at);
            }
        }
        facts.unread |= has_new_output(OutputFacts {
            agent: facts.agent.is_some(),
            selected: cx.on_screen,
            program_running: facts.command.is_some(),
            unseen: true,
        });
    }

    fn on_bell(&mut self, facts: &mut TerminalFacts, at: Millis) {
        let since = self.bell_counted.map(|counted| at.saturating_sub(counted));
        let bell = on_bell(facts.status, self.reported(), facts.agent.is_some(), since);
        facts.bell |= bell.mark;
        if bell.needs_you {
            self.bell_counted = Some(at);
            self.set_status(facts, Status::Attention, at);
        }
    }

    /// A program's own notification is the bell with words: the bell's rules say whether it counts
    /// as needing you (an agent's hooks already say when), and the notifications tell it in the
    /// program's words.
    fn on_notice(&mut self, facts: &mut TerminalFacts, title: String, body: String, at: Millis) {
        if self.reported() || facts.status == Status::Exited {
            return;
        }
        self.on_bell(facts, at);
        note_notice(facts, title, body);
    }

    fn on_exit(&mut self, facts: &mut TerminalFacts, at: Millis) {
        // The screen stays: the last output (often why it exited) can still be read.
        self.set_status(facts, Status::Exited, at);
        self.foreground = None;
    }
}
