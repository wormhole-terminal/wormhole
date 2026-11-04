//! What the surfaces above the window (the menu bar, the palette, the shortcut sheet, Settings, the Studio)
//! and the window's own callers may ask of the project feature about a window: its projects, the project
//! in front and its actions. Each takes the window's workspace and finds the feature's entity in it, so
//! none holds the window's lease while it works on the entity.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::project::{Feature, FeatureStanding};
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

// What Settings' Projects page and the status item read of the shown profile's hub, and the Studio of a
// window's, said as questions so the surfaces above the feature don't hold the hub.

/// Whether `feature` is on by default for all projects in the shown profile.
pub(crate) fn default_enabled(feature: Feature, cx: &App) -> bool {
    super::hub(cx).prefs.default_enabled(feature)
}

/// The projects the shown profile trusts, whether or not they are open anywhere.
pub(crate) fn trusted_roots(cx: &App) -> Vec<PathBuf> {
    super::hub(cx).trust.list().into_iter().map(|trusted| trusted.root.clone()).collect()
}

/// An approval is being asked for in the shown profile, and the last thing that went wrong with a project
/// in it, for Settings' Projects page.
pub(crate) fn approval(cx: &App) -> (bool, Option<SharedString>) {
    let hub = super::hub(cx);
    (hub.approving, hub.error.clone())
}

/// One row of the Projects page per feature, for the project at `root` as the shown profile has it: the
/// feature, where it stands, the project's own switch for it, and the default it falls back to.
pub(crate) fn feature_rows(
    root: &Path,
    project: &LoadedProject,
    cx: &App,
) -> Vec<(Feature, FeatureStanding, Option<bool>, bool)> {
    let prefs = &super::hub(cx).prefs;
    Feature::ALL
        .into_iter()
        .map(|f| {
            // Notes: with no notes.md nothing syncs unless the person switches it on, so
            // "Default (on)" would say what doesn't happen.
            let default = match f {
                Feature::Notes => prefs.default_enabled(f) && project.folder.has(f),
                _ => prefs.default_enabled(f),
            };
            (f, project.snapshot.standing(f, project.status), prefs.project_override(root, f), default)
        })
        .collect()
}

/// The shown profile's project at `root`, if it has loaded, is trusted with its Actions on: what the status
/// item lists the actions of.
pub(crate) fn shown_actionable<'a>(root: &Path, cx: &'a App) -> Option<&'a LoadedProject> {
    let hub = super::hub(cx);
    let project = hub.projects.get(root)?;
    hub.enabled(project, Feature::Actions).then_some(project)
}

/// The project folder "Use in this project" writes a theme to: the one at `root`, as the profile of the
/// window in front has it, when it is trusted and its theme isn't switched off. Otherwise what stands in
/// the way, in the person's words.
pub(crate) fn theme_target(
    profile: &ProfileId,
    root: &Path,
    cx: &App,
) -> Result<wormhole_core::project::ProjectFolder, &'static str> {
    let hub = hub_of(profile, cx);
    let project = hub.projects.get(root).ok_or("The project in front hasn't loaded yet.")?;
    if hub.trust.status_of(&project.snapshot) != wormhole_core::project::TrustStatus::Trusted {
        return Err("Trust this project first (Project tab), then use the theme in it.");
    }
    let switched_on = if project.snapshot.has(Feature::Theme) {
        hub.enabled(project, Feature::Theme)
    } else {
        hub.prefs.enabled(root, Feature::Theme)
    };
    if !switched_on {
        return Err("This project's theme is switched off (Project tab).");
    }
    Ok(project.folder.clone())
}
