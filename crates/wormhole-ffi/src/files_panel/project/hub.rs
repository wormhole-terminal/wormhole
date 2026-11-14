//! What the service knows of the project folders it has looked at: the person's switches, the trust
//! store, the notes sync's bases and files. The app's `ProjectHub` in one place, over core's stores.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use wormhole_core::project::{
    Feature, NotesBaseStore, NotesFile, ProjectFolder, ProjectPrefs, ProjectSnapshot, TrustRequest,
    TrustStatus, TrustStore,
};

pub(super) struct Hub {
    /// The profile folder the stores live in.
    dir: PathBuf,
    pub(super) prefs: ProjectPrefs,
    pub(super) trust: TrustStore,
    /// The profile's, shared by every window of it (`shared_bases`).
    notes_bases: Arc<Mutex<NotesBaseStore>>,
    pub(super) notes_files: HashMap<PathBuf, NotesFile>,
    /// Why a project's notes.md can't be synced right now, shown in the Project tab.
    pub(super) notes_errors: HashMap<PathBuf, String>,
    /// What the tab last showed of each project: approving trusts exactly this, so a file changed while
    /// the person looked reads "changed" instead of being approved unseen.
    pub(super) shown: HashMap<PathBuf, TrustRequest>,
    /// The last snapshot of each project, read again only when a file it came from changed (display only:
    /// what acts is always read fresh).
    snapshots: HashMap<PathBuf, Arc<ProjectSnapshot>>,
    /// Waiting for the owner's authentication.
    pub(super) approving: bool,
    pub(super) error: Option<String>,
}

impl Hub {
    /// The profile folder the stores live in.
    pub(super) fn dir(&self) -> &Path {
        &self.dir
    }

    pub(super) fn load(dir: &Path) -> Self {
        let mut problems = Vec::new();
        let (prefs, err) = ProjectPrefs::load_or_recover(dir);
        problems.extend(err.map(|err| format!("Project preferences were reset: {err}")));
        let (trust, err) = TrustStore::load_or_recover(dir);
        problems.extend(err.map(|err| format!("Trusted projects were reset, approve them again: {err}")));
        let notes_bases = shared_bases(dir);
        for problem in &problems {
            tracing::warn!("{problem}");
        }
        Self {
            dir: dir.to_path_buf(),
            prefs,
            trust,
            notes_bases,
            notes_files: HashMap::new(),
            notes_errors: HashMap::new(),
            shown: HashMap::new(),
            snapshots: HashMap::new(),
            approving: false,
            error: (!problems.is_empty()).then(|| problems.join("\n")),
        }
    }

    /// The approvals as the file has them now: the project commands window approves through a desk of its own, and
    /// the tab must say what it did.
    pub(super) fn reload_trust(&mut self) {
        let (trust, err) = TrustStore::load_or_recover(&self.dir);
        match err {
            Some(err) => tracing::warn!("trusted projects were not read again: {err}"),
            None => self.trust = trust,
        }
    }

    /// What approving `folder` would trust: what the tab last showed of it, else what it says now; the
    /// reason when a file does not load.
    pub(super) fn approval_request(&mut self, folder: &ProjectFolder) -> Result<TrustRequest, String> {
        if let Some(request) = self.shown.get(&folder.root) {
            return Ok(request.clone());
        }
        let snapshot = self.snapshot(folder);
        snapshot
            .trust_request()
            .ok_or_else(|| snapshot.errors().next().map(|err| err.to_string()).unwrap_or_default())
    }

    /// The project's snapshot for showing: kept while no file it was read from changed.
    pub(super) fn snapshot(&mut self, folder: &ProjectFolder) -> Arc<ProjectSnapshot> {
        let current = self.snapshots.get(&folder.root).is_some_and(|snapshot| snapshot.is_current());
        if !current {
            self.snapshots.insert(folder.root.clone(), Arc::new(folder.snapshot()));
        }
        let snapshot = Arc::clone(&self.snapshots[&folder.root]);
        // An approval given under an older fingerprint is rewritten once, as the app does at load.
        if self.trust.upgrade(&snapshot) {
            self.save_trust();
        }
        snapshot
    }

    /// Why `feature` may not act on `snapshot`, or `None` when it may: present in the folder, switched on by
    /// the person, and trusted if it needs to be. Judged on the content that would run, never the disk
    /// again. The one rule every action, prompt and layout passes (the app's `ProjectHub::enabled`).
    pub(super) fn refusal(&self, snapshot: &ProjectSnapshot, feature: Feature) -> Option<String> {
        if !snapshot.has(feature) {
            return Some(format!("this project has no {}", feature.file_name()));
        }
        if !self.prefs.enabled(snapshot.root(), feature) {
            return Some(format!("{} is switched off for this project", feature.label()));
        }
        if !self.trust.allows_in(snapshot, feature) {
            return Some(
                "the project is not trusted (its files changed, or were never approved)".to_string(),
            );
        }
        None
    }

    pub(super) fn enabled(&self, snapshot: &ProjectSnapshot, feature: Feature) -> bool {
        self.refusal(snapshot, feature).is_none()
    }

    /// The folder's current content is approved (what lets a notes.md be read and written).
    pub(super) fn trusted(&self, snapshot: &ProjectSnapshot) -> bool {
        self.trust.status_of(snapshot) == TrustStatus::Trusted
    }

    /// Persists the person's switches and the approvals; a failure is said in the tab, which would
    /// otherwise read "trusted" while nothing was saved.
    pub(super) fn save(&mut self) {
        if let Err(err) = self.prefs.save() {
            tracing::warn!("saving project preferences: {err}");
            self.error = Some(format!("Couldn't save your project settings: {err}"));
        }
        self.save_trust();
    }

    fn save_trust(&mut self) {
        if let Err(err) = self.trust.save() {
            tracing::warn!("saving trusted projects: {err}");
            self.error = Some(format!("Couldn't save trusted projects: {err}"));
        }
    }

    /// Notes why `root`'s notes.md can't be synced (`None`: it can). Whether what the tab says changed.
    pub(super) fn set_notes_error(&mut self, root: &Path, error: Option<String>) -> bool {
        match error {
            Some(error) if self.notes_errors.get(root) == Some(&error) => false,
            Some(error) => {
                self.notes_errors.insert(root.to_path_buf(), error);
                true
            }
            None => self.notes_errors.remove(root).is_some(),
        }
    }

    /// The notes sync's merge bases, every window's of the profile.
    pub(super) fn bases(&self) -> MutexGuard<'_, NotesBaseStore> {
        self.notes_bases.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn save_notes_bases(&mut self) {
        let mut bases = self.bases();
        if bases.is_dirty()
            && let Err(err) = bases.save()
        {
            tracing::warn!("saving notes sync state: {err}");
        }
    }
}

/// The merge bases kept in profile folder `dir`: one store in the process for every window of that profile, as the GPUI app
/// keeps one hub per profile. A copy per window wrote over the other windows' bases in `notes-sync.json` (the next launch
/// then merged their terminals against stale ones), and a terminal moved to another window merged against none there.
fn shared_bases(dir: &Path) -> Arc<Mutex<NotesBaseStore>> {
    static BASES: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<NotesBaseStore>>>>> =
        LazyLock::new(Default::default);
    let mut all = BASES.lock().unwrap_or_else(PoisonError::into_inner);
    let store = all.entry(dir.to_path_buf()).or_insert_with(|| {
        let (bases, err) = NotesBaseStore::load_or_recover(dir);
        if let Some(err) = err {
            tracing::warn!("notes sync state: {err}");
        }
        Arc::new(Mutex::new(bases))
    });
    Arc::clone(store)
}
