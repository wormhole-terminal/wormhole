//! A registered terminal's events as what the model's status machine is told: the program's output,
//! title, bell and notices, the shell's marks, what programs report about themselves, who holds the
//! terminal's process, and what the person did to it. The rules that turn them into a row's state
//! are core's (`wormhole_core::workspace`'s `StatusMachine`); this file only names events and does
//! the I/O the machine asks for. It runs on the terminal's own event thread, so the model's lock is
//! never held while the terminal is asked anything.

use std::sync::{Arc, Weak};
use std::time::SystemTime;

use wormhole_core::agents::{AGENTS, AgentId};
use wormhole_core::workspace::{ForegroundLook, Front, StatusEvent};
use wormhole_term::{Mark, PtyEvent};

use crate::events::{EventFeed, Touch};
use crate::terminal::Terminal;
use crate::workspace::Shared;
use crate::workspace_pump::{AFTER_MARK, AFTER_OUTPUT};

struct TerminalFeed {
    shared: Weak<Shared>,
    id: i64,
    terminal: Weak<Terminal>,
}

/// Start feeding the model from `terminal`: what is known now at once, the rest as events come.
pub(crate) fn attach(shared: &Arc<Shared>, id: i64, terminal: &Arc<Terminal>) {
    let feed =
        Arc::new(TerminalFeed { shared: Arc::downgrade(shared), id, terminal: Arc::downgrade(terminal) });
    terminal.set_feed(feed);
    let shell = terminal.name().to_string();
    let events = shared.with_model(|model| model.update_facts(id, |facts| facts.shell = shell));
    shared.post(events);
    // It was just sized: what it prints now is its shell starting. Then who holds it and where it is.
    shared.pump.born(id, shared.now());
    shared.feed_quiet(id, StatusEvent::Resized);
    shared.pump.probe_by(id, 0);
}

/// The agent a hook names by its command ("claude").
pub(crate) fn agent_by_key(key: &str) -> Option<AgentId> {
    AGENTS.iter().find(|agent| agent.command == key).map(|agent| agent.id)
}

impl EventFeed for TerminalFeed {
    fn on_event(&self, event: &PtyEvent) {
        let Some(shared) = self.shared.upgrade() else { return };
        let id = self.id;
        match event {
            PtyEvent::Output => {
                shared.feed_quiet(id, StatusEvent::Output);
                shared.pump.probe_by(id, shared.now() + AFTER_OUTPUT);
            }
            PtyEvent::Title(title) => shared.feed(id, StatusEvent::Title(title.clone())),
            PtyEvent::ResetTitle => shared.feed(id, StatusEvent::ResetTitle),
            // The shell said where it is: no need to wait for the next look.
            PtyEvent::Cwd => shared.pump.probe_by(id, 0),
            PtyEvent::Bell => shared.feed(id, StatusEvent::Bell),
            PtyEvent::Notify(_) => shared.feed(id, StatusEvent::Notice),
            PtyEvent::Mark(mark, _) => {
                let ended = match *mark {
                    Mark::Finished(Some(code)) => self.terminal.upgrade().and_then(|t| t.ended_command(code)),
                    _ => None,
                };
                shared.feed(id, StatusEvent::Mark { mark: *mark, ended });
                shared.pump.probe_by(id, shared.now() + AFTER_MARK);
            }
            PtyEvent::ProgramStatus => {
                let summary = self.terminal.upgrade().and_then(|t| t.program_report(None));
                shared.feed(id, StatusEvent::Reported { summary, by_report: true });
            }
            PtyEvent::Exited(_) => shared.feed(id, StatusEvent::Exited),
            _ => {}
        }
    }

    fn on_touch(&self, touch: Touch) {
        let Some(shared) = self.shared.upgrade() else { return };
        let event = match touch {
            Touch::Typed => StatusEvent::Typed,
            Touch::Key { key, control } => StatusEvent::Key { key, control },
            Touch::Resized => StatusEvent::Resized,
        };
        shared.feed_quiet(self.id, event);
    }
}

impl Shared {
    /// Tell terminal `id`'s status machine, and the shell's side of what follows: events for Swift,
    /// what the machine asks of the terminal, and the pump, whose deadlines may have moved.
    pub(crate) fn feed(&self, id: i64, event: StatusEvent) {
        let at = self.now();
        let events = self.with_model(|model| model.feed(id, event, at));
        self.post(events);
        self.serve_asks();
        self.pump.kick();
    }

    /// [`feed`](Self::feed) for what comes often (output, typing): the pump is woken only when a row
    /// changed, since what happens next is then in the model's deadlines.
    pub(crate) fn feed_quiet(&self, id: i64, event: StatusEvent) {
        let at = self.now();
        let events = self.with_model(|model| model.feed(id, event, at));
        let changed = !events.is_empty();
        self.post(events);
        self.serve_asks();
        if changed {
            self.pump.kick();
        }
    }

    /// Do what the machines asked of the terminals' records, and hand the records back.
    pub(crate) fn serve_asks(&self) {
        for (id, ask) in self.with_model(|model| model.take_asks()) {
            let Some(terminal) = self.terminal_of(id) else { continue };
            let summary = terminal.program_report(Some(ask));
            let at = self.now();
            let events = self
                .with_model(|model| model.feed(id, StatusEvent::Reported { summary, by_report: false }, at));
            self.post(events);
        }
    }

    /// Look at terminal `id`'s process: where it is, who holds it, which agent runs. Whether to look
    /// again later (the program is still there).
    pub(crate) fn probe(&self, id: i64) -> bool {
        let Some(terminal) = self.terminal_of(id) else { return false };
        if !terminal.is_alive() {
            return false;
        }
        let seen = terminal.probe();
        let look = ForegroundLook {
            in_front: seen.in_front,
            command: seen
                .command
                .map(|front| Front { command: front.command, since: self.since(front.started) }),
            agent: seen.agent,
            shell_in_front: seen.shell_in_front,
        };
        let at = self.now();
        let events = self.with_model(|model| {
            let mut events = model.update_facts(id, |facts| facts.cwd = seen.cwd.or(facts.cwd.take()));
            events.extend(model.feed(id, StatusEvent::Foreground(look), at));
            events
        });
        self.post(events);
        self.serve_asks();
        self.pump.kick();
        true
    }

    /// A moment of the system's clock on the workspace's: a process that started before the
    /// workspace did is taken to have started with it.
    fn since(&self, at: SystemTime) -> u64 {
        at.duration_since(self.epoch).map_or(0, |ago| u64::try_from(ago.as_millis()).unwrap_or(u64::MAX))
    }
}
