//! The hub: what the app knows of the project folders it has looked at, for one profile. The
//! preferences (which parts are switched on, for everyone and for each project), the trust store, the
//! loaded snapshot of each project in use, and the notes sync's bases and files. [`LoadedProject`] is one
//! project as the app uses it. Which profile's hub is the shown one is `hubs`' business.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::SharedString;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::project::{
    Action, Feature, NotesBaseStore, NotesFile, ProjectFolder, ProjectPrefs, ProjectSnapshot, TrustRequest,
    TrustStatus, TrustStore,
};

#[derive(Default)]
pub(crate) struct ProjectHub {
    pub prefs: ProjectPrefs,
    pub trust: TrustStore,
    pub projects: HashMap<PathBuf, LoadedProject>,
    /// Waiting for the Touch ID / password dialog.
    pub approving: bool,
    pub error: Option<SharedString>,
    /// Settings overrides currently applied (to avoid re-applying every tick).
    pub(super) applied_overrides: Option<serde_json::Map<String, serde_json::Value>>,
    /// Project theme JSON currently applied.
    pub(super) applied_theme: Option<String>,
    /// Each session's notes merge base and its project, kept across restarts.
    pub(super) notes_bases: NotesBaseStore,
    /// Each project's notes.md, read again only when it changed.
    pub(super) notes_files: HashMap<PathBuf, NotesFile>,
    /// Why a project's notes.md can't be synced right now (reported once, shown in the Project tab).
    pub notes_errors: HashMap<PathBuf, String>,
}

/// A project folder as the app uses it. Everything comes from one [`ProjectSnapshot`]: what the
/// Project tab shows, what trust is checked against, and what actions, layouts and new sessions run.
#[derive(Clone)]
pub(crate) struct LoadedProject {
    pub folder: ProjectFolder,
    pub status: TrustStatus,
    /// What trusting would enable (shown in the approval card); `None` while a file is broken.
    pub request: Option<TrustRequest>,
    /// Problems loading the folder's files, shown in the Project tab.
    pub errors: Vec<String>,
    /// What loaded but was set aside in part (an action's key that is taken), shown there too.
    pub warnings: Vec<String>,
    /// Action keys, parsed once (they are checked on every modified key press).
    pub(super) keys: Vec<(KeySpec, String)>,
    pub snapshot: Arc<ProjectSnapshot>,
}

impl ProjectHub {
    pub(crate) fn load_from(dir: &Path) -> Self {
        // A file that doesn't load is kept aside as `.bad`; say so where trust is shown.
        let mut problems = Vec::new();
        let (prefs, err) = ProjectPrefs::load_or_recover(dir);
        problems.extend(err.map(|err| format!("Project preferences were reset: {err}")));
        let (trust, err) = TrustStore::load_or_recover(dir);
        problems.extend(err.map(|err| format!("Trusted projects were reset, approve them again: {err}")));
        let (notes_bases, err) = NotesBaseStore::load_or_recover(dir);
        if let Some(err) = err {
            tracing::warn!("notes sync state: {err}");
        }
        for problem in &problems {
            tracing::warn!("{problem}");
        }
        Self {
            prefs,
            trust,
            projects: HashMap::new(),
            approving: false,
            error: (!problems.is_empty()).then(|| problems.join("\n").into()),
            applied_overrides: None,
            applied_theme: None,
            notes_bases,
            notes_files: HashMap::new(),
            notes_errors: HashMap::new(),
        }
    }

    /// Present in the folder, switched on by the user, and trusted if it needs to be. Checks
    /// the loaded snapshot (the content that would run), never the disk.
    pub fn enabled(&self, project: &LoadedProject, feature: Feature) -> bool {
        project.snapshot.has(feature)
            && self.prefs.enabled(&project.folder.root, feature)
            && self.trust.allows_in(&project.snapshot, feature)
    }

    /// The folder's current content is approved. What starts a program for a project's sake (`gh`,
    /// for its branch's pull request) asks this rather than [`Self::enabled`]: it is a feature of
    /// none of the project's files, only a thing the person lets happen in a folder they trust.
    pub fn trusted(&self, project: &LoadedProject) -> bool {
        self.trust.status_of(&project.snapshot) == TrustStatus::Trusted
    }

    /// Ids from `first_free` on are handed out to new sessions and splits from now on; see
    /// [`NotesBaseStore::forget_from`].
    pub(crate) fn ids_start_at(&mut self, first_free: i64) {
        self.notes_bases.forget_from(first_free);
    }

    /// `id` was just handed out: whatever base was kept under it belongs to a session that is gone.
    pub(crate) fn id_allocated(&mut self, id: i64) {
        self.notes_bases.remove(id);
    }

    pub(super) fn save_notes_bases(&mut self) {
        if self.notes_bases.is_dirty()
            && let Err(err) = self.notes_bases.save()
        {
            tracing::warn!("saving notes sync state: {err}");
        }
    }

    /// Persist prefs and trust; a failure is shown where they are (the UI would otherwise say
    /// "Trusted" although nothing was saved).
    pub(super) fn save(&mut self) {
        if let Err(err) = self.prefs.save() {
            tracing::warn!("saving project preferences: {err}");
            self.error = Some(format!("Couldn't save your project settings: {err}").into());
        }
        if let Err(err) = self.trust.save() {
            tracing::warn!("saving trusted projects: {err}");
            self.error = Some(format!("Couldn't save trusted projects: {err}").into());
        }
    }
}

pub(crate) fn load_project(folder: ProjectFolder, trust: &TrustStore) -> LoadedProject {
    LoadedProject::new(Arc::new(folder.snapshot()), trust)
}

impl LoadedProject {
    /// One broken file doesn't stop the rest; its error is listed in the Project tab instead.
    pub(super) fn new(snapshot: Arc<ProjectSnapshot>, trust: &TrustStore) -> Self {
        Self {
            folder: snapshot.folder().clone(),
            status: trust.status_of(&snapshot),
            request: snapshot.trust_request(),
            errors: snapshot.errors().map(|err| err.to_string()).collect(),
            warnings: snapshot.warnings().iter().map(|warning| warning.to_string()).collect(),
            keys: snapshot.actions().iter().filter_map(|a| Some((a.key_spec()?, a.id.clone()))).collect(),
            snapshot,
        }
    }

    pub fn actions(&self) -> &[Action] {
        self.snapshot.actions()
    }
}
