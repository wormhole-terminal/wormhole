//! Keeping the hub's copy of the projects in use current: reading them again when a file changed,
//! forgetting the ones no terminal uses, and telling every window when preferences or trust changed.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::*;
use wormhole_core::project::ProjectFolder;

use super::hub::{LoadedProject, ProjectHub, load_project};
use super::{hub, hub_mut};
use crate::app_state::{AppState, app};

/// Bring the hub's copy of each project in `roots` up to date. A project is read again only when
/// a file it was read from changed (a few `stat`s otherwise); its trust status is recomputed from
/// memory. Returns whether anything a view shows changed.
pub(super) fn load_projects(roots: impl IntoIterator<Item = PathBuf>, cx: &mut App) -> bool {
    let ProjectHub { projects, trust, .. } = &mut hub_mut(cx);
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

/// Projects with a terminal open in any window.
fn open_roots(cx: &App) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for window in &app(cx).windows {
        if let Ok(found) = window.workspace.read_with(cx, super::api::roots) {
            roots.extend(found);
        }
    }
    roots
}

/// Forget projects no session in any window uses any more, and the notes bases of closed sessions.
pub(crate) fn prune_projects(cx: &mut App) {
    let open = open_roots(cx);
    let sessions = open_session_ids(cx);
    let hub = &mut hub_mut(cx);
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
pub(super) fn projects_changed(cx: &mut App) {
    hub_mut(cx).save();
    reload_projects(cx);
}

/// Every window looks again for the project each of its terminals is in (one was made, or what a
/// view shows of one changed).
pub(super) fn reload_projects(cx: &mut App) {
    // Deferred: this may run inside a window's own update.
    cx.defer(|cx| {
        for window in app(cx).windows.clone() {
            AppState::with_workspace(&window, cx, |ws, _, cx| ws.refresh_features(cx));
        }
    });
    cx.refresh_windows();
}

/// Projects worth offering in Settings: open sessions', recent and trusted
/// ones that still have a `.wormhole/` folder, most relevant first.
pub(crate) fn known_projects(cx: &App) -> Vec<PathBuf> {
    let state = app(cx);
    let mut roots: Vec<PathBuf> = Vec::new();
    let candidates = open_roots(cx)
        .into_iter()
        .chain(state.recent_projects.roots().iter().cloned())
        .chain(hub(cx).trust.list().into_iter().map(|p| p.root.clone()));
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
    let hub = hub(cx);
    match hub.projects.get(root) {
        Some(project) => Some(project.clone()),
        None => ProjectFolder::open(root).map(|folder| load_project(folder, &hub.trust)),
    }
}
