//! The project stores of a profile, one in the process for every window of it, as the GPUI app keeps one `ProjectHub`
//! per profile (`features/project/hubs.rs`): the person's switches, the approvals and the notes sync's bases. A copy per
//! window, saved whole, wrote one window's stale copy over another's change: a project revoked in one window was trusted
//! again when a second window approved any project, and a switch turned off in one came back on when the other saved.
//! Now a change is made under the store's one lock, to what its file holds at that moment, and saved at once; a read
//! looks at the file's stamp first and reads it again when another program wrote it (the command line, the project
//! commands window's desk).
//!
//! A project's `notes.md` has one lock in the process too ([`notes_file`]), held from its read to its write: two windows
//! merging terminals of the same project take turns, and neither writes what it merged from a read the other's write
//! made stale.

use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use wormhole_core::project::{
    NotesBaseStore, NotesFile, ProjectError, ProjectFolder, ProjectPrefs, TrustStore,
};
use wormhole_core::storage::files::FileStamp;

/// A store kept as a file in the profile folder.
pub(super) trait Stored: Sized {
    const FILE: &'static str;
    fn read_from(dir: &Path) -> Result<Self, ProjectError>;
    fn write(&self) -> Result<(), ProjectError>;
}

impl Stored for ProjectPrefs {
    const FILE: &'static str = ProjectPrefs::FILE_NAME;
    fn read_from(dir: &Path) -> Result<Self, ProjectError> {
        ProjectPrefs::load(dir)
    }
    fn write(&self) -> Result<(), ProjectError> {
        self.save()
    }
}

impl Stored for TrustStore {
    const FILE: &'static str = TrustStore::FILE_NAME;
    fn read_from(dir: &Path) -> Result<Self, ProjectError> {
        TrustStore::load(dir)
    }
    fn write(&self) -> Result<(), ProjectError> {
        self.save()
    }
}

/// A store as its file has it.
pub(super) struct OnDisk<T> {
    value: T,
    dir: PathBuf,
    /// The file's stamp taken before the read `value` came from; meaningless while `known` is false (after a save, and
    /// before the first look), when the next look reads the file again.
    stamp: Option<FileStamp>,
    known: bool,
}

impl<T: Stored> OnDisk<T> {
    fn new(dir: &Path, value: T) -> Self {
        Self { value, dir: dir.to_path_buf(), stamp: None, known: false }
    }

    /// Reads the file again when it changed since the last read: one `lstat` when it did not.
    pub(super) fn fresh(&mut self) {
        let stamp = FileStamp::of(&self.dir.join(T::FILE));
        if self.known && stamp == self.stamp {
            return;
        }
        match T::read_from(&self.dir) {
            Ok(value) => self.value = value,
            // What the process holds stays: every change this process made is in it. Said once per version of the file.
            Err(err) => tracing::warn!("{} not read again: {err}", T::FILE),
        }
        (self.stamp, self.known) = (stamp, true);
    }

    /// `change` made to what the file holds now, and saved when it says it changed something. Whether it did; a save that
    /// failed leaves the change in the process (it holds until the file changes or the app quits) and says why.
    pub(super) fn change(&mut self, change: impl FnOnce(&mut T) -> bool) -> Result<bool, ProjectError> {
        self.fresh();
        if !change(&mut self.value) {
            return Ok(false);
        }
        let saved = self.value.write();
        // After a save: read again at the next look, so a write another program made just after this one is never taken
        // for this one. After a failed one the file is as `fresh` found it, so the stamp still stands and the change
        // holds here until the file changes: a revoke that could not be saved must not be read back as trusted.
        self.known = saved.is_err();
        saved.map(|()| true)
    }
}

impl<T> Deref for OnDisk<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

/// The stores of one profile folder, shared by every window of it.
#[derive(Clone)]
pub(super) struct Stores {
    pub(super) prefs: Arc<Mutex<OnDisk<ProjectPrefs>>>,
    pub(super) trust: Arc<Mutex<OnDisk<TrustStore>>>,
    pub(super) bases: Arc<Mutex<NotesBaseStore>>,
    /// What was wrong with the files when they were first read (set aside and started afresh), said in every window's tab.
    pub(super) problems: Option<String>,
}

/// The stores kept in profile folder `dir`: the same ones for every window of that profile, however the folder is named.
pub(super) fn of_profile(dir: &Path) -> Stores {
    static ALL: LazyLock<Mutex<HashMap<PathBuf, Stores>>> = LazyLock::new(Default::default);
    let key = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    lock(&ALL).entry(key).or_insert_with(|| load(dir)).clone()
}

/// [`OnDisk::change`] on the approvals of profile folder `dir`, for a desk that has no window's service (the project
/// commands window): under the same lock as every window's own changes, and every window's Project tab told. Whether it
/// changed them.
pub(crate) fn change_trust_in(
    dir: &Path,
    change: impl FnOnce(&mut TrustStore) -> bool,
) -> Result<bool, ProjectError> {
    let trust = of_profile(dir).trust;
    let changed = lock(&trust).change(change);
    super::service::tell_every_tab();
    changed
}

fn load(dir: &Path) -> Stores {
    let mut problems = Vec::new();
    let (prefs, err) = ProjectPrefs::load_or_recover(dir);
    problems.extend(err.map(|err| format!("Project preferences were reset: {err}")));
    let (trust, err) = TrustStore::load_or_recover(dir);
    problems.extend(err.map(|err| format!("Trusted projects were reset, approve them again: {err}")));
    let (bases, err) = NotesBaseStore::load_or_recover(dir);
    if let Some(err) = err {
        tracing::warn!("notes sync state: {err}");
    }
    for problem in &problems {
        tracing::warn!("{problem}");
    }
    Stores {
        prefs: Arc::new(Mutex::new(OnDisk::new(dir, prefs))),
        trust: Arc::new(Mutex::new(OnDisk::new(dir, trust))),
        bases: Arc::new(Mutex::new(bases)),
        problems: (!problems.is_empty()).then(|| problems.join("\n")),
    }
}

/// `store` locked, as its file has it now.
pub(super) fn now<T: Stored>(store: &Mutex<OnDisk<T>>) -> MutexGuard<'_, OnDisk<T>> {
    let mut held = lock(store);
    held.fresh();
    held
}

/// The `notes.md` of `folder`, one in the process whichever window or profile merges with it: lock it from the read to
/// the write.
pub(super) fn notes_file(folder: &ProjectFolder) -> Arc<Mutex<NotesFile>> {
    static ALL: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<NotesFile>>>>> = LazyLock::new(Default::default);
    let mut all = lock(&ALL);
    let file = all
        .entry(folder.notes_path())
        .or_insert_with(|| Arc::new(Mutex::new(NotesFile::new(folder.clone()))));
    Arc::clone(file)
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
