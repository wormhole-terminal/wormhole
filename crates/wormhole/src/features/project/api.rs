//! What the surfaces above the window (the menu bar, the palette, the shortcut sheet, Settings, the Studio)
//! and the window's own callers may ask of the project feature about a window: its projects, the project
//! in front and its actions. Each takes the window's workspace and finds the feature's entity in it, so
//! none holds the window's lease while it works on the entity.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::project::Feature;
use wormhole_core::storage::paths::ProfileId;

use super::{LoadedProject, Project, hub_of, project_folders_on};
use crate::onboarding::TrustCard;
use crate::window::Workspace;

/// Projects of the window's terminals and of its folders (whose colour they hold), in terminal order. None
/// while the feature is off, so nothing that scans them (the menu bar's project actions, pruning the hub)
/// keeps a stale project alive.
pub(crate) fn roots(ws: &Workspace, cx: &App) -> Vec<PathBuf> {
    let facts = ws.facts(cx);
    if !project_folders_on(facts.profile, cx) {
        return Vec::new();
    }
    let Some(project) = ws.feature::<Project>() else { return Vec::new() };
    let project = project.read(cx);
    let terminals = facts.sessions().filter_map(|session| project.of.get(&session.id)?.root.clone());
    terminals.chain(project.folder_projects.values().cloned()).collect()
}

/// The project folder of the terminal in front, as of the window's last look (what Settings marks "set by
/// this project").
pub(crate) fn active_root(ws: &Workspace, cx: &App) -> Option<PathBuf> {
    ws.feature::<Project>()?.read(cx).of.get(&ws.facts(cx).active()?.id)?.root.clone()
}

/// The terminal of the window that is in the project at `root`.
pub(crate) fn session_for_project(ws: &Workspace, root: &Path, cx: &App) -> Option<usize> {
    let project = ws.feature::<Project>()?;
    let project = project.read(cx);
    ws.facts(cx)
        .sessions()
        .position(|session| project.of.get(&session.id).is_some_and(|of| of.root.as_deref() == Some(root)))
}

/// The project of the terminal in front, if it is trusted and has Actions on: what the menus, the palette
/// and the shortcut sheet list the actions of.
pub(crate) fn actionable<'a>(ws: &Workspace, cx: &'a App) -> Option<&'a LoadedProject> {
    let project = ws.feature::<Project>()?;
    let project = project.read(cx).project(ws.facts(cx).active()?.id, ws.profile(), cx)?;
    hub_of(ws.profile(), cx).enabled(project, Feature::Actions).then_some(project)
}

/// Runs action `id` of the project of the terminal in front.
pub(crate) fn run_action(ws: &Workspace, id: &str, cx: &mut App) {
    let Some(project) = ws.feature::<Project>() else { return };
    let Some(session) = ws.facts(cx).active().map(|active| active.id) else { return };
    let profile = ws.profile().clone();
    project.update(cx, |project, cx| project.run_action(session, &profile, id, cx));
}

/// The id of the action of the terminal in front's project that `keystroke` is the key of.
pub(crate) fn action_for_key(ws: &Workspace, keystroke: &Keystroke, cx: &App) -> Option<String> {
    ws.feature::<Project>()?.read(cx).action_for_key(ws.facts(cx).active()?.id, ws.profile(), keystroke, cx)
}

/// The first run's trust card for the project at `root`, as `profile` (the window's) sees it: `None` when
/// the project can't be read or is trusted already.
pub(crate) fn trust_card(profile: &ProfileId, root: &Path, cx: &App) -> Option<TrustCard> {
    let project = super::load_project_info(profile, root, cx)?;
    if project.status == wormhole_core::project::TrustStatus::Trusted {
        return None;
    }
    Some(TrustCard {
        root: root.to_path_buf(),
        name: root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        dir: wormhole_core::project::dir_name_at(root),
        commands: project.request.as_ref().map_or(0, |request| request.commands.len()),
        approving: hub_of(profile, cx).approving,
    })
}

/// Ask for Touch ID, then trust the project at `root` for `profile` (the first run's button).
pub(crate) fn approve(profile: &ProfileId, root: &Path, cx: &mut App) {
    super::approve_project(profile, root, cx);
}

/// Whether a worktree may be made of `repo` for a window of `profile`: a project folder there must be
/// trusted with its settings on in that profile, since the new terminal takes the project's env and shell;
/// no project, no condition.
pub(crate) fn worktree_allowed(profile: &ProfileId, repo: &Path, cx: &mut App) -> Result<(), String> {
    // With the project-folder feature off, `.wormhole/` isn't read at all, so a worktree carries
    // no project settings and there's nothing to trust.
    if !project_folders_on(profile, cx) {
        return Ok(());
    }
    let Some(folder) = wormhole_core::project::find_project(repo) else { return Ok(()) };
    let Some(project) = super::load_project_info(profile, &folder.root, cx) else {
        return Err("The project's .wormhole folder can't be read".into());
    };
    if hub_of(profile, cx).enabled(&project, Feature::Settings) {
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
pub(crate) fn worktree_setup(profile: &ProfileId, repo: &Path, cx: &mut App) -> Option<String> {
    if !project_folders_on(profile, cx) {
        return None;
    }
    let folder = wormhole_core::project::find_project(repo)?;
    let project = super::load_project_info(profile, &folder.root, cx)?;
    if !hub_of(profile, cx).enabled(&project, Feature::Settings) {
        return None;
    }
    project.snapshot.config()?.worktree_setup.clone()
}
