//! The app's windows: which windows there are, what is saved of each, and the ids they share. What the GPUI
//! app keeps in `AppState` (`alloc_window_id`, the open windows, `save_window`) and `window/save.rs`, over
//! core's `Store`. A window is a `Workspace` (`workspace_windows`); this object is what they have in common:
//! one counter of terminal and split ids, one of window ids, the database.
//!
//! One `Windows` is one profile's: its database, its saved windows. The profiles open at once (the shown one and the
//! parked ones) are made `beside` each other and share what is the app's, not a profile's: the id counters (a
//! terminal's id is never taken twice) and the list of live windows (the control line reaches every terminal).
//!
//! Swift decides nothing here: it asks for the windows to open at launch, opens an `NSWindow` for each and
//! calls `save` when something about one changed (a revision moved, the frame moved).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, Weak};

use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};
use wormhole_core::storage::store::{Store, WindowRecord, front_last};
use wormhole_core::view::layout::frame::WindowFrame;
use wormhole_core::view::layout::{REMEMBERED_PANES_KEY, RememberedPanes};
use wormhole_core::workspace::{IdCounter, SavedLayout};

use super::frame::FrameRecord;
use crate::error::FfiError;
use crate::workspace::{Shared, Workspace};
use crate::{isolation, profile};

/// The app's terminal ids, once its windows are made (`Windows::new`).
static APP_IDS: OnceLock<IdCounter> = OnceLock::new();

/// The first terminal id not handed out yet in the whole app, once its windows are made: every id below it may belong
/// to a terminal of some window, every id from it on is new (the GPUI app's `AppState::next_id`).
pub(crate) fn next_terminal_id() -> Option<i64> {
    APP_IDS.get().map(IdCounter::peek)
}

/// A window saved at the last quit (or the last change), in the order to open them: the one that was in front
/// last.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SavedWindow {
    pub id: i64,
    /// Where it was; `None` for a record without one, or one that is garbage.
    pub frame: Option<FrameRecord>,
    pub terminals: u32,
}

/// A window the app is about to open for something that asked: its id and the id of its first terminal are
/// known before it is on screen.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PlannedWindow {
    pub window: i64,
    pub terminal: i64,
    /// Where its terminal starts.
    pub folder: Option<String>,
}

#[derive(uniffi::Object)]
pub struct Windows {
    store: Mutex<Store>,
    /// The profile these windows belong to: a request is judged by the profile of the window it reaches.
    profile: ProfilePaths,
    pub(super) ids: IdCounter,
    pub(super) next_window: Arc<AtomicI64>,
    /// Windows planned and not saved yet: what they start with (`Workspace::restore` reads it).
    pub(super) planned: Mutex<HashMap<i64, WindowRecord>>,
    pub(super) live: Arc<Mutex<Live>>,
    /// The columns last written as the ones new windows start with.
    remembered: Mutex<Option<RememberedPanes>>,
    /// Told of every window made from now on (the control socket's event publishers).
    on_register: Arc<Mutex<Option<OnRegister>>>,
    /// The files the terminals changed, kept with the windows (`claims`).
    pub(super) claims: super::claims::Claims,
}

pub(super) type Live = Vec<(i64, Weak<Shared>)>;
type OnRegister = Box<dyn Fn(&Arc<Shared>) + Send + Sync>;

