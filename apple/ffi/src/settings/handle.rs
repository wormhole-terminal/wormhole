//! The `SettingsHandle` object: one profile's settings as the Settings window holds them. Swift sends
//! keys and JSON values, Rust answers with core's verdict; one coalesced `wake()` says the file
//! changed under it, and `drain()` returns the revision to compare. No Swift type mirrors `Settings`
//! and Swift never reads or writes `config.json`.
//!
//! Threads: every call locks the state for the length of a `stat` or an atomic write (about a
//! millisecond) and may come from any thread. The lock is never held across `wake()`. The watcher
//! thread (`watch`) is the only caller of `wake()`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::Value;
use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};

use super::error::SettingsError;
use super::state::State;
use super::values::to_text;
use super::watch::Follower;
use crate::isolation;

/// What Swift implements to hear that the settings changed on a Rust thread (a hand edit of the file,
/// another window). Called with no lock held, at most once between two `drain()` calls; it must return
/// at once: note the wake and hop to the main queue. Never call the handle from inside it.
#[uniffi::export(with_foreign)]
pub trait SettingsListener: Send + Sync {
    fn wake(&self);
}

pub(crate) struct Shared {
    pub(crate) state: Mutex<State>,
    woken: AtomicBool,
    listener: Arc<dyn SettingsListener>,
}

impl Shared {
    pub(crate) fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn wake(&self) {
        if !self.woken.swap(true, Ordering::AcqRel) {
            self.listener.wake();
        }
    }
}

/// What the page's banner says: the file does not read, or the last save failed, or a key means nothing.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsHealth {
    pub error: Option<String>,
    /// The file did not read in full: changes are kept in memory and not saved until it is fixed.
    pub unreadable: bool,
}

/// A row of the Changed page.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsChangedRow {
    /// The schema's section id (`appearance`, `menuBar`).
    pub section: String,
    pub entry: String,
}

/// One profile's settings. Hold it for as long as the window is open: dropping it stops the watch.
#[derive(uniffi::Object)]
pub struct SettingsHandle {
    pub(crate) shared: Arc<Shared>,
    pub(crate) root: DataRoot,
    pub(crate) paths: ProfilePaths,
    _follower: Follower,
}

impl SettingsHandle {
    /// Loads `profile`'s settings under `root` (writing the defaults when the file is not there) and
    /// starts following them. The one constructor tests use, over a folder of their own.
    pub(crate) fn open_at(
        root: &DataRoot,
        profile: &ProfileId,
        listener: Arc<dyn SettingsListener>,
    ) -> Arc<Self> {
        let paths = root.profile(profile);
        let shared = Arc::new(Shared {
            state: Mutex::new(State::load(&paths)),
            woken: AtomicBool::new(false),
            listener,
        });
        let watched = Arc::downgrade(&shared);
        let follower = Follower::start(&paths, move || {
            let Some(shared) = watched.upgrade() else { return false };
            let moved = shared.lock().poll();
            if moved {
                shared.wake();
            }
            true
        });
        Arc::new(Self { shared, root: root.clone(), paths, _follower: follower })
    }

    pub(crate) fn parse(entries: &str) -> Result<Vec<(String, Value)>, SettingsError> {
        match serde_json::from_str::<Value>(entries) {
            Ok(Value::Object(map)) => Ok(map.into_iter().collect()),
            _ => Err(SettingsError::Invalid { reason: "settings to set must be a JSON object".into() }),
        }
    }
}

#[uniffi::export]
impl SettingsHandle {
    /// Opens a profile's settings: the one Wormhole is in when `profile` is `None`. Refuses unless the
    /// process is an isolated instance of the experiment. Reads (and may create) a file: call it off
    /// the main thread in a real app.
    #[uniffi::constructor]
    pub fn open(
        profile: Option<String>,
        listener: Arc<dyn SettingsListener>,
    ) -> Result<Arc<Self>, SettingsError> {
        isolation::require()?;
        let root = DataRoot::current();
        let id = match profile {
            Some(text) => {
                ProfileId::parse(&text).map_err(|err| SettingsError::Profile { reason: err.to_string() })?
            }
            None => crate::profile::active_in(&root).id().clone(),
        };
        Ok(Self::open_at(&root, &id, listener))
    }

    /// The effective value of every key the schema names, as one JSON object (a project's overrides
    /// on top of the file; keys inside an object by their path, `motion.insert`). Pull it when
    /// `drain()` or a write says the revision moved.
    pub fn values(&self) -> String {
        to_text(self.shared.lock().values())
    }

