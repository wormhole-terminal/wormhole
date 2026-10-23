//! What the surfaces above the window (the menu bar, the palette, the shortcut sheet, Settings, the Studio)
//! and the window's own callers may ask of the project feature about a window: its projects, the project
//! in front and its actions. Each takes the window's workspace and finds the feature's entity in it, so
//! none holds the window's lease while it works on the entity.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::project::Feature;

use super::{LoadedProject, Project, hub, project_folders_on};
use crate::window::Workspace;

/// Projects of the window's terminals and of its folders (whose colour they hold), in terminal order. None
/// while the feature is off, so nothing that scans them (the menu bar's project actions, pruning the hub)
/// keeps a stale project alive.
pub(crate) fn roots(ws: &Workspace, cx: &App) -> Vec<PathBuf> {
    if !project_folders_on(cx) {
        return Vec::new();
    }
    let Some(project) = ws.feature::<Project>() else { return Vec::new() };
    let project = project.read(cx);
    let terminals = ws.sessions.iter().filter_map(|session| project.of.get(&session.id)?.root.clone());
    terminals.chain(project.folder_projects.values().cloned()).collect()
}

/// The project folder of the terminal in front, as of the window's last look (what Settings marks "set by
/// this project").
pub(crate) fn active_root(ws: &Workspace, cx: &App) -> Option<PathBuf> {
    ws.feature::<Project>()?.read(cx).of.get(&ws.active_session().id)?.root.clone()
}

/// The terminal of the window that is in the project at `root`.
pub(crate) fn session_for_project(ws: &Workspace, root: &Path, cx: &App) -> Option<usize> {
    let project = ws.feature::<Project>()?;
    let project = project.read(cx);
    ws.sessions
        .iter()
        .position(|session| project.of.get(&session.id).is_some_and(|of| of.root.as_deref() == Some(root)))
}

/// The project of the terminal in front, if it is trusted and has Actions on: what the menus, the palette
/// and the shortcut sheet list the actions of.
pub(crate) fn actionable<'a>(ws: &Workspace, cx: &'a App) -> Option<&'a LoadedProject> {
    let project = ws.feature::<Project>()?;
    let project = project.read(cx).project(ws.active_session().id, cx)?;
    hub(cx).enabled(project, Feature::Actions).then_some(project)
}

/// Runs action `id` of the project of the terminal in front.
pub(crate) fn run_action(ws: &Workspace, id: &str, cx: &mut App) {
    let Some(project) = ws.feature::<Project>() else { return };
    let session = ws.active_session().id;
    project.update(cx, |project, cx| project.run_action(session, id, cx));
}

/// The id of the action of the terminal in front's project that `keystroke` is the key of.
pub(crate) fn action_for_key(ws: &Workspace, keystroke: &Keystroke, cx: &App) -> Option<String> {
    ws.feature::<Project>()?.read(cx).action_for_key(ws.active_session().id, keystroke, cx)
}

/// Whether a worktree may be made of `repo`: a project folder there must be trusted with its
/// settings on, since the new terminal takes the project's env and shell; no project, no condition.
pub(crate) fn worktree_allowed(repo: &Path, cx: &mut App) -> Result<(), String> {
    // With the project-folder feature off, `.wormhole/` isn't read at all, so a worktree carries
    // no project settings and there's nothing to trust.
    if !project_folders_on(cx) {
        return Ok(());
    }
    let Some(folder) = wormhole_core::project::find_project(repo) else { return Ok(()) };
    let Some(project) = super::load_project_info(&folder.root, cx) else {
        return Err("The project's .wormhole folder can't be read".into());
    };
    if hub(cx).enabled(&project, Feature::Settings) {
        Ok(())
    } else {
        Err("Trust the project first: a terminal in its worktree would run with the project's settings"
            .into())
    }
}

/// What a new worktree of `repo` runs first: its trusted project's `worktree.setup`, under the
/// same conditions [`worktree_allowed`] checks (a project that isn't trusted with its settings on runs
/// nothing). It is the repository's project that is asked, not the copy of `.wormhole/` the new
/// worktree checks out: that one has not been trusted in its new folder.
pub(crate) fn worktree_setup(repo: &Path, cx: &mut App) -> Option<String> {
    if !project_folders_on(cx) {
        return None;
    }
    let folder = wormhole_core::project::find_project(repo)?;
    let project = super::load_project_info(&folder.root, cx)?;
    if !hub(cx).enabled(&project, Feature::Settings) {
        return None;
    }
    project.snapshot.config()?.worktree_setup.clone()
}
