//! The `Workspace` object: one window's terminals, groups and splits, as Swift holds them. It wraps
//! core's sans-IO `WorkspaceModel` (`wormhole_core::workspace`) behind one mutex and speaks the
//! contract the `Terminal` object already does: Swift sends actions, Rust answers with events, one
//! coalesced `wake()` tells Swift that something changed on a Rust thread, and the snapshots are
//! pulled when a revision moved.
//!
//! Two ways events reach Swift, one handler: the events of a call Swift made (`apply`, `set_facts`)
//! come back from that call; events caused elsewhere (a terminal's title, bell or working
//! directory arriving on its event thread, `workspace_feed`) are queued, `wake()` is called once, and
//! `drain()` returns them. The model's lock is never held while calling Swift or a `Terminal`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use wormhole_core::agents::AGENTS;
use wormhole_core::view::sidebar::{SidebarNumbers, Status};
use wormhole_core::workspace::{
    Action, ChromeSnapshot, DropPlan, Event, Failure, ModelConfig, Over, PanesSnapshot, Refusal, Revisions,
    SidebarSnapshot, TerminalFacts, WorkspaceModel,
};

use crate::terminal::Terminal;
use crate::workspace_feed;

/// What Swift implements to hear that there is something to drain. Called from a Rust thread, with no
/// lock held, at most once between two drains; it must return at once: note the wake and hop to the
/// main queue. Never call into the workspace or a terminal from inside it.
#[uniffi::export(with_foreign)]
pub trait WorkspaceListener: Send + Sync {
    fn wake(&self);
}

/// What the model tells Swift (core's `Event`, with the folder as text).
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum WorkspaceEvent {
    /// A terminal was made: start a program for it (`Terminal.start`), then `register_terminal`.
    Opened {
        id: i64,
        cwd: Option<String>,
    },
    /// A terminal was closed: its registered `Terminal` has been closed; drop its view.
    Closed {
        id: i64,
    },
    Selected {
        id: Option<i64>,
    },
    /// A row moved or joined a split: animate it.
    Moved {
        id: i64,
    },
    /// The sidebar's rows, groups or numbers changed: pull `sidebar`.
    Listing,
    /// The splits or the panes' focus changed: pull `panes`.
    Layout,
    /// Visibility, panel tab, search or overlay changed: pull `chrome`.
    Chrome,
    /// The action changed nothing.
    Refused {
        reason: Refusal,
    },
}

impl From<Event> for WorkspaceEvent {
    fn from(event: Event) -> Self {
        let text = |path: PathBuf| path.to_string_lossy().into_owned();
        match event {
            Event::Opened { id, cwd } => Self::Opened { id, cwd: cwd.map(text) },
            Event::Closed { id } => Self::Closed { id },
            Event::Selected { id } => Self::Selected { id },
            Event::Moved { id } => Self::Moved { id },
            Event::Listing => Self::Listing,
            Event::Layout => Self::Layout,
            Event::Chrome => Self::Chrome,
            Event::Refused(reason) => Self::Refused { reason },
        }
    }
}

/// What the shell knows of the machine and the settings, once.
#[derive(Debug, Clone, uniffi::Record)]
pub struct WorkspaceConfig {
    /// The home folder, for `~/Projects/app` on a row's second line.
    pub home: Option<String>,
    /// ⌘1 to ⌘9 count along the list as shown (`true`), or stay with their terminal (`false`).
    pub numbers_follow_list: bool,
    /// Where a terminal starts when none is selected to take the folder from.
    pub start_folder: Option<String>,
    /// What a pane's header adds to the least height of a pane, in points.
    pub pane_header_height: f32,
}

