//! The `ProjectService` object: the Project tab over core's `project` domain. One service holds the
//! person's switches, the trust store and the notes sync state, as the app's hub does for a profile.
//!
//! Trust is enforced here, in Rust, and not left to the caller: an action, a prompt or the layout runs
//! only through [`Hub::enabled`] (the file is there, the person has it on, the folder's content is
//! approved), judged on a snapshot read at that moment and acted on from the same bytes. A Swift caller
//! can ask for anything; it gets effects only for what trust allows, and `approve` asks the owner first.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use wormhole_core::project::{self as proj, Feature, ProjectFolder};

use super::effects::{self, ProjectEffect};
use super::hub::Hub;
use super::records::{ProjectFailure, ProjectStateRecord};
use super::state;
use super::trust::OwnerAuthenticator;
use crate::error::FfiError;
use crate::files_panel::listener::PanelListener;
use crate::files_panel::watch::FolderWatch;
use crate::profile;

#[derive(uniffi::Object)]
pub struct ProjectService {
    hub: Mutex<Hub>,
    pub(super) auth: Arc<dyn OwnerAuthenticator>,
    /// The watches Swift holds, which hear of what the service itself changed (trust, a switch, a colour).
    watches: Mutex<Vec<Weak<FolderWatch>>>,
}

