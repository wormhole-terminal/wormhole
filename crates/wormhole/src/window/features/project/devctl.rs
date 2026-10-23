//! The project feature's devctl commands: what its tab's buttons do, for a script to press.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::project::ProjectColorChoice;

use super::folder_color::{FolderPaint, set_folder_choice};
use super::{Project, approve_project, create_project_folder, open_trust_window};
use crate::window::Workspace;

/// `project`: shows the tab.
pub(crate) fn show_tab(ws: &mut Workspace, cx: &mut Context<Workspace>) {
    ws.show_tab(TabId::Project, cx);
}

/// `folder-color`: colour `folder` (the active terminal's when none is named), or take its colour away,
/// where its colour lives (the project's `config.json`, else the person's settings), as the band's menu
/// does. Says which folder it was.
pub(crate) fn folder_color(
    ws: &Workspace,
    folder: Option<&Path>,
    color: Option<ProjectColorChoice>,
    cx: &mut Context<Workspace>,
) -> Result<PathBuf, &'static str> {
    let folder = match folder {
        Some(folder) => folder.to_path_buf(),
        None => ws.active_folder(cx).ok_or("the active terminal has no folder yet")?,
    };
    let project = ws.feature::<Project>().ok_or("the window has no project feature")?;
    let paint: FolderPaint = project.read(cx).folder_paint(ws.profile(), &folder, cx);
    set_folder_choice(&paint, &folder, color, cx);
    Ok(folder)
}

/// `project-init`: what the Project tab's Create project folder button does; says which folder it makes
/// the `.wormhole/` in.
pub(crate) fn init(ws: &Workspace, cx: &mut Context<Workspace>) -> Result<PathBuf, &'static str> {
    let folder = ws.active_folder(cx).ok_or("the active terminal has no folder yet")?;
    create_project_folder(folder.clone(), cx);
    Ok(folder)
}

/// `project-trust`: what the Project tab's trust button does for the active terminal's project (Touch ID
/// is answered by `WORMHOLE_AUTH`); says which folder it asked for.
pub(crate) fn trust(ws: &Workspace, cx: &mut Context<Workspace>) -> Result<PathBuf, &'static str> {
    let root = super::api::active_root(ws, cx).ok_or("the active terminal has no project")?;
    approve_project(&root, cx);
    Ok(root)
}

/// `trust-window`: the window for the active terminal's project.
pub(crate) fn trust_window(ws: &Workspace, cx: &mut Context<Workspace>) -> anyhow::Result<()> {
    let root = super::api::active_root(ws, cx).ok_or_else(|| anyhow::anyhow!("no project here"))?;
    cx.defer(move |cx| open_trust_window(root, cx));
    Ok(())
}
