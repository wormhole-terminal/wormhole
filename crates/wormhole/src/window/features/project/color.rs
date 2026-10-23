//! A project's colour, changed from a folder's band menu or the Project tab: the one edit the app
//! makes to a project's own `config.json`. Core's `ProjectFolder::set_project_color` splices the one
//! key into the file (comments and layout stay, a locked or unreadable file is refused); this is
//! the off-the-UI-thread call around it and what every window does next.

use std::path::Path;

use gpui_kit::*;
use wormhole_core::project::{ProjectColorChoice, ProjectFolder};

use super::{hub_mut, project_folders_on, reload_projects};

/// What the hub's error says when writing the colour failed, so a later success can take it away
/// without touching the other problems it may be showing.
const FAILED: &str = "Couldn't change the project's colour";

/// Write `color` as the `project_color` of the project at `root` (`None` takes the key out), then
/// have every window read the project again, so the band, the title row and the Project tab turn to
/// it at once. The file is read, spliced and replaced off the UI thread.
pub(in crate::window) fn set_project_color(root: &Path, color: Option<ProjectColorChoice>, cx: &mut App) {
    // The one write into a project's `config.json`: never while the project-folder feature is off.
    if !project_folders_on(cx) {
        return;
    }
    let root = root.to_path_buf();
    cx.spawn(async move |cx| {
        let target = root.clone();
        let made = cx
            .background_spawn(async move {
                match ProjectFolder::open(&target) {
                    Some(folder) => folder.set_project_color_choice(color).map_err(|err| err.to_string()),
                    None => Err(format!("{}: has no .wormhole folder any more", target.display())),
                }
            })
            .await;
        cx.update(|cx| project_color_set(&root, made, cx));
    })
    .detach();
}

/// What came of writing the colour into the project at `root`.
fn project_color_set(root: &Path, made: Result<bool, String>, cx: &mut App) {
    let hub = hub_mut(cx);
    match made {
        Ok(changed) => {
            tracing::info!(root = %root.display(), changed, "project colour set");
            if hub.error.as_ref().is_some_and(|error| error.starts_with(FAILED)) {
                hub.error = None;
            }
        }
        Err(err) => {
            tracing::warn!("{FAILED}: {err}");
            hub.error = Some(format!("{FAILED}: {err}").into());
        }
    }
    reload_projects(cx);
}
