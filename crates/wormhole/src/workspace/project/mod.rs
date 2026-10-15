//! `.wormhole/` project folders: detection per session, trust (approved with
//! Touch ID), per-feature toggles, project settings / env, actions, prompts,
//! session layouts and two-way `notes.md` sync.
//!
//! Every feature is optional and can be switched off per project, so a
//! committed `.wormhole/` never forces anything on a collaborator.
//! Nothing executable runs until the folder is trusted.
//!
//! Here: the hub (one snapshot per project in use) and loading. `detect`: which project each session
//! is in, and the settings, theme, env and shell that follow. `trust`: approval and the feature
//! switches. `actions`: actions, layouts, prompts and action keys. `notes_sync`: notes.md.
//! `create`: making a project folder. `color`: writing its colour. `panel`: the Project tab.

mod actions;
mod color;
mod create;
mod detect;
mod notes_sync;
mod panel;
mod trust;
mod trust_window;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wormhole_core::keyspec::KeySpec;
use wormhole_core::project::{
    Action, Feature, NotesBaseStore, NotesFile, ProjectFolder, ProjectPrefs, ProjectSnapshot, TrustRequest,
    TrustStatus, TrustStore,
};
use wormhole_core::shells::ShellCommand;
use wormhole_core::store::data_dir;
use wormhole_core::terminal::SpawnOptions;

use super::app::{AppState, app, app_mut};
use super::*;

pub(super) use color::set_project_color;
pub(crate) use trust::{approve_project, revoke_project, set_feature_default, set_feature_for};
#[cfg(feature = "devctl")]
pub(crate) use trust_window::devctl_trust_shot;
pub(crate) use trust_window::open_trust_window;

/// Whether the project-folder feature is on (its flag, see core `features`). Everything that reads
/// or writes a `.wormhole/` asks this itself instead of trusting that `refresh_projects` has
/// already cleared the window's projects: a settings change doesn't run that refresh, so for up to
/// a tick after the flag goes off the old projects are still there.
pub(crate) fn project_folders_on(cx: &App) -> bool {
    settings(cx).feature_enabled(wormhole_core::features::FeatureFlag::ProjectFolders)
}

pub(crate) struct ProjectHub {
    pub prefs: ProjectPrefs,
    pub trust: TrustStore,
    pub projects: HashMap<PathBuf, LoadedProject>,
    /// Waiting for the Touch ID / password dialog.
    pub approving: bool,
    pub error: Option<SharedString>,
    /// Settings overrides currently applied (to avoid re-applying every tick).
    applied_overrides: Option<serde_json::Map<String, serde_json::Value>>,
    /// Project theme JSON currently applied.
    applied_theme: Option<String>,
    /// Each session's notes merge base and its project, kept across restarts.
    notes_bases: NotesBaseStore,
    /// Each project's notes.md, read again only when it changed.
    notes_files: HashMap<PathBuf, NotesFile>,
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
    keys: Vec<(KeySpec, String)>,
    pub snapshot: Arc<ProjectSnapshot>,
}

impl ProjectHub {
    pub fn load() -> Self {
        Self::load_from(&data_dir())
    }