pub(super) fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Windows {
    fn open_store(dir: &Path) -> Result<Store, FfiError> {
        Store::open_at(dir).map_err(|err| FfiError::Start { reason: format!("{err:#}") })
    }

    fn over(store: Store, profile: ProfilePaths, ids: IdCounter, next_window: Arc<AtomicI64>) -> Self {
        Self {
            store: Mutex::new(store),
            profile,
            ids,
            next_window,
            planned: Mutex::default(),
            live: Arc::default(),
            remembered: Mutex::default(),
            on_register: Arc::default(),
            claims: super::claims::Claims::default(),
        }
    }

    pub(crate) fn store_locked(&self) -> MutexGuard<'_, Store> {
        locked(&self.store)
    }

    /// The default profile of the data folder `dir`.
    #[cfg(test)]
    pub(crate) fn in_dir(dir: &Path) -> Result<Arc<Self>, FfiError> {
        Self::in_profile(DataRoot::new(dir).default_profile())
    }

    pub(crate) fn in_profile(profile: ProfilePaths) -> Result<Arc<Self>, FfiError> {
        let store = Self::open_store(profile.dir())?;
        let (terminal, window) =
            store.next_ids().map_err(|err| FfiError::Start { reason: format!("{err:#}") })?;
        Ok(Arc::new(Self::over(
            store,
            profile,
            IdCounter::starting_at(terminal),
            Arc::new(AtomicI64::new(window)),
        )))
    }

    pub(crate) fn profile(&self) -> &ProfilePaths {
        &self.profile
    }

    pub(crate) fn beside_in(&self, root: &DataRoot, profile: &str) -> Result<Arc<Self>, FfiError> {
        let failed = |reason: String| FfiError::Start { reason };
        let id = ProfileId::parse(profile).map_err(|err| failed(err.to_string()))?;
        let list = crate::profile::list_in(root);
        if list.get(&id).is_none() {
            return Err(failed(format!("no profile {id}")));
        }
        let profile = root.profile(&id);
        let store = Self::open_store(profile.dir())?;
        let (terminal, window) = store.next_ids().map_err(|err| failed(format!("{err:#}")))?;
        self.ids.keep_above(terminal.saturating_sub(1));
        self.next_window.fetch_max(window, Ordering::Relaxed);
        Ok(Arc::new(Self {
            live: self.live.clone(),
            on_register: self.on_register.clone(),
            ..Self::over(store, profile, self.ids.clone(), self.next_window.clone())
        }))
    }

    pub(crate) fn ids(&self) -> IdCounter {
        self.ids.clone()
    }

    pub(crate) fn register(&self, window: i64, shared: &Arc<Shared>) {
        {
            let mut live = locked(&self.live);
            live.retain(|(_, known)| known.strong_count() > 0);
            live.push((window, Arc::downgrade(shared)));
        }
        if let Some(told) = locked(&self.on_register).as_ref() {
            told(shared);
        }
    }

    /// `told` hears of every window made from now on.
    pub(crate) fn on_register(&self, told: impl Fn(&Arc<Shared>) + Send + Sync + 'static) {
        *locked(&self.on_register) = Some(Box::new(told));
    }

    /// Every window's shared state that is still alive, in the order they were made: the control line reaches
    /// the terminals of all of them.
    pub(crate) fn shares(&self) -> Vec<Arc<Shared>> {
        locked(&self.live).iter().filter_map(|(_, shared)| shared.upgrade()).collect()
    }

    /// The record `window` starts from: a planned one, else what is saved.
    pub(crate) fn record_of(&self, window: i64) -> Option<WindowRecord> {
        if let Some(record) = locked(&self.planned).get(&window) {
            return Some(record.clone());
        }
        let loaded =
            locked(&self.store).load().inspect_err(|err| tracing::error!("loading windows: {err:#}"));
        loaded.ok()?.into_iter().find(|record| record.id == window)
    }

    /// A window opened from `from` (its panels, no groups) with one new terminal in `folder` (where `from`'s selected
    /// terminal is without), or the columns as they were last left when there is no window to copy.
    pub(crate) fn plan_like(&self, from: Option<&Shared>, folder: Option<String>) -> PlannedWindow {
        let (layout, selected) = from
            .map(|from| {
                from.peek_model(|model| {
                    let cwd =
                        model.selected().and_then(|id| model.facts(id)).and_then(|facts| facts.cwd.clone());
                    (model.window_record(0, None).layout, cwd.map(|cwd| cwd.to_string_lossy().into_owned()))
                })
            })
            .unwrap_or_default();
        let folder = folder.or(selected);
        let layout = match layout.as_deref() {
            Some(json) => SavedLayout::for_new_window(Some(json)),
            None => {
                let store = locked(&self.store);
                let last = RememberedPanes::parse(store.meta(REMEMBERED_PANES_KEY).ok().flatten().as_deref());
                let layout =
                    SavedLayout::new(last.panes(), SavedLayout::default().right_tab, Default::default());
                serde_json::to_string(&layout).unwrap_or_default()
            }
        };
        let (window, terminal) = (self.next_window.fetch_add(1, Ordering::Relaxed), self.ids.next());
        let record = WindowRecord {
            id: window,
            layout: Some(layout),
            sessions: vec![wormhole_core::storage::store::SessionRecord::new(terminal, None, folder.clone())],
            ..Default::default()
        };
        locked(&self.planned).insert(window, record);
        PlannedWindow { window, terminal, folder }
    }

    /// `record` as the database holds it: the notes and tasks of each terminal are the Notes tab's own rows, which
    /// a window's record does not carry and a save must not wipe.
    fn with_notes(store: &Store, mut record: WindowRecord) -> WindowRecord {
        for session in &mut record.sessions {
            if let Ok(saved) = store.load_session_notes(session.id) {
                session.notes = saved.notes;
                session.tasks = saved.tasks;
                session.tasks_collapsed = saved.tasks_collapsed;
                session.notes_collapsed = saved.notes_collapsed;
            }
        }
        record
    }

    /// A terminal that has no `Terminal` yet (its host runs and could not be shown) keeps the host and shell its saved
    /// row names, so the next launch still finds it.
    fn keep_hosts_unattached(store: &Store, record: &mut WindowRecord, unattached: &[i64]) {
        if unattached.is_empty() {
            return;
        }
        let Some(saved) = store.load().ok().and_then(|all| all.into_iter().find(|w| w.id == record.id))
        else {
            return;
        };
        for session in record.sessions.iter_mut().filter(|session| unattached.contains(&session.id)) {
            if let Some(before) = saved.sessions.iter().find(|before| before.id == session.id) {
                session.host_id.clone_from(&before.host_id);
                session.shell.clone_from(&before.shell);
            }
        }
    }

    /// The record of `workspace` as window `window`, with the hosts and notes the database holds for it.
    pub(super) fn record_of_window(
        &self,
        store: &Store,
        window: i64,
        workspace: &Workspace,
        frame: Option<FrameRecord>,
    ) -> WindowRecord {
        let bounds = frame.map(|frame| WindowFrame::from(frame).to_json());
        let shared = workspace.shared();
        let mut record = shared.peek_model(|model| model.window_record(window, bounds));
        // Which session host runs a terminal is the terminal's to say; a record without it would forget the hosts.
        let mut unattached = Vec::new();
        for session in &mut record.sessions {
            match shared.terminal_of(session.id) {
                Some(terminal) => session.host_id = terminal.host_id(),
                None => unattached.push(session.id),
            }
        }
        Self::keep_hosts_unattached(store, &mut record, &unattached);
        Self::with_notes(store, record)
    }

    /// Write `record` into the database held by `store`, remembering its columns for new windows.
    pub(super) fn write(&self, mut store: MutexGuard<'_, Store>, window: i64, record: WindowRecord) -> bool {
        self.remember(&store, record.layout.as_deref());
        match store.save_window(&record) {
            Ok(()) => {
                locked(&self.planned).remove(&window);
                true
            }
            Err(err) => {
                tracing::warn!("saving window {window}: {err:#}");
                false
            }
        }
    }

    /// Remember the columns of this window as the ones new windows start with, when they changed.
    fn remember(&self, store: &Store, layout: Option<&str>) {
        let Some(layout) = layout.and_then(|json| serde_json::from_str::<SavedLayout>(json).ok()) else {
            return;
        };
        let now = RememberedPanes::of(&layout.panes());
        let mut last = locked(&self.remembered);
        if *last != Some(now) {
            *last = Some(now);
            if let Err(err) = store.set_meta(REMEMBERED_PANES_KEY, &now.to_json()) {
                tracing::warn!("remembering the side columns: {err:#}");
            }
        }
    }
}

