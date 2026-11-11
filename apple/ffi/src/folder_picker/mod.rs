//! The folder picker's providers and decorators (core's `folder_picker`: typed paths, the multi-level fuzzy
//! walk, recent folders by frecency, chevrons and git branch badges) as jobs Swift starts and may cancel. The
//! disk is read on threads of our own under the app's gate, every look through core's `looks` so a hung mount is
//! skipped; the answers come back through one listener, once per job, whatever became of it.
//!
//! Swift keeps the picker's model (selection, keys, merging answers); this is only what feeds it.

mod gate;
mod records;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use wormhole_core::folder_picker::{RecentFolders, keep_out, parts};
use wormhole_core::platform::paths;
use wormhole_core::view::picker::{Cancel, Decorator, Provider};

pub use records::*;

/// What Swift implements to hear a job's answer. Called from a Rust thread with no lock held, exactly once per
/// job (a job that was cancelled answers with whatever it had, or nothing: the caller drops it).
#[uniffi::export(with_foreign)]
pub trait PickerListener: Send + Sync {
    fn answered(&self, answer: PickerAnswerRecord);
    fn decorated(&self, found: Vec<PickerDecorationRecord>);
}

/// A query or a decoration that is running or waiting for a place.
#[derive(uniffi::Object)]
pub struct PickerJob {
    cancel: Cancel,
    cancelled: AtomicBool,
}

#[uniffi::export]
impl PickerJob {
    /// The text has moved on or the picker closed: the answer is not wanted. Stops a job that waits for a
    /// place at once; a read in progress stops between its steps.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.cancel.cancel();
    }
}

/// The store's `meta` key the recent folders are kept under, as in the GPUI app (`app_state/recent_folders.rs`), so the
/// list of one is the other's.
const RECENTS_KEY: &str = "recent_folders";

/// The recent folders the data folder's database holds, as `recents_json()` writes them (nil for none or no database).
#[uniffi::export]
pub fn folder_recents_load() -> Option<String> {
    crate::isolation::require().ok()?;
    wormhole_core::storage::store::Store::open().ok()?.meta(RECENTS_KEY).ok().flatten()
}

/// Keeps `json` (what `recents_json()` wrote) as the data folder's recent folders. False when it could not be kept.
#[uniffi::export]
pub fn folder_recents_save(json: String) -> bool {
    crate::isolation::require().is_ok()
        && wormhole_core::storage::store::Store::open().is_ok_and(|store| store.set_meta(RECENTS_KEY, &json).is_ok())
}

/// The providers and decorators of one picker, and the recent folders they share.
#[derive(uniffi::Object)]
pub struct FolderPicker {
    providers: Vec<Arc<dyn Provider>>,
    decorators: Vec<Arc<dyn Decorator>>,
    recents: Arc<Mutex<RecentFolders>>,
}

#[uniffi::export]
impl FolderPicker {
    /// `recents_json` is what `recents_json()` wrote before (nil for none); `recent_title` is the words over the
    /// recent folders, which are the app's.
    #[uniffi::constructor]
    pub fn new(recents_json: Option<String>, recent_title: String) -> Arc<Self> {
        let recents = Arc::new(Mutex::new(RecentFolders::from_json(recents_json.as_deref())));
        let parts = parts(recents.clone(), &recent_title);
        Arc::new(Self { providers: parts.providers, decorators: parts.decorators, recents })
    }

    /// How many providers there are; each is asked on its own so a slow one holds back only its rows.
    pub fn provider_count(&self) -> u32 {
        self.providers.len() as u32
    }

    /// How many decorators there are.
    pub fn decorator_count(&self) -> u32 {
        self.decorators.len() as u32
    }

    /// The kinds of row decorator `index` knows something about (none named: every kind).
    pub fn decorator_kinds(&self, index: u32) -> Vec<String> {
        self.decorators
            .get(index as usize)
            .map(|d| d.kinds().iter().map(|k| (*k).to_string()).collect())
            .unwrap_or_default()
    }