    fn load_from(dir: &Path) -> Self {
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
    pub(super) fn ids_start_at(&mut self, first_free: i64) {
        self.notes_bases.forget_from(first_free);
    }

    /// `id` was just handed out: whatever base was kept under it belongs to a session that is gone.
    pub(super) fn id_allocated(&mut self, id: i64) {
        self.notes_bases.remove(id);
    }

    fn save_notes_bases(&mut self) {
        if self.notes_bases.is_dirty()
            && let Err(err) = self.notes_bases.save()
        {
            tracing::warn!("saving notes sync state: {err}");
        }
    }

    /// Persist prefs and trust; a failure is shown where they are (the UI would otherwise say
    /// "Trusted" although nothing was saved).
    fn save(&mut self) {
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

fn load_project(folder: ProjectFolder, trust: &TrustStore) -> LoadedProject {
    LoadedProject::new(Arc::new(folder.snapshot()), trust)
}

impl LoadedProject {
    /// One broken file doesn't stop the rest; its error is listed in the Project tab instead.
    fn new(snapshot: Arc<ProjectSnapshot>, trust: &TrustStore) -> Self {
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

/// Bring the hub's copy of each project in `roots` up to date. A project is read again only when
/// a file it was read from changed (a few `stat`s otherwise); its trust status is recomputed from
/// memory. Returns whether anything a view shows changed.
fn load_projects(roots: impl IntoIterator<Item = PathBuf>, cx: &mut App) -> bool {
    let ProjectHub { projects, trust, .. } = &mut app_mut(cx).hub;
    let mut seen = HashSet::new();
    let mut changed = false;
    let mut upgraded = false;
    for root in roots {
        if !seen.insert(root.clone()) {
            continue;
        }
        match projects.get_mut(&root) {
            Some(project) => {
                if let Some(snapshot) = project.snapshot.reload_if_changed() {
                    *project = LoadedProject::new(Arc::new(snapshot), trust);
                    changed = true;
                } else {
                    let status = trust.status_of(&project.snapshot);
                    changed |= status != project.status;
                    project.status = status;
                }
            }
            None => {
                if let Some(folder) = ProjectFolder::open(&root) {
                    projects.insert(root.clone(), load_project(folder, trust));
                    changed = true;
                }
            }
        }
        // An approval given before a colour stopped counting is still good for the content it was
        // given for: write it in the current form, or the first colour chosen afterwards (here or
        // in the file) would find one it can't recognise, and ask again.
        if let Some(project) = projects.get(&root) {
            upgraded |= trust.upgrade(&project.snapshot);
        }
    }
    if upgraded && let Err(err) = trust.save() {
        tracing::warn!("saving trusted projects after upgrading them: {err}");
    }
    changed
}

/// Forget projects no session in any window uses any more, and the notes bases of closed sessions.
pub(super) fn prune_projects(cx: &mut App) {
    let open = AppState::open_project_roots(cx);
    let sessions = open_session_ids(cx);
    let hub = &mut app_mut(cx).hub;
    hub.projects.retain(|root, _| open.contains(root));
    hub.notes_files.retain(|root, _| open.contains(root));
    hub.notes_errors.retain(|root, _| open.contains(root));
    if let Some(sessions) = sessions {
        hub.notes_bases.retain(|session| sessions.contains(&session));
    }
    hub.save_notes_bases();
}

/// Every session id in every window, or `None` when that can't be known for sure (quitting, or a
/// window that can't be read): forgetting a base too early would bring deletions back.
fn open_session_ids(cx: &App) -> Option<HashSet<i64>> {
    let state = app(cx);
    if state.quitting || state.windows.is_empty() {
        return None;
    }
    let mut ids = HashSet::new();
    for window in &state.windows {
        let sessions =
            window.workspace.read_with(cx, |ws, _| ws.sessions.iter().map(|s| s.id).collect::<Vec<_>>());
        ids.extend(sessions.ok()?);
    }
    Some(ids)
}

/// Preferences or trust changed: every window re-reads its projects.
fn projects_changed(cx: &mut App) {
    app_mut(cx).hub.save();
    reload_projects(cx);
}

/// Every window looks again for the project each of its terminals is in (one was made, or what a
/// view shows of one changed).
fn reload_projects(cx: &mut App) {
    // Deferred: this may run inside a window's own update.
    cx.defer(|cx| {
        for window in app(cx).windows.clone() {
            AppState::with_workspace(&window, cx, |ws, _, cx| ws.refresh_projects(cx));
        }
    });
    cx.refresh_windows();
}

/// Projects worth offering in Settings: open sessions', recent and trusted
/// ones that still have a `.wormhole/` folder, most relevant first.
pub(crate) fn known_projects(cx: &App) -> Vec<PathBuf> {
    let state = app(cx);
    let mut roots: Vec<PathBuf> = Vec::new();
    let candidates = AppState::open_project_roots(cx)
        .into_iter()
        .chain(state.menu.recent().iter().cloned())
        .chain(state.hub.trust.list().into_iter().map(|p| p.root.clone()));
    for root in candidates {
        if !roots.contains(&root) && ProjectFolder::open(&root).is_some() {
            roots.push(root);
        }
    }
    roots
}

/// A project's files as loaded for display: the hub's snapshot when a session uses the project (so
/// Settings shows what trusting it would approve), else read now.
pub(crate) fn load_project_info(root: &Path, cx: &App) -> Option<LoadedProject> {
    let hub = &app(cx).hub;
    match hub.projects.get(root) {
        Some(project) => Some(project.clone()),
        None => ProjectFolder::open(root).map(|folder| load_project(folder, &hub.trust)),
    }
}

pub(crate) type BTreeMapEnv = std::collections::BTreeMap<String, String>;

pub(super) fn spawn_options(
    tab_id: i64,
    cwd: Option<PathBuf>,
    restore: Option<String>,
    env: BTreeMapEnv,
    shell: ShellCommand,
) -> SpawnOptions {
    SpawnOptions {
        cwd,
        tab_id: tab_id.to_string(),
        restore,
        scrollback_lines: 0,
        allow_screen_checksum: false,
        allow_window_ops: false,
        env,
        shell: Some(shell),
        welcome: false,
        colors: None,
    }
}