#[uniffi::export]
impl Windows {
    /// Fails unless this is an isolated instance of the experiment, or the database can't be opened.
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, FfiError> {
        isolation::require()?;
        let root = DataRoot::current();
        let windows = Self::in_profile(profile::active_in(&root))?;
        // Past what every profile saved, so the ones opened `beside` this never meet an id again.
        let (terminal, window) = profile::first_free_ids(&root);
        windows.ids.keep_above(terminal.saturating_sub(1));
        windows.next_window.fetch_max(window, Ordering::Relaxed);
        // The app's one counter (the profiles beside it share it), for what must know which ids are still to come.
        let _ = APP_IDS.set(windows.ids());
        Ok(windows)
    }

    /// Another profile's windows, open at the same time as these (the profile Wormhole is switching to): its own
    /// database and saved windows, the same counters and live windows. `profile` is in `profiles.json`.
    pub fn beside(&self, profile: String) -> Result<Arc<Self>, FfiError> {
        isolation::require()?;
        self.beside_in(&DataRoot::current(), &profile)
    }

    /// The windows saved, to open in this order: the one that was in front last, so it is in front again.
    pub fn saved(&self) -> Vec<SavedWindow> {
        let store = locked(&self.store);
        let mut windows =
            store.load().inspect_err(|err| tracing::error!("loading windows: {err:#}")).unwrap_or_default();
        front_last(&mut windows, store.front_window().ok().flatten());
        windows
            .into_iter()
            .map(|record| SavedWindow {
                id: record.id,
                frame: record.bounds.as_deref().and_then(WindowFrame::from_json).map(Into::into),
                terminals: u32::try_from(record.sessions.len()).unwrap_or(u32::MAX),
            })
            .collect()
    }

