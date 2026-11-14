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
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Instant, SystemTime};

use wormhole_core::agents::AGENTS;
use wormhole_core::view::layout::Edge;
use wormhole_core::view::panes::{Preset, Zone};
use wormhole_core::view::sidebar::{Landing, RowState, SidebarNumbers, Status};
use wormhole_core::workspace::{
    Action, ChromeSnapshot, DiskGround, DropPlan, EdgeDrag, Event, Failure, IdCounter, ModelConfig, Over,
    PanesSnapshot, Refusal, Revisions, SidebarSnapshot, StatusEvent, TerminalFacts, TerminalsSnapshot,
    TitleSnapshot, WorkspaceModel, open_items,
};

use crate::hook_server::Tap;
use crate::terminal::Terminal;
use crate::windows::Windows;
use crate::workspace_extras::Extras;
use crate::workspace_feed::{self, agent_by_key};
use crate::workspace_git::GitJobs;
use crate::workspace_pump::{self, Pump};

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
    /// A terminal came from another window with its program running: make a view for it, start nothing.
    Adopted {
        id: i64,
    },
    Selected {
        id: Option<i64>,
    },
    /// A row moved or joined a split: animate it.
    Moved {
        id: i64,
    },
    /// "Forget text" was set on the terminal: erase what was saved of its screen.
    SavedTextForgotten {
        id: i64,
    },
    /// The group is to move to a window of its own, with these terminals (and their splits').
    GroupToWindow {
        group: u32,
        terminals: Vec<i64>,
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
            Event::Adopted { id } => Self::Adopted { id },
            Event::Selected { id } => Self::Selected { id },
            Event::Moved { id } => Self::Moved { id },
            Event::SavedTextForgotten { id } => Self::SavedTextForgotten { id },
            Event::GroupToWindow { group, terminals } => Self::GroupToWindow { group, terminals },
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
    /// The command that just finished, while its result is unseen: it still names the terminal.
    pub finished_command: Option<String>,
    /// The repository it is in (its root), and whether the tree is dirty, how far the branch is ahead
    /// of and behind its upstream, how many files the repository has changed.
    pub repo: Option<String>,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
    pub repo_files: Option<u32>,
}