/// What Swift may set of a terminal's facts, whole (tests, previews, a feed of its own). A terminal
/// registered with `register_terminal` feeds its title, folder, command, bell and failures itself.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TerminalFactsRecord {
    pub title: Option<String>,
    pub shell: String,
    /// The agent's command ("claude"), as `AgentInfo.key` names it.
    pub agent: Option<String>,
    pub reports: bool,
    pub status: Status,
    pub message: Option<String>,
    pub since_ms: u64,
    pub cwd: Option<String>,
    pub branch: Option<String>,
    pub command: Option<String>,
    pub running_since_ms: Option<u64>,
    pub bell: bool,
    pub failure_code: Option<i32>,
    pub failure_command: Option<String>,
    pub failure_at_ms: u64,
    pub unread: bool,
    pub changes: Option<u32>,
    pub notes: u32,
}

impl From<TerminalFactsRecord> for TerminalFacts {
    fn from(record: TerminalFactsRecord) -> Self {
        let failure = record.failure_code.map(|code| Failure {
            code,
            command: record.failure_command,
            at: record.failure_at_ms,
        });
        Self {
            title: record.title,
            shell: record.shell,
            agent: record
                .agent
                .and_then(|key| AGENTS.iter().find(|agent| agent.command == key).map(|a| a.id)),
            reports: record.reports,
            status: record.status,
            message: record.message,
            since: record.since_ms,
            cwd: record.cwd.map(PathBuf::from),
            branch: record.branch,
            command: record.command,
            running_since: record.running_since_ms,
            bell: record.bell,
            failure,
            unread: record.unread,
            changes: record.changes.map(|n| n as usize),
            notes: record.notes as usize,
        }
    }
}

