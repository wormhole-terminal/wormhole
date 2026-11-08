//! What the control socket hears of the terminals, besides what Swift is told: the event bus's
//! publisher (`wh events`, `wh wait`). The workspace calls the [`Tap`] when its model may have moved and
//! when a terminal's program says something; nothing is built or read while nobody listens (one relaxed
//! load, as in the GPUI app's `automation`).
//!
//! State changes are found by comparing what `list` would say now with what it said last (`Publisher::diff`,
//! on a thread of its own, so the model's lock is never held while the bus is written to). Output and
//! bells come from the terminals' own events.

use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use wormhole_core::ipc::bus::Bus;
use wormhole_core::ipc::control::event::{BlockPhase, Body, Event, Topic};
use wormhole_core::ipc::control::{Listing, WindowInfo};
use wormhole_term::{BlockState, Mark, PtyEvent};

use super::views::{self, Views};
use crate::workspace::Shared;

/// What the workspace tells the server.
pub(crate) trait Tap: Send + Sync {
    /// The model may have changed: look when somebody listens.
    fn model_touched(&self);
    /// A registered terminal's program did something.
    fn terminal_event(&self, id: i64, event: &PtyEvent);
}

/// A command line is cut to this many characters in an event: it is a line of terminal text.
const COMMAND_CHARS: usize = 512;

/// The topics a model change can be told on.
const MODEL_TOPICS: [Topic; 5] =
    [Topic::Status, Topic::Title, Topic::Folder, Topic::Terminals, Topic::Splits];

pub(crate) struct Publisher {
    bus: Arc<Bus>,
    shared: Weak<Shared>,
    window: i64,
    kick: SyncSender<()>,
    /// What was last told; held while comparing, so the events leave in order.
    last: Mutex<Views>,
}

impl Publisher {
    /// The publisher for `shared`'s terminals, with its thread. `Shared` owns it as its tap.
    pub(crate) fn start(shared: &Arc<Shared>, bus: Arc<Bus>, window: i64) -> std::io::Result<Arc<Self>> {
        let (kick, wakes) = sync_channel(1);
        let last = shared.peek_model(|model| views::read(model, shared.now()));
        let publisher =
            Arc::new(Self { bus, shared: Arc::downgrade(shared), window, kick, last: Mutex::new(last) });
        let weak = Arc::downgrade(&publisher);
        std::thread::Builder::new().name("wormhole-hook-tap".into()).spawn(move || run(&wakes, &weak))?;
        Ok(publisher)
    }

    /// `list` as it reads now, and the baseline the next events are told against: one moment, so a stream
    /// that starts with this listing neither misses a change nor tells one the listing already shows.
    pub(crate) fn listing_and_rebase(&self) -> Listing {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(shared) = self.shared.upgrade() else { return Listing::default() };
        let now = shared.peek_model(|model| views::read(model, shared.now()));
        let listing = self.listing_of(&now);
        *last = now;
        listing
    }

    /// Look again soon: something the caller just did may have changed what is waited for.
    pub(crate) fn kick(&self) {
        let _ = self.kick.try_send(());
    }

    /// `list` as it reads now.
    pub(crate) fn listing(&self) -> Listing {
        let Some(shared) = self.shared.upgrade() else { return Listing::default() };
        self.listing_of(&shared.peek_model(|model| views::read(model, shared.now())))
    }

    fn listing_of(&self, views: &Views) -> Listing {
        Listing {
            windows: vec![WindowInfo {
                id: self.window,
                front: true,
                terminals: views.listed(),
                splits: views.listed_splits(),
            }],
        }
    }

    fn event(&self, terminal: Option<i64>, body: Body) -> Event {
        Event { window: Some(self.window), terminal, body }
    }