    /// A window for something that asked (⌘N, `new-window`, a moved group): laid out like `from`, with one
    /// terminal starting in `folder`. Open a window for `window`, then `Workspace::in_window(...)` and `restore()`.
    pub fn plan(&self, from: Option<Arc<Workspace>>, folder: Option<String>) -> PlannedWindow {
        self.plan_like(from.as_ref().map(|from| &**from.shared()), folder)
    }

    /// A window with nothing to start from (a first launch, one that `Workspace::receive` will fill): just an id.
    pub fn alloc_window_id(&self) -> i64 {
        self.next_window.fetch_add(1, Ordering::Relaxed)
    }

    /// Write what `workspace` holds as window `window`'s record (terminals, splits, groups, columns, selection)
    /// with where the window is. Cheap when nothing changed: the rows already equal are not rewritten. False
    /// when the write failed.
    pub fn save(&self, window: i64, workspace: Arc<Workspace>, frame: Option<FrameRecord>) -> bool {
        let mut store = locked(&self.store);
        let record = self.record_of_window(&store, window, &workspace, frame);
        let ids: Vec<i64> = record.sessions.iter().map(|session| session.id).collect();
        self.keep_claims(&mut store, window, &workspace, &ids);
        self.write(store, window, record)
    }

    /// Which window is in front, so the next launch brings the same one forward.
    pub fn save_front(&self, window: i64) {
        if let Err(err) = locked(&self.store).save_front_window(window) {
            tracing::warn!("saving the front window: {err:#}");
        }
    }

    /// The person closed window `window`: it and what it held are forgotten (a quit saves instead).
    pub fn forget(&self, window: i64) {
        locked(&self.planned).remove(&window);
        locked(&self.live).retain(|(id, _)| *id != window);
        if let Err(err) = locked(&self.store).delete_window(window) {
            tracing::warn!("forgetting window {window}: {err:#}");
        }
    }
}