    /// The same keys at their defaults: what a row's reset arrow puts back. Never changes.
    pub fn defaults(&self) -> String {
        to_text(super::values::schema_values(&wormhole_core::config::Settings::default()))
    }

    /// Sets one key to a JSON value. Core refuses (and nothing changes) a key that is no setting and a
    /// value its setting does not read, with a sentence for the control. A value that is clamped
    /// reads back clamped.
    pub fn set(&self, key: String, value: String) -> Result<(), SettingsError> {
        let value = serde_json::from_str::<Value>(&value)
            .map_err(|err| SettingsError::Invalid { reason: format!("`{key}`: {err}") })?;
        self.write(vec![(key, value)])
    }

    /// Sets several keys in one save, all or nothing (`entries` is a JSON object of key to value): a
    /// control that changes two keys, a slider's settle.
    pub fn set_many(&self, entries: String) -> Result<(), SettingsError> {
        self.write(Self::parse(&entries)?)
    }

    /// Puts keys back at their defaults and saves (one row's reset, Reset All).
    pub fn reset(&self, keys: Vec<String>) {
        self.shared.lock().reset(&keys);
    }

    /// Puts every key that differs from its default back: Advanced's "Reset all settings". Returns how
    /// many keys were changed.
    pub fn reset_all(&self) -> u32 {
        let mut state = self.shared.lock();
        let keys = state.changed_keys();
        state.reset(&keys);
        u32::try_from(keys.len()).unwrap_or(u32::MAX)
    }

    /// The schema keys that differ from their default in the person's own settings (a project's
    /// overrides do not count): the rows' changed marks.
    pub fn changed_keys(&self) -> Vec<String> {
        self.shared.lock().changed_keys()
    }

    /// The current values of `keys` as one line of JSON, `{"blur": true}`, for a row's "Copy as JSON": a snippet to paste
    /// into config.json, in core's shape (a key inside an object as that object; a key holding nothing left out).
    pub fn copy_json(&self, keys: Vec<String>) -> String {
        self.shared.lock().copy_json(&keys)
    }

    /// The same for every key the Changed page lists: the page's "Copy as JSON".
    pub fn copy_changed_json(&self) -> String {
        self.shared.lock().changed_json()
    }

    /// The rows the Changed page lists, in page order.
    pub fn changed_rows(&self) -> Vec<SettingsChangedRow> {
        let rows = self.shared.lock().changed_rows();
        rows.into_iter()
            .map(|(section, entry)| SettingsChangedRow { section, entry: entry.to_string() })
            .collect()
    }

    /// Layers the active terminal's project settings (a JSON object, from a trusted project's
    /// `.wormhole/config.json`; `None` for none) over the person's. Keys a project may not change are
    /// dropped. Trust is the caller's to decide: this never reads a project.
    pub fn set_project_overrides(&self, overrides: Option<String>) -> Result<(), SettingsError> {
        let map = match overrides {
            Some(text) => Some(match serde_json::from_str::<Value>(&text) {
                Ok(Value::Object(map)) => map,
                _ => {
                    return Err(SettingsError::Invalid {
                        reason: "a project's settings must be a JSON object".into(),
                    });
                }
            }),
            None => None,
        };
        self.shared.lock().set_project_overrides(map);
        Ok(())
    }

    /// The ids of the rows a project's settings set: the "set by project" marks.
    pub fn overridden_entries(&self) -> Vec<String> {
        self.shared.lock().overridden_entries().into_iter().map(str::to_string).collect()
    }

    /// Moves whenever what a window shows would read differently (a write, a reload, a project's
    /// overrides). Compare it; do not count it.
    pub fn revision(&self) -> u64 {
        self.shared.lock().revision()
    }

    /// Re-arms `wake()` and returns the revision: call it first thing after a wake.
    pub fn drain(&self) -> u64 {
        self.shared.woken.store(false, Ordering::Release);
        self.revision()
    }

    pub fn health(&self) -> SettingsHealth {
        let health = self.shared.lock().health();
        SettingsHealth { error: health.error, unreadable: health.unreadable }
    }
}

impl SettingsHandle {
    fn write(&self, writes: Vec<(String, Value)>) -> Result<(), SettingsError> {
        self.shared.lock().set_many(writes).map(drop).map_err(|reason| SettingsError::Invalid { reason })
    }
}
