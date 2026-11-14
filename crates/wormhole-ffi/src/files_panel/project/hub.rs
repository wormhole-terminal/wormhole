//! What the service knows of the project folders it has looked at: the person's switches, the trust store and the
//! notes sync's bases (the profile's, shared by its windows: `stores`), and what this window showed and was told. The
//! app's `ProjectHub` in one place, over core's stores.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, MutexGuard};

use wormhole_core::project::{
    Feature, NotesBaseStore, ProjectFolder, ProjectPrefs, ProjectSnapshot, TrustRequest, TrustStore,
};

use super::stores::{self, OnDisk, Stores};

pub(super) struct Hub {
    /// The profile folder the stores live in.
    dir: PathBuf,
    /// The profile's, shared by every window of it.
    stores: Stores,
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
        let stores = stores::of_profile(dir);
        Self {
            dir: dir.to_path_buf(),
            error: stores.problems.clone(),
            stores,
            notes_errors: HashMap::new(),
            shown: HashMap::new(),
            snapshots: HashMap::new(),
            approving: false,
        }
    }

    /// The profile's stores, to hold their locks while this hub is changed (the notes sync).
    pub(super) fn stores(&self) -> Stores {
        self.stores.clone()
    }

    /// The person's switches as the profile has them now (another window's change included).
    pub(super) fn prefs(&self) -> MutexGuard<'_, OnDisk<ProjectPrefs>> {
        stores::now(&self.stores.prefs)
    }

    /// The approvals as the profile has them now: a project revoked in another window, in Settings or by the command
    /// line reads revoked here at once.
    pub(super) fn trust(&self) -> MutexGuard<'_, OnDisk<TrustStore>> {
        stores::now(&self.stores.trust)
    }

    /// Changes the person's switches as the file has them now and saves them, under the profile's lock: what another
    /// window switched stays as it switched it. A failure is said in the tab.
    pub(super) fn change_prefs(&mut self, change: impl FnOnce(&mut ProjectPrefs)) {
        let prefs = Arc::clone(&self.stores.prefs);
        let saved = stores::lock(&prefs).change(|prefs| {
            change(prefs);
            true
        });
        if let Err(err) = saved {
            tracing::warn!("saving project preferences: {err}");
            self.error = Some(format!("Couldn't save your project settings: {err}"));
        }
    }

    /// Changes the approvals as the file has them now and saves them when `change` says it changed them, under the
    /// profile's lock: an approval never writes back a project another window revoked. A failure is said in the tab,
    /// which would otherwise read "trusted" while nothing was saved.
    pub(super) fn change_trust(&mut self, change: impl FnOnce(&mut TrustStore) -> bool) {
        let trust = Arc::clone(&self.stores.trust);
        let saved = stores::lock(&trust).change(change);
        if let Err(err) = saved {
            tracing::warn!("saving trusted projects: {err}");
            self.error = Some(format!("Couldn't save trusted projects: {err}"));
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
        self.change_trust(|trust| trust.upgrade(&snapshot));
        snapshot
    }

    /// Why `feature` may not act on `snapshot`, or `None` when it may: present in the folder, switched on by
    /// the person, and trusted if it needs to be. Judged on the content that would run, never the disk
    /// again. The one rule every action, prompt and layout passes (the app's `ProjectHub::enabled`).
    pub(super) fn refusal(&self, snapshot: &ProjectSnapshot, feature: Feature) -> Option<String> {
        if !snapshot.has(feature) {
            return Some(format!("this project has no {}", feature.file_name()));
        }
        if !self.prefs().enabled(snapshot.root(), feature) {
            return Some(format!("{} is switched off for this project", feature.label()));
        }
        if !self.trust().allows_in(snapshot, feature) {
            return Some(
                "the project is not trusted (its files changed, or were never approved)".to_string(),
            );
        }
        None
    }

    pub(super) fn enabled(&self, snapshot: &ProjectSnapshot, feature: Feature) -> bool {
        self.refusal(snapshot, feature).is_none()
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
        stores::lock(&self.stores.bases)
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
