//! "Create project folder" in the Project tab: `.wormhole/` in the active terminal's folder (its
//! repository's root, else the folder itself, as its band names it), made off the UI thread by
//! core's `project::init`, which refuses to touch anything that is already there.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::project::{self as proj, ProjectError, ProjectFolder};
use wormhole_core::storage::paths::ProfileId;

use super::{hub_of_mut, project_folders_on, reload_projects};

/// Make the project folder in `folder` (the terminal's folder, as its band names it), then have every
/// window look for it, so the tab turns into the project's at once. A new project is untrusted and sets
/// nothing, which the tab already shows.
pub(crate) fn create_project_folder(profile: &ProfileId, folder: PathBuf, cx: &mut App) {
    if !project_folders_on(profile, cx) {
        return;
    }
    let profile = profile.clone();
    cx.spawn(async move |cx| {
        let target = folder.clone();
        let made = cx.background_spawn(async move { proj::init(&target) }).await;
        cx.update(|cx| project_made(&profile, &folder, made, cx));
    })
    .detach();
}

/// What came of making the project folder in `folder`.
fn project_made(profile: &ProfileId, folder: &Path, made: Result<PathBuf, ProjectError>, cx: &mut App) {
    match made {
        Ok(dir) => {
            tracing::info!(dir = %dir.display(), "project folder created");
            if let Some(hub) = hub_of_mut(profile, cx) {
                hub.error = None;
            }
        }
        // A second click, or another program, got there first: it is a project all the same.
        Err(err) if ProjectFolder::open(folder).is_some() => {
            tracing::info!("project folder already there: {err}");
        }
        Err(err) => {
            tracing::warn!("creating the project folder: {err}");
            if let Some(hub) = hub_of_mut(profile, cx) {
                hub.error = Some(format!("Couldn't create the project folder: {err}").into());
            }
        }
    }
    reload_projects(cx);
}