    /// Ask provider `index` for `query`; the listener hears `answered` once.
    pub fn ask(
        &self,
        index: u32,
        query: PickerQueryRecord,
        listener: Arc<dyn PickerListener>,
    ) -> Arc<PickerJob> {
        let provider = self.providers.get(index as usize).cloned();
        run(
            listener,
            move |cancel, listener| {
                let answer = provider.map(|p| PickerAnswerRecord::from(&p.query(&query.to_core(), cancel)));
                listener.answered(answer.unwrap_or_else(PickerAnswerRecord::empty));
            },
            no_answer,
        )
    }

    /// Ask decorator `index` about the rows with these keys; the listener hears `decorated` once.
    pub fn decorate(
        &self,
        index: u32,
        keys: Vec<String>,
        listener: Arc<dyn PickerListener>,
    ) -> Arc<PickerJob> {
        let decorator = self.decorators.get(index as usize).cloned();
        run(
            listener,
            move |cancel, listener| {
                let found = decorator
                    .map(|d| d.decorate(&keys, cancel).iter().map(PickerDecorationRecord::from).collect())
                    .unwrap_or_default();
                listener.decorated(found);
            },
            no_decorations,
        )
    }

    /// A terminal was in `path` at `now_secs` (Unix seconds): a visit, unless it is a place never recorded (the
    /// data folder, temporary folders) or a repeat within seconds. Whether the list changed.
    pub fn record_visit(&self, path: String, now_secs: u64) -> bool {
        let keep_out = keep_out(Some(&wormhole_core::storage::paths::data_dir()), &paths::temp());
        self.lock().record(Path::new(&path), now_secs, &keep_out)
    }

    /// Forget a recent folder (⌘⌫ on its row). Whether it was there.
    pub fn forget(&self, path: String) -> bool {
        self.lock().forget(&path)
    }

    /// Forget every recent folder (the flag is off: turning the feature off forgets what it learned).
    pub fn clear(&self) {
        *self.lock() = RecentFolders::default();
    }

    /// The recent folders as the host keeps them (read back by `new`).
    pub fn recents_json(&self) -> String {
        self.lock().to_json()
    }
}

impl FolderPicker {
    fn lock(&self) -> std::sync::MutexGuard<'_, RecentFolders> {
        self.recents.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Start `work` on a thread of its own once the gate has a place: it gets the job's cancel token and the
/// listener. A job that is cancelled before it starts never reads, and `nothing` is its answer; the listener
/// hears exactly one of the two.
fn run(
    listener: Arc<dyn PickerListener>,
    work: impl FnOnce(&Cancel, &dyn PickerListener) + Send + 'static,
    nothing: fn(&dyn PickerListener),
) -> Arc<PickerJob> {
    let job = Arc::new(PickerJob { cancel: Cancel::new(), cancelled: AtomicBool::new(false) });
    let worker = job.clone();
    let heard = listener.clone();
    let spawned = std::thread::Builder::new().name("wormhole-ffi-picker".into()).spawn(move || {
        match gate::enter(|| !worker.cancelled.load(Ordering::Acquire)) {
            Some(_place) => work(&worker.cancel, heard.as_ref()),
            None => nothing(heard.as_ref()),
        }
    });
    if spawned.is_err() {
        nothing(listener.as_ref());
    }
    job
}

fn no_answer(listener: &dyn PickerListener) {
    listener.answered(PickerAnswerRecord::empty());
}

fn no_decorations(listener: &dyn PickerListener) {
    listener.decorated(Vec::new());
}

/// How long a read may take before a front end says it is still reading: the gate's deadline, after which the
/// job also stops counting against the others.
#[uniffi::export]
pub fn folder_picker_reading_after_ms() -> u64 {
    gate::DEADLINE.as_millis() as u64
}

#[cfg(test)]
mod tests;
