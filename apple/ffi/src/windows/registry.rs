//! The app's windows: which windows there are, what is saved of each, and the ids they share. What the GPUI
//! app keeps in `AppState` (`alloc_window_id`, the open windows, `save_window`) and `window/save.rs`, over
//! core's `Store`. A window is a `Workspace` (`workspace_windows`); this object is what they have in common:
//! one counter of terminal and split ids, one of window ids, the database.
//!
//! Swift decides nothing here: it asks for the windows to open at launch, opens an `NSWindow` for each and
//! calls `save` when something about one changed (a revision moved, the frame moved).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use wormhole_core::storage::store::{Store, WindowRecord, front_last};
use wormhole_core::view::layout::frame::WindowFrame;
use wormhole_core::view::layout::{REMEMBERED_PANES_KEY, RememberedPanes};
use wormhole_core::workspace::{IdCounter, SavedLayout};

use super::frame::FrameRecord;
use crate::error::FfiError;
use crate::isolation;
use crate::workspace::{Shared, Workspace};

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
    ids: IdCounter,
    next_window: AtomicI64,
    /// Windows planned and not saved yet: what they start with (`Workspace::restore` reads it).
    planned: Mutex<HashMap<i64, WindowRecord>>,
    live: Mutex<Vec<(i64, Weak<Shared>)>>,
    /// The columns last written as the ones new windows start with.
    remembered: Mutex<Option<RememberedPanes>>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Windows {
    pub(crate) fn in_dir(dir: &Path) -> Result<Arc<Self>, FfiError> {
        let store = Store::open_at(dir).map_err(|err| FfiError::Start { reason: format!("{err:#}") })?;
        let (terminal, window) =
            store.next_ids().map_err(|err| FfiError::Start { reason: format!("{err:#}") })?;
        Ok(Arc::new(Self {
            store: Mutex::new(store),
            ids: IdCounter::starting_at(terminal),
            next_window: AtomicI64::new(window),
            planned: Mutex::default(),
            live: Mutex::default(),
            remembered: Mutex::default(),
        }))
    }

    pub(crate) fn ids(&self) -> IdCounter {
        self.ids.clone()
    }

    pub(crate) fn register(&self, window: i64, shared: &Arc<Shared>) {
        let mut live = locked(&self.live);
        live.retain(|(_, known)| known.strong_count() > 0);
        live.push((window, Arc::downgrade(shared)));
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
        Self::in_dir(&wormhole_core::storage::paths::data_dir())
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
        let bounds = frame.map(|frame| WindowFrame::from(frame).to_json());
        let record = workspace.shared().peek_model(|model| model.window_record(window, bounds));
        let mut store = locked(&self.store);
        self.remember(&store, record.layout.as_deref());
        let record = Self::with_notes(&store, record);
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