/// What the object and the threads that feed it share.
pub(crate) struct Shared {
    model: Mutex<WorkspaceModel>,
    pending: Mutex<Vec<WorkspaceEvent>>,
    /// The listener was woken and Swift has not drained since.
    woken: AtomicBool,
    listener: Arc<dyn WorkspaceListener>,
    /// The model's clock: the milliseconds since the workspace was made.
    started: Instant,
    terminals: Mutex<HashMap<i64, Arc<Terminal>>>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Shared {
    pub(crate) fn now(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Run `change` on the model and return the events it caused, converted. The lock is released
    /// before the caller does anything with them.
    pub(crate) fn with_model<R>(&self, change: impl FnOnce(&mut WorkspaceModel) -> R) -> R {
        change(&mut locked(&self.model))
    }

    /// Events caused away from a Swift call: queue them and wake once (until drained).
    pub(crate) fn post(&self, events: Vec<Event>) {
        if events.is_empty() {
            return;
        }
        let events = self.close_what_closed(events);
        {
            let mut pending = locked(&self.pending);
            for event in events {
                // A part that changed twice is told once.
                if matches!(event, WorkspaceEvent::Listing | WorkspaceEvent::Layout | WorkspaceEvent::Chrome)
                    && pending.contains(&event)
                {
                    continue;
                }
                pending.push(event);
            }
        }
        if !self.woken.swap(true, Ordering::AcqRel) {
            self.listener.wake();
        }
    }

    /// A closed terminal's registered `Terminal` ends with it: the model's `Closed` is the one place
    /// a program's life ends, so Swift cannot forget to.
    pub(crate) fn close_what_closed(&self, events: Vec<Event>) -> Vec<WorkspaceEvent> {
        for event in &events {
            if let Event::Closed { id } = event
                && let Some(terminal) = locked(&self.terminals).remove(id)
            {
                terminal.close();
            }
        }
        events.into_iter().map(WorkspaceEvent::from).collect()
    }
}

/// One window's terminals, groups and splits.
#[derive(uniffi::Object)]
pub struct Workspace {
    shared: Arc<Shared>,
}

#[uniffi::export]
impl Workspace {
    #[uniffi::constructor]
    pub fn new(config: WorkspaceConfig, listener: Arc<dyn WorkspaceListener>) -> Arc<Self> {
        let model = WorkspaceModel::new(ModelConfig {
            home: config.home.map(PathBuf::from),
            numbers: if config.numbers_follow_list { SidebarNumbers::List } else { SidebarNumbers::Terminal },
            start_folder: config.start_folder.map(PathBuf::from),
            pane_header_h: config.pane_header_height,
        });
        Arc::new(Self {
            shared: Arc::new(Shared {
                model: Mutex::new(model),
                pending: Mutex::new(Vec::new()),
                woken: AtomicBool::new(false),
                listener,
                started: Instant::now(),
                terminals: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// Do what the person did. What changed comes back as events (see the module's note).
    pub fn apply(&self, action: Action) -> Vec<WorkspaceEvent> {
        let events = self.shared.with_model(|model| model.apply(action));
        self.shared.close_what_closed(events)
    }

    /// Everything queued by threads other than the caller's since the last drain; re-arms the wake.
    pub fn drain(&self) -> Vec<WorkspaceEvent> {
        self.shared.woken.store(false, Ordering::Release);
        std::mem::take(&mut *locked(&self.shared.pending))
    }

    /// The list as the sidebar draws it, with the words as they read now. Pull it when `Listing`
    /// came, or when `refresh_in_ms` has passed.
    pub fn sidebar(&self) -> SidebarSnapshot {
        self.sidebar_at(self.now_ms())
    }

    /// [`Self::sidebar`] as it reads at `now_ms` on the workspace's clock (tests, previews).
    pub fn sidebar_at(&self, now_ms: u64) -> SidebarSnapshot {
        self.shared.with_model(|model| model.sidebar_snapshot(now_ms))
    }

    /// The panes of the selected terminal's split in an area of `width` x `height` points.
    pub fn panes(&self, width: f32, height: f32) -> PanesSnapshot {
        self.shared.with_model(|model| model.panes_snapshot(width, height))
    }

    pub fn chrome(&self) -> ChromeSnapshot {
        self.shared.with_model(|model| model.chrome_snapshot())
    }

    pub fn revisions(&self) -> Revisions {
        self.shared.with_model(|model| model.revisions())
    }

    pub fn selected(&self) -> Option<i64> {
        self.shared.with_model(|model| model.selected())
    }

    /// What dropping terminal `dragged` on `over` would do, for the indicator while the pointer is
    /// still down; `None` when nothing would change. A pure question: nothing moves.
    pub fn drop_plan(&self, dragged: i64, over: Over) -> Option<DropPlan> {
        self.shared.with_model(|model| model.drop_plan(dragged, over))
    }

    /// Replace a terminal's facts whole. Unknown ids are ignored.
    pub fn set_facts(&self, id: i64, facts: TerminalFactsRecord) -> Vec<WorkspaceEvent> {
        let events = self.shared.with_model(|model| model.set_facts(id, facts.into()));
        self.shared.close_what_closed(events)
    }

    /// What git says of a terminal's folder: the branch and the files changed.
    pub fn set_git(&self, id: i64, branch: Option<String>, changes: Option<u32>) -> Vec<WorkspaceEvent> {
        let events = self.shared.with_model(|model| {
            model.update_facts(id, |facts| {
                facts.branch = branch;
                facts.changes = changes.map(|n| n as usize);
            })
        });
        self.shared.close_what_closed(events)
    }

    /// Hand over the `Terminal` started for terminal `id` (after `Opened`): its title, folder, bell,
    /// commands and failures feed the model from now on, and `Closed` ends it.
    pub fn register_terminal(&self, id: i64, terminal: Arc<Terminal>) {
        locked(&self.shared.terminals).insert(id, terminal.clone());
        workspace_feed::attach(&self.shared, id, &terminal);
    }

    /// The registered `Terminal` of terminal `id`: its frames and keys.
    pub fn terminal(&self, id: i64) -> Option<Arc<Terminal>> {
        locked(&self.shared.terminals).get(&id).cloned()
    }

    /// The workspace's clock: milliseconds since it was made. Facts that carry a time use it.
    pub fn now_ms(&self) -> u64 {
        self.shared.now()
    }
}