impl ProjectService {
    /// A service over the stores in `dir`.
    pub(crate) fn in_dir(dir: &Path, auth: Arc<dyn OwnerAuthenticator>) -> Arc<Self> {
        Arc::new(Self { hub: Mutex::new(Hub::load(dir)), auth, watches: Mutex::new(Vec::new()) })
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, Hub> {
        self.hub.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Tells every watch of the service that something changed.
    pub(super) fn changed(&self) {
        let mut watches = self.watches.lock().unwrap_or_else(PoisonError::into_inner);
        watches.retain(|watch| watch.upgrade().inspect(|watch| watch.poke()).is_some());
    }
}

/// The project folder at `root` as given by Swift: a canonical path with a `.wormhole/` in it, else `None`.
/// It does not walk up: only `project` finds a project from a terminal's folder.
pub(super) fn open(root: &str) -> Option<ProjectFolder> {
    ProjectFolder::open(&std::fs::canonicalize(root).ok()?)
}

#[uniffi::export]
impl ProjectService {
    /// Fails unless this is an isolated instance of the experiment (the stores live in its data folder).
    #[uniffi::constructor]
    pub fn new(authenticator: Arc<dyn OwnerAuthenticator>) -> Result<Arc<Self>, FfiError> {
        Ok(Self::in_dir(&profile::active_dir()?, authenticator))
    }

    /// The window this belongs to moved to profile `profile` ("Move this window to..."): the switches, the trust and the
    /// notes sync state are that profile's from here on, as the GPUI app's window reads its own profile's hub. False
    /// (nothing changes) for a profile that is not in `profiles.json`.
    pub fn follow_profile(&self, profile: String) -> bool {
        let Some(dir) = profile::dir_of(&profile) else { return false };
        *self.lock() = Hub::load(&dir);
        self.changed();
        true
    }

    /// The project governing `folder` (a terminal's folder: the first `.wormhole/` above it, stopping
    /// at a repository's root), as the tab draws it; `None` where there is none.
    pub fn project(&self, folder: String) -> Option<ProjectStateRecord> {
        let found = proj::find_project(Path::new(&folder))?;
        let mut hub = self.lock();
        hub.reload_trust();
        let snapshot = hub.snapshot(&found);
        Some(state::build(&mut hub, &snapshot))
    }

    /// Switches `feature` (`settings`, `theme`, `actions`, `prompts`, `layout`, `notes`) on or off for
    /// this project, for this person only.
    pub fn set_feature(&self, root: String, feature: String, on: bool) -> Result<(), ProjectFailure> {
        let feature = Feature::from_key(&feature).ok_or_else(|| ProjectFailure::not_found(feature))?;
        let folder = open(&root).ok_or_else(|| ProjectFailure::not_found(root))?;
        let mut hub = self.lock();
        hub.prefs.set_enabled(&folder.root, feature, on);
        hub.save();
        drop(hub);
        self.changed();
        Ok(())
    }

    /// Makes `folder/.wormhole/` with a `config.json` that sets nothing yet and an empty `notes.md`; it
    /// refuses to touch anything that is already there. Returns the new project's root.
    pub fn create_project(&self, folder: String) -> Result<String, ProjectFailure> {
        let made = proj::init(Path::new(&folder));
        // A second click, or another program, got there first: it is a project all the same.
        if let Err(err) = &made
            && ProjectFolder::open(Path::new(&folder)).is_none()
        {
            let failure = ProjectFailure::failed(err);
            self.lock().error = Some(format!("Couldn't create the project folder: {err}"));
            return Err(failure);
        }
        self.lock().error = None;
        self.changed();
        Ok(folder)
    }

    /// The effects of running action `id` of the project at `root`, for the host to perform. Refused
    /// unless the project's Actions are on and its content is trusted.
    pub fn run_action(&self, root: String, id: String) -> Result<Vec<ProjectEffect>, ProjectFailure> {
        let (_, snapshot) = self.fresh(&root, Feature::Actions)?;
        let effects = effects::of_action(&snapshot, &id);
        if effects.is_empty() {
            return Err(ProjectFailure::not_found(format!("action {id}")));
        }
        Ok(effects)
    }

    /// The effects of `layout.json`: the terminals to open. Refused unless Layout is on and trusted.
    pub fn open_layout(&self, root: String) -> Result<Vec<ProjectEffect>, ProjectFailure> {
        let (folder, snapshot) = self.fresh(&root, Feature::Layout)?;
        Ok(effects::of_layout(&folder, &snapshot))
    }

    /// The effect of pasting prompt `id` into the terminal. Refused unless Prompts are on.
    pub fn send_prompt(&self, root: String, id: String) -> Result<Vec<ProjectEffect>, ProjectFailure> {
        let body = self.prompt_text(root, id.clone())?;
        Ok(effects::of_prompt(&body))
    }

    /// A prompt's text, for copying it into a note. Refused unless Prompts are on.
    pub fn prompt_text(&self, root: String, id: String) -> Result<String, ProjectFailure> {
        let (_, snapshot) = self.fresh(&root, Feature::Prompts)?;
        snapshot
            .prompts()
            .iter()
            .find(|prompt| prompt.id == id)
            .map(|prompt| prompt.body.clone())
            .ok_or_else(|| ProjectFailure::not_found(format!("prompt {id}")))
    }

    /// Follows the project folder of `folder` (its `.wormhole/`, or `folder` for one made later) and the
    /// service's own changes of trust and switches. See `FilesService::watch` for the rules.
    pub fn watch(&self, folder: String, listener: Arc<dyn PanelListener>) -> Arc<FolderWatch> {
        let (root, keep): (_, fn(&Path) -> bool) = match proj::find_project(Path::new(&folder)) {
            Some(found) => (found.dir, |_| true),
            None => (folder.into(), |path| {
                path.components().any(|part| {
                    [proj::DIR_NAME, proj::LEGACY_DIR_NAME].iter().any(|n| part.as_os_str() == *n)
                })
            }),
        };
        let watch = FolderWatch::start(root, listener, keep);
        self.watches.lock().unwrap_or_else(PoisonError::into_inner).push(Arc::downgrade(&watch));
        watch
    }
}

impl ProjectService {
    /// The project at `root`, read now, when `feature` may act: the one gate every effect passes.
    fn fresh(
        &self,
        root: &str,
        feature: Feature,
    ) -> Result<(ProjectFolder, proj::ProjectSnapshot), ProjectFailure> {
        let folder = open(root).ok_or_else(|| ProjectFailure::not_found(root))?;
        // Read here, so what is checked is what runs: the same snapshot gives the trust verdict and the
        // content, and a `git checkout` between the two cannot make them differ.
        let snapshot = folder.snapshot();
        if let Some(reason) = self.lock().refusal(&snapshot, feature) {
            return Err(ProjectFailure::refused(reason));
        }
        Ok((folder, snapshot))
    }
}
