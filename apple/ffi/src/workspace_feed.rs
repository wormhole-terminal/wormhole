//! A registered terminal's events as facts for the model: what the sidebar row needs of it, learned
//! on the terminal's own event thread. The row says what the terminal says: the title its program
//! set, the folder the shell reported, the command in front, the bell and the output nobody saw, a
//! command that failed. (A coding agent's status comes from its hooks and the model's status
//! machine, which join the model in a later phase.)

use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::SystemTime;

use wormhole_core::view::sidebar::Status;
use wormhole_core::view::sidebar::command::{OutputFacts, has_new_output};
use wormhole_core::view::sidebar::names::title_name;
use wormhole_core::workspace::{Failure, TerminalFacts};
use wormhole_term::{Mark, PtyEvent};

use crate::events::EventFeed;
use crate::terminal::Terminal;
use crate::workspace::Shared;

struct TerminalFeed {
    shared: Weak<Shared>,
    id: i64,
    terminal: Weak<Terminal>,
    /// The command that started last, for the failure that names it.
    last_command: Mutex<Option<String>>,
}

/// Start feeding the model from `terminal`: what is known now at once, the rest as events come.
pub(crate) fn attach(shared: &Arc<Shared>, id: i64, terminal: &Arc<Terminal>) {
    let feed = Arc::new(TerminalFeed {
        shared: Arc::downgrade(shared),
        id,
        terminal: Arc::downgrade(terminal),
        last_command: Mutex::new(None),
    });
    terminal.set_feed(feed.clone());
    let name = terminal.name().to_string();
    feed.learn(move |facts, probe| {
        facts.shell = name;
        probe.apply_to(facts);
    });
}

/// What the terminal's process says right now (cheap system calls, no model lock held).
struct Probe {
    cwd: Option<std::path::PathBuf>,
    agent: Option<wormhole_core::agents::AgentId>,
    command: Option<(String, SystemTime)>,
}

impl Probe {
    fn of(terminal: &Terminal) -> Self {
        let seen = terminal.probe();
        Self {
            cwd: seen.cwd,
            agent: seen.agent,
            command: seen.command.map(|foreground| (foreground.command, foreground.started)),
        }
    }

    fn apply_to(&self, facts: &mut TerminalFacts) {
        facts.cwd = self.cwd.clone().or(facts.cwd.take());
        facts.agent = self.agent;
    }
}

impl TerminalFeed {
    /// Change the facts with what the process says, under the model's lock, and tell Swift if a row
    /// shows it. The probe is taken first: the model's lock is never held across a call into the
    /// terminal.
    fn learn(&self, change: impl FnOnce(&mut TerminalFacts, &Probe)) {
        let (Some(shared), Some(terminal)) = (self.shared.upgrade(), self.terminal.upgrade()) else { return };
        let probe = Probe::of(&terminal);
        let events = shared.with_model(|model| model.update_facts(self.id, |facts| change(facts, &probe)));
        shared.post(events);
    }

    fn selected(shared: &Shared, id: i64) -> bool {
        shared.with_model(|model| model.selected()) == Some(id)
    }
}

impl EventFeed for TerminalFeed {
    fn on_event(&self, event: &PtyEvent) {
        let Some(shared) = self.shared.upgrade() else { return };
        let id = self.id;
        let selected = || Self::selected(&shared, id);
        match event {
            PtyEvent::Title(title) => {
                let title = title_name(title);
                self.learn(|facts, _| facts.title = title);
            }
            PtyEvent::ResetTitle => self.learn(|facts, _| facts.title = None),
            PtyEvent::Cwd => self.learn(|facts, probe| probe.apply_to(facts)),
            PtyEvent::Bell if !selected() => self.learn(|facts, _| {
                facts.bell = true;
                if !facts.told() {
                    facts.status = Status::Attention;
                }
            }),
            PtyEvent::Output if !selected() => self.learn(|facts, _| {
                facts.unread |= has_new_output(OutputFacts {
                    agent: facts.agent.is_some(),
                    selected: false,
                    program_running: facts.command.is_some(),
                    unseen: true,
                });
            }),
            PtyEvent::Mark(mark, _) => self.on_mark(*mark, &shared),
            PtyEvent::Exited(_) => self.learn(|facts, _| {
                facts.status = Status::Exited;
                facts.command = None;
                facts.running_since = None;
            }),
            _ => {}
        }
    }
}

impl TerminalFeed {
    /// The shell's marks: a command starts running (the program in front is looked up now), or ends
    /// (and with a failing exit code the row says how).
    fn on_mark(&self, mark: Mark, shared: &Shared) {
        let now = shared.now();
        match mark {
            Mark::OutputStart => {
                let Some(terminal) = self.terminal.upgrade() else { return };
                let command = Probe::of(&terminal).command;
                *self.last_command.lock().unwrap_or_else(PoisonError::into_inner) =
                    command.as_ref().map(|(text, _)| text.clone());
                self.learn(|facts, probe| {
                    probe.apply_to(facts);
                    facts.command = command.as_ref().map(|(text, _)| text.clone());
                    facts.running_since = command.as_ref().map(|(_, started)| since(shared, started));
                    facts.failure = None;
                    if facts.status == Status::Failed {
                        facts.status = Status::Idle;
                    }
                });
            }
            Mark::Finished(code) => {
                let named = self.last_command.lock().unwrap_or_else(PoisonError::into_inner).take();
                let failed = code.filter(|&code| code != 0);
                let selected = Self::selected(shared, self.id);
                self.learn(|facts, probe| {
                    probe.apply_to(facts);
                    facts.command = None;
                    facts.running_since = None;
                    facts.failure = failed.map(|code| Failure { code, command: named, at: now });
                    if failed.is_some() && !selected && !facts.told() {
                        facts.status = Status::Failed;
                    }
                });
            }
            Mark::PromptStart(_) | Mark::CommandStart => {}
        }
    }
}

/// When a process started, on the workspace's clock.
fn since(shared: &Shared, started: &SystemTime) -> u64 {
    let ago = SystemTime::now().duration_since(*started).unwrap_or_default();
    shared.now().saturating_sub(u64::try_from(ago.as_millis()).unwrap_or(u64::MAX))
}
