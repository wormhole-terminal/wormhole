//! The `ProjectService` object: the Project tab over core's `project` domain. One service holds the
//! person's switches, the trust store and the notes sync state, as the app's hub does for a profile.
//!
//! Trust is enforced here, in Rust, and not left to the caller: an action, a prompt or the layout runs
//! only through [`Hub::enabled`] (the file is there, the person has it on, the folder's content is
//! approved), judged on a snapshot read at that moment and acted on from the same bytes. A Swift caller
//! can ask for anything; it gets effects only for what trust allows, and `approve` asks the owner first.

use std::ops::Not as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use wormhole_core::keys::keyspec::KeySpec;
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
    /// The Project feature is off for the window (its flag, or the person's switch): no `.wormhole/` is read
    /// for effect or written (the GPUI app's `project_folders_on`).
    off: AtomicBool,
}

impl ProjectService {
    /// A service over the stores in `dir`.
    pub(crate) fn in_dir(dir: &Path, auth: Arc<dyn OwnerAuthenticator>) -> Arc<Self> {
        Arc::new(Self {
            hub: Mutex::new(Hub::load(dir)),
            auth,
            watches: Mutex::new(Vec::new()),
            off: AtomicBool::new(false),
        })
    }

    /// Whether the Project feature is off for the window: asked first by everything that would act on a project.
    pub(super) fn off(&self) -> bool {
        self.off.load(Ordering::Relaxed)
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

/// The words of a refused worktree: one rule for the sheet (`ProjectService`) and the command line.
const WORKTREE_REFUSAL: &str =
    "Trust the project first: a terminal in its worktree would run with the project's settings";

/// Why a worktree may not be made of `repo` for the profile whose stores are in `dir`, read from the files as they are now
/// (the command line has no window's service to ask). The caller has already checked that the project folders feature is on.
pub(crate) fn worktree_refusal_in(dir: &Path, repo: &Path) -> Option<String> {
    let found = proj::find_project(repo)?;
    let snapshot = found.snapshot();
    Hub::load(dir).enabled(&snapshot, Feature::Settings).not().then(|| WORKTREE_REFUSAL.into())
}

impl ProjectService {
    pub(crate) fn worktree_refusal(&self, repo: &Path) -> Option<String> {
        // Off: the new terminal takes nothing from the project, so there is no condition.
        if self.off() {
            return None;
        }
        let found = proj::find_project(repo)?;
        let snapshot = found.snapshot();
        let mut hub = self.lock();
        hub.reload_trust();
        if hub.enabled(&snapshot, Feature::Settings) {
            return None;
        }
        Some(WORKTREE_REFUSAL.into())
    }
}

/// The project folder at `root` as given by Swift: a canonical path with a `.wormhole/` in it, else `None`.
/// It does not walk up: only `project` finds a project from a terminal's folder.
pub(super) fn open(root: &str) -> Option<ProjectFolder> {
    ProjectFolder::open(&std::fs::canonicalize(root).ok()?)
}

impl ProjectService {
    /// The stores of the profile folder `dir`, read in place of the ones in use (split off so a test needs no profile list).
    pub(crate) fn follow_dir(&self, dir: &std::path::Path) {
        *self.lock() = Hub::load(dir);
        self.changed();
    }
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
        self.follow_dir(&dir);
        true
    }

    /// The window's Project feature is on or off (core's feature mask, which the window draws its tabs from). Off,
    /// nothing reads a `.wormhole/` for effect or writes one: no settings or theme laid over, no action, layout or
    /// prompt, no worktree condition or setup, no notes.md sync, and a folder's colour is the person's own. On
    /// until told otherwise.
    pub fn set_project_feature_on(&self, on: bool) {
        if self.off.swap(!on, Ordering::Relaxed) == on {
            self.changed();
        }
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

    /// What went wrong with the person's own stores (preferences or approvals reset, a save that failed), which
    /// the tab says where there is no project too (`panel.rs` `render_no_project`); `None` when nothing did.
    pub fn problem(&self) -> Option<String> {
        self.lock().error.clone()
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

    /// The id of the action of the project at `root` whose key is `glyphs` (Apple's notation: `\u{21E7}\u{2318}R`), asked of the
    /// core at the moment of the key: `None` unless the project is enabled and trusted now with Actions on (the key is then
    /// nobody's and goes on to the terminal), as the GPUI app's `action_for_key` answers.
    pub fn action_for_key(&self, root: String, glyphs: String) -> Option<String> {
        let pressed = KeySpec::from_glyphs(&glyphs)?;
        let (_, snapshot) = self.fresh(&root, Feature::Actions).ok()?;
        let action =
            snapshot.actions().iter().find(|a| a.key_spec().is_some_and(|key| key.same_key(&pressed)))?;
        Some(action.id.clone())
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

    /// Why a worktree may not be made of `repo` (`project::api::worktree_allowed`): a project there must be trusted with
    /// its Settings on, since the new terminal takes the project's env and shell; no project, no condition. `None`: it may, as
    /// it does with the Project feature off (`set_project_feature_on`).
    pub fn worktree_allowed(&self, repo: String) -> Option<String> {
        self.worktree_refusal(Path::new(&repo))
    }

    /// What a new worktree of `repo` runs first: its trusted project's `worktree.setup`, under the same conditions as
    /// [`Self::worktree_allowed`]; a project that is not trusted with its Settings on runs nothing.
    pub fn worktree_setup(&self, repo: String) -> Option<String> {
        if self.off() {
            return None;
        }
        let found = proj::find_project(Path::new(&repo))?;
        let snapshot = found.snapshot();
        let mut hub = self.lock();
        hub.reload_trust();
        if !hub.enabled(&snapshot, Feature::Settings) {
            return None;
        }
        snapshot.config()?.worktree_setup.clone()
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
        if self.off() {
            return Err(ProjectFailure::refused("project folders are switched off"));
        }
        let folder = open(root).ok_or_else(|| ProjectFailure::not_found(root))?;
        // Read here, so what is checked is what runs: the same snapshot gives the trust verdict and the
        // content, and a `git checkout` between the two cannot make them differ.
        let snapshot = folder.snapshot();
        let mut hub = self.lock();
        hub.reload_trust();
        if let Some(reason) = hub.refusal(&snapshot, feature) {
            return Err(ProjectFailure::refused(reason));
        }
        Ok((folder, snapshot))
    }
}
