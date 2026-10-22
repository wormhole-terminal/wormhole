//! "Create project folder" in the Project tab: `.wormhole/` in the active terminal's folder (its
//! repository's root, else the folder itself, as its band names it), made off the UI thread by
//! core's `project::init`, which refuses to touch anything that is already there.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::project::{self as proj, ProjectError, ProjectFolder};

use super::{project_folders_on, reload_projects};
use crate::app_state::app_mut;
use crate::workspace::Workspace;

impl Workspace {
    /// Make the project folder for the active terminal's folder, then have every window look for
    /// it, so the tab turns into the project's at once. A new project is untrusted and sets
    /// nothing, which the tab already shows.
    pub(in crate::window) fn create_project_folder(&mut self, cx: &mut Context<Self>) {
        if !project_folders_on(cx) {
            return;
        }
        let Some(folder) = self.active_folder(cx) else { return };
        cx.spawn(async move |this, cx| {
            let target = folder.clone();
            let made = cx.background_spawn(async move { proj::init(&target) }).await;
            this.update(cx, |_, cx| project_made(&folder, made, cx)).ok();
        })
        .detach();
    }
}

/// What came of making the project folder in `folder`.
fn project_made(folder: &Path, made: Result<PathBuf, ProjectError>, cx: &mut App) {
    match made {
        Ok(dir) => {
            tracing::info!(dir = %dir.display(), "project folder created");
            app_mut(cx).hub.error = None;
        }
        // A second click, or another program, got there first: it is a project all the same.
        Err(err) if ProjectFolder::open(folder).is_some() => {
            tracing::info!("project folder already there: {err}");
        }
        Err(err) => {
            tracing::warn!("creating the project folder: {err}");
            app_mut(cx).hub.error = Some(format!("Couldn't create the project folder: {err}").into());
        }
    }
    reload_projects(cx);
}