    /// Tell what changed since the last look.
    fn diff(&self) {
        let Some(shared) = self.shared.upgrade() else { return };
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        let now = shared.peek_model(|model| views::read(model, shared.now()));
        for seen in &now.terminals {
            let id = seen.info.id;
            let Some(old) = last.terminals.iter().find(|old| old.info.id == id) else {
                let folder = seen.info.folder.clone();
                self.bus.publish(Topic::Terminals, || self.event(Some(id), Body::TerminalOpened { folder }));
                continue;
            };
            if seen.hidden != old.hidden {
                self.bus.set_hidden(id, seen.hidden);
            }
            let (was, status) = (old.info.status, seen.info.status);
            if was != status {
                let (agent, message) = (seen.info.agent.clone(), seen.message());
                self.bus.publish(Topic::Status, || {
                    self.event(Some(id), Body::Status { status, was, agent, message })
                });
            }
            if old.title != seen.title || old.info.name != seen.info.name {
                let (title, name) = (seen.title.clone(), seen.info.name.clone());
                self.bus.publish(Topic::Title, || self.event(Some(id), Body::Title { title, name }));
            }
            if old.info.folder != seen.info.folder {
                let folder = seen.info.folder.clone();
                self.bus.publish(Topic::Folder, || self.event(Some(id), Body::Folder { folder }));
            }
        }
        for gone in last.terminals.iter().filter(|old| now.terminals.iter().all(|n| n.info.id != old.info.id))
        {
            let id = gone.info.id;
            self.bus.publish(Topic::Terminals, || self.event(Some(id), Body::TerminalClosed));
            self.bus.forget_terminal(id);
        }
        for split in &now.splits {
            if last.splits.iter().all(|old| old != split) {
                let (id, terminals, focused) = (split.id, split.terminals.clone(), Some(split.focused));
                self.bus.publish(Topic::Splits, || {
                    self.event(None, Body::Split { split: id, terminals, focused })
                });
            }
        }
        for old in last.splits.iter().filter(|old| now.splits.iter().all(|n| n.id != old.id)) {
            // Empty: the split is gone.
            let id = old.id;
            self.bus.publish(Topic::Splits, || {
                self.event(None, Body::Split { split: id, terminals: Vec::new(), focused: None })
            });
        }
        *last = now;
    }
}

fn run(wakes: &Receiver<()>, publisher: &Weak<Publisher>) {
    while wakes.recv().is_ok() {
        // What came while this ran is one more look, not one each.
        while !matches!(wakes.try_recv(), Err(TryRecvError::Empty | TryRecvError::Disconnected)) {}
        let Some(publisher) = publisher.upgrade() else { return };
        publisher.diff();
    }
}

impl Publisher {
    /// Terminal `id`'s shell marked a command's start or end: tell which command it was. The mark carries the
    /// number of its prompt, which is the block's id, so a mark heard late is still told about the right
    /// command; how it went comes from the shell's own log by that number, the line typed from the grid.
    fn block(&self, id: i64, mark: Mark, prompt: u64) {
        let (phase, exit_code) = match mark {
            Mark::OutputStart => (BlockPhase::Started, None),
            Mark::Finished(code) => (BlockPhase::Finished, code),
            Mark::PromptStart(_) | Mark::CommandStart => return,
        };
        let Some(terminal) = self.shared.upgrade().and_then(|shared| shared.terminal_of(id)) else { return };
        // Before any prompt (0) the log has nothing to say.
        let Some((state, found)) =
            terminal.with_pty(|pty| (pty.command_state(prompt), pty.block(prompt))).filter(|_| prompt > 0)
        else {
            return;
        };
        // An Enter on an empty line is marked as ending a block too, with no command in it: that is not a
        // command finishing, and the command before it already said so.
        if !state.ran() {
            return;
        }
        let took_ms = match (phase, state) {
            (BlockPhase::Finished, BlockState::Finished { took, .. }) => {
                took.map(|took| u64::try_from(took.as_millis()).unwrap_or(u64::MAX))
            }
            _ => None,
        };
        self.bus.publish(Topic::Blocks, || {
            let command =
                wormhole_term::clean(found.as_ref().map_or("", |block| &block.command), COMMAND_CHARS);
            // The rows of its output are known once it ended, from the marks on the grid.
            let output_lines =
                found.as_ref().filter(|_| phase == BlockPhase::Finished).map(|block| block.output_rows());
            let body = Body::Block {
                block: i64::try_from(prompt).unwrap_or(i64::MAX),
                phase,
                exit_code,
                took_ms,
                command: (!command.is_empty()).then_some(command),
                output_lines,
            };
            self.event(Some(id), body)
        });
    }
}

impl Tap for Publisher {
    fn model_touched(&self) {
        if MODEL_TOPICS.iter().any(|topic| self.bus.wants(*topic)) {
            let _ = self.kick.try_send(());
        }
    }

    fn terminal_event(&self, id: i64, event: &PtyEvent) {
        match event {
            PtyEvent::Output if self.bus.wants_prints() => {
                let Some(shared) = self.shared.upgrade() else { return };
                self.bus.pulse(self.window, id, || {
                    let terminal = shared.terminal_of(id)?;
                    terminal.reader().map(|reader| reader.downgrade())
                });
            }
            PtyEvent::Bell => self.bus.publish(Topic::Bell, || self.event(Some(id), Body::Bell)),
            PtyEvent::Mark(mark, prompt) if self.bus.wants(Topic::Blocks) => self.block(id, *mark, *prompt),
            _ => {}
        }
    }
}