impl From<TerminalFactsRecord> for TerminalFacts {
    fn from(record: TerminalFactsRecord) -> Self {
        let failure = record.failure_code.map(|code| Failure {
            serial: 1,
            code,
            command: record.failure_command,
            at: record.failure_at_ms,
            took: None,
            last_line: None,
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
            notice: None,
            unread: record.unread,
            changes: record.changes.map(|n| n as usize),
            notes: record.notes as usize,
            finished_command: record.finished_command,
            repo: record.repo.map(PathBuf::from),
            dirty: record.dirty,
            ahead: record.ahead,
            behind: record.behind,
            repo_files: record.repo_files.map(|n| n as usize),
            superproject: None,
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
    /// The system's clock at the moment `started`: a process's start time is read against it.
    pub(crate) epoch: SystemTime,
    terminals: Mutex<HashMap<i64, Arc<Terminal>>>,
    /// The thread that waits for the moments the model needs (`workspace_pump`).
    pub(crate) pump: Arc<Pump>,
    /// What git is asked and followed with (`workspace_git`).
    pub(crate) git: GitJobs,
    /// The title row's ports and pull request (`workspace_extras`).
    pub(crate) extras: Extras,
    /// Who hears what happens to the terminals besides Swift: the control socket's event bus
    /// (`hook_server`). Set once, by the server.
    pub(crate) tap: OnceLock<Arc<dyn Tap>>,
    /// The windows this one is among, and its id there (`workspace_windows`). Set by `Workspace::in_window`; a window moved to
    /// another profile is among that profile's from then on (`Windows::rehome`).
    windows: Mutex<Option<(Arc<Windows>, i64)>>,
    /// The texts of the terminals the window was restored with, until each starts (`workspace_texts`).
    pub(crate) restored_texts: crate::workspace_texts::RestoredTexts,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Shared {
    /// The registered `Terminal` of terminal `id`.
    pub(crate) fn terminal_of(&self, id: i64) -> Option<Arc<Terminal>> {
        locked(&self.terminals).get(&id).cloned()
    }

    /// Every registered terminal with its id.
    pub(crate) fn registered(&self) -> Vec<(i64, Arc<Terminal>)> {
        locked(&self.terminals).iter().map(|(id, terminal)| (*id, terminal.clone())).collect()
    }

    /// The windows this one is among and its id there, `None` when it is not among several.
    pub(crate) fn windows(&self) -> Option<(Arc<Windows>, i64)> {
        locked(&self.windows).clone()
    }

    pub(crate) fn set_windows(&self, windows: Arc<Windows>, id: i64) {
        *locked(&self.windows) = Some((windows, id));
    }

    /// The id of the window this workspace is (1 when it is not among several).
    pub(crate) fn window_id(&self) -> i64 {
        self.windows().map_or(1, |(_, id)| id)
    }

    /// Unregister terminal `id` without ending it (it is moving to another window).
    pub(crate) fn take_terminal(&self, id: i64) -> Option<Arc<Terminal>> {
        locked(&self.terminals).remove(&id)
    }

    /// Register a running terminal that came from another window; `workspace_feed::attach` feeds from it.
    pub(crate) fn put_terminal(&self, id: i64, terminal: Arc<Terminal>) {
        locked(&self.terminals).insert(id, terminal);
    }

    pub(crate) fn now(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Run `change` on the model and return the events it caused, converted. The lock is released
    /// before the caller does anything with them.
    pub(crate) fn with_model<R>(&self, change: impl FnOnce(&mut WorkspaceModel) -> R) -> R {
        let result = change(&mut locked(&self.model));
        // After the lock is let go: the tap only notes that the model may have moved.
        if let Some(tap) = self.tap.get() {
            tap.model_touched();
        }
        result
    }

    /// Read the model without telling the tap: what the tap's own thread reads must not wake it again.
    pub(crate) fn peek_model<R>(&self, read: impl FnOnce(&WorkspaceModel) -> R) -> R {
        read(&locked(&self.model))
    }

    /// Events caused away from a Swift call: queue them and wake once (until drained).
    pub(crate) fn post(&self, events: Vec<Event>) {
        if events.is_empty() {
            return;
        }
        let events = self.close_what_closed(events);
        self.sync_fold_guards();
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

    /// What needs the person is never under a fold: each terminal's folds are told whether its row says so now.
    fn sync_fold_guards(&self) {
        let terminals: Vec<(i64, Arc<Terminal>)> =
            locked(&self.terminals).iter().map(|(id, terminal)| (*id, terminal.clone())).collect();
        for (id, terminal) in terminals {
            let needs = self.peek_model(|model| {
                model.facts(id).is_some_and(|facts| facts.row_state() == RowState::NeedsYou)
            });
            terminal.fold_needs_you(needs);
        }
    }

    /// A closed terminal's registered `Terminal` ends with it: the model's `Closed` is the one place
    /// a program's life ends, so Swift cannot forget to.
    pub(crate) fn close_what_closed(&self, events: Vec<Event>) -> Vec<WorkspaceEvent> {
        for event in &events {
            match event {
                Event::Closed { id } => {
                    self.pump.forget(*id);
                    if let Some(terminal) = locked(&self.terminals).remove(id) {
                        terminal.close();
                    }
                }
                // "Don't keep this terminal's text": what was saved goes now, not at the next minute's save.
                Event::SavedTextForgotten { id } => self.erase_saved_text(*id),
                _ => {}
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
        Self::build(config, listener, None)
    }

    /// Do what the person did. What changed comes back as events (see the module's note).
    pub fn apply(&self, action: Action) -> Vec<WorkspaceEvent> {
        let events = self.shared.with_model(|model| model.apply(action));
        // Selecting looks at a terminal, which may ask for its programs' reports to be dropped; a new
        // or closed terminal and a new selection move what the pump waits for.
        self.shared.serve_asks();
        self.shared.pump.kick();
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

    /// Every terminal, those a folded group hides included, in the list's order: the palette, the
    /// status bar's counts and menus read it. Pull it when `revisions().terminals` moved.
    pub fn terminals(&self) -> TerminalsSnapshot {
        self.shared.with_model(|model| {
            let snapshot = model.terminals_snapshot();
            crate::workspace_cwd::keyed(model, snapshot)
        })
    }

    /// The selected terminal's title row: parts, chevrons, path steps to measure and the path menu.
    /// `boot_volume` is what the system calls the startup volume.
    pub fn title(&self, boot_volume: String) -> TitleSnapshot {
        self.shared.with_model(|model| model.title_snapshot(&boot_volume))
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

    /// Where dropping group `group` on `over` would put it; `None` when nothing would change.
    pub fn group_drop_plan(&self, group: u32, over: Over) -> Option<Landing> {
        self.shared.with_model(|model| model.group_drop_plan(group, over))
    }

    /// Where a drag of a column's edge with the pointer `extent` from the window edge it hangs on
    /// would leave it, without moving it (the readout beside the pointer).
    pub fn edge_drag(&self, edge: Edge, extent: f32) -> EdgeDrag {
        self.shared.with_model(|model| model.edge_drag(edge, extent))
    }

    /// Whether the area has room for the layout's panes at their least size.
    pub fn can_layout(&self, preset: Preset) -> bool {
        self.shared.with_model(|model| model.can_layout(preset))
    }

    /// Why dropping terminal `dragged` on `target`'s `zone` would be refused, nothing when it works (a member of
    /// the split moves, any other terminal joins); the pane area's drop hint reads it.
    pub fn pane_drop_refusal(&self, dragged: i64, target: i64, zone: Zone) -> Option<Refusal> {
        self.shared.with_model(|model| model.pane_drop_refusal(dragged, target, zone))
    }

    /// Whether group `group`'s fence lets terminal `id` (and its split) in: the menus grey out a group that refuses.
    pub fn group_admits(&self, group: u32, id: i64) -> bool {
        self.shared.with_model(|model| model.admits(group, id))
    }

    /// The terminal last selected in `group` (`None`: the terminals in no group): what opening a tab set selects.
    pub fn last_selected_in(&self, group: Option<u32>) -> Option<i64> {
        self.shared.with_model(|model| model.last_selected_in(group))
    }

    /// The folder to ask the person about before a terminal opens in a group with none, if there is
    /// one worth offering. Answer with `setGroupFolder` or `declineGroupFolder`.
    pub fn group_folder_offer(&self, group: u32) -> Option<String> {
        self.shared.with_model(|model| {
            model.group_folder_offer(group).map(|folder| folder.to_string_lossy().into_owned())
        })
    }

    /// How many of `group`'s terminals would leave it if it kept only the terminals in `folder`: the folder question's count.
    pub fn group_folder_leaving(&self, group: u32, folder: String) -> u32 {
        self.shared.with_model(|model| model.members_leaving(group, std::path::Path::new(&folder)) as u32)
    }

    /// An agent's hook event for terminal `id` (what `WORMHOLE_TAB_ID` names): `agent` is its command
    /// ("claude"), `event` as the hook script spells it (`Stop`, `Notification:permission_prompt`,
    /// `PostToolUse:wakes=b7`), `message` the words it carries. What the event means for the status is
    /// core's (`StatusMachine`). Unknown agents and terminals are ignored.
    pub fn hook(
        &self,
        id: i64,
        agent: String,
        event: String,
        message: Option<String>,
    ) -> Vec<WorkspaceEvent> {
        let Some(agent) = agent_by_key(&agent) else { return Vec::new() };
        let at = self.shared.now();
        let status = StatusEvent::Hook { agent, event, message };
        let events = self.shared.with_model(|model| model.feed(id, status, at));
        let events = self.shared.close_what_closed(events);
        self.shared.serve_asks();
        self.shared.pump.kick();
        events
    }

    /// The window came to the front or went behind (`active`), and whether it can be seen at all
    /// (`seen`: not minimised, hidden or covered). In front, what is on screen is looked at: results
    /// are spent and bells heard. Git is read as often as the terminals are seen.
    pub fn set_window_state(&self, active: bool, seen: bool) -> Vec<WorkspaceEvent> {
        let at = self.shared.now();
        let events = self.shared.with_model(|model| {
            model.set_window_seen(seen);
            model.set_window_active(active, at)
        });
        let events = self.shared.close_what_closed(events);
        self.shared.pump.kick();
        events
    }

    /// A terminal's notes and tasks, as the row counts them: `tasks_done` has one flag per task, `notes`
    /// the text of each note. The row shows how many are still open (`open_items`: tasks not done and
    /// notes with something in them).
    pub fn set_notes(&self, id: i64, tasks_done: Vec<bool>, notes: Vec<String>) -> Vec<WorkspaceEvent> {
        let open = open_items(tasks_done, notes.iter().map(String::as_str));
        let events = self.shared.with_model(|model| model.update_facts(id, |facts| facts.notes = open));
        self.shared.close_what_closed(events)
    }

    /// Ask git again where it was missing (Retry); a terminal whose repository cannot be read otherwise
    /// keeps its last answer.
    pub fn retry_git(&self) {
        self.shared.with_model(|model| model.git_retry());
        self.shared.pump.kick();
    }

    /// Replace a terminal's facts whole. Unknown ids are ignored.
    pub fn set_facts(&self, id: i64, facts: TerminalFactsRecord) -> Vec<WorkspaceEvent> {
        let events = self.shared.with_model(|model| model.set_facts(id, facts.into()));
        // Its folder or what it works at may be new to the git feed.
        self.shared.pump.kick();
        self.shared.close_what_closed(events)
    }

    /// What git says of a terminal's folder, set from outside (previews, tests). A registered terminal's
    /// repository is read by the workspace itself, which overwrites this at its next answer.
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

impl Workspace {
    /// A workspace whose ids come from `ids` when it shares them with other windows (`workspace_windows`).
    pub(crate) fn build(
        config: WorkspaceConfig,
        listener: Arc<dyn WorkspaceListener>,
        ids: Option<IdCounter>,
    ) -> Arc<Self> {
        let model = WorkspaceModel::new(ModelConfig {
            home: config.home.map(PathBuf::from),
            numbers: if config.numbers_follow_list { SidebarNumbers::List } else { SidebarNumbers::Terminal },
            start_folder: config.start_folder.map(PathBuf::from),
            pane_header_h: config.pane_header_height,
            // The disk is this crate's to ask: a group that keeps only its folder's terminals reads
            // where each really is.
            ground: Some(Arc::new(DiskGround)),
            ids,
            ..ModelConfig::default()
        });
        let shared = Arc::new(Shared {
            model: Mutex::new(model),
            pending: Mutex::new(Vec::new()),
            woken: AtomicBool::new(false),
            listener,
            started: Instant::now(),
            epoch: SystemTime::now(),
            terminals: Mutex::new(HashMap::new()),
            pump: Arc::default(),
            git: GitJobs::default(),
            extras: Extras::default(),
            tap: OnceLock::new(),
            windows: Mutex::new(None),
            restored_texts: Default::default(),
        });
        workspace_pump::spawn(&shared);
        Arc::new(Self { shared })
    }

    /// What the control socket's server works on (`hook_server`).
    pub(crate) fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }
}

impl Drop for Workspace {
    /// The pump thread ends with the workspace.
    fn drop(&mut self) {
        self.shared.pump.stop();
    }
}
