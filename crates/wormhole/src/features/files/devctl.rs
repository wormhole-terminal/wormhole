//! What devctl's Files commands do (`files`, `files-root`, `files-follow`, `files-expand`, `files-menu`,
//! `files-pin`): the selected terminal's tree, through the window's `Files` entity like the keys do.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::view::files::{Row, RowKind};

use super::Files;
use crate::window::Workspace;

/// Runs `f` on the window's Files for the selected terminal.
fn run<R>(
    ws: &Workspace,
    cx: &mut Context<Workspace>,
    f: impl FnOnce(&mut Files, i64, &mut Context<Files>) -> R,
) -> Result<R> {
    let files = ws.feature::<Files>().ok_or_else(|| anyhow!("the window has no Files"))?;
    let session = ws.active_session().id;
    Ok(files.update(cx, |files, fcx| f(files, session, fcx)))
}

/// `files-root <path>`: the tree is of that folder, as if it was chosen in the path menu.
pub(crate) fn root(ws: &mut Workspace, folder: &Path, cx: &mut Context<Workspace>) -> Result<()> {
    ws.show_tab(TabId::Files, cx);
    run(ws, cx, |files, session, cx| files.choose(session, folder.to_path_buf(), cx))
}

/// `files-follow`: Follow Terminal.
pub(crate) fn follow(ws: &mut Workspace, cx: &mut Context<Workspace>) -> Result<()> {
    run(ws, cx, |files, session, cx| files.follow_terminal(session, cx))
}

/// `files-expand <relative path>`: opens that folder and every folder above it.
pub(crate) fn expand(ws: &mut Workspace, relative: &Path, cx: &mut Context<Workspace>) -> Result<()> {
    ws.show_tab(TabId::Files, cx);
    run(ws, cx, |files, session, cx| {
        let of = files.of_mut(session);
        of.tree.open_path(relative);
        of.stale = true;
        files.reread(session, cx);
        cx.notify();
    })
}

/// `files-menu [model]`: the path menu, drawn in the window so `shot` shows it (a native menu can't be
/// screenshotted); `model` only records it for `menu` and `menu-pick`.
pub(crate) fn menu(
    ws: &mut Workspace,
    drawn: bool,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Result<()> {
    ws.show_tab(TabId::Files, cx);
    let files = ws.feature::<Files>().ok_or_else(|| anyhow!("the window has no Files"))?;
    let session = ws.active_session().id;
    let menu = files.update(cx, |files, fcx| {
        let at = files.of.get(&session).map(|of| of.head_at.below());
        files.path_menu_for(session, fcx).zip(at)
    });
    let (menu, at) = menu.ok_or_else(|| anyhow!("the tree has no folder yet"))?;
    crate::platform::show_menu(menu.clone(), at, window, cx);
    if drawn {
        crate::style::menu_layer::show(menu, at, window, cx);
    }
    Ok(())
}

/// `files-pin <path>`: pins the folder, or unpins it if it was pinned. Says which.
pub(crate) fn pin(ws: &Workspace, folder: &Path, cx: &mut Context<Workspace>) -> Result<String> {
    run(ws, cx, |files, _, cx| {
        let was = Files::pinned(cx).iter().any(|pinned| pinned == folder);
        files.set_pinned(folder, !was, cx);
        if was { format!("unpinned {}", folder.display()) } else { format!("pinned {}", folder.display()) }
    })
}

/// `files`: the terminal's tree as text.
pub(crate) fn describe(ws: &Workspace, cx: &mut Context<Workspace>) -> Result<String> {
    let files = ws.feature::<Files>().ok_or_else(|| anyhow!("the window has no Files"))?;
    let session = ws.active_session().id;
    let pinned = Files::pinned(cx);
    let files = files.read(cx);
    let Some(of) = files.of.get(&session) else { return Ok("files: not looked at yet\n".into()) };
    let Some(root) = &of.root else { return Ok("files: no folder\n".into()) };
    let mode = if of.chosen.is_some() { "chosen" } else { "following" };
    let mut out = format!("files: {} ({mode}) pinned={}\n", root.display(), join(&pinned));
    for row in of.tree.rows() {
        let selected = of.selected.as_deref() == Some(row.path.as_path()) && row.is_entry();
        let _ = writeln!(out, "{}", line(&row, selected));
    }
    Ok(out)
}

fn join(paths: &[PathBuf]) -> String {
    let all: Vec<String> = paths.iter().map(|path| path.display().to_string()).collect();
    if all.is_empty() { "-".into() } else { all.join(",") }
}

/// One row: indented by depth, `v` an open folder, `>` a closed one, `~` a link, `*` the selected row.
fn line(row: &Row, selected: bool) -> String {
    let indent = "  ".repeat(row.depth + 1);
    let mark = if selected { '*' } else { ' ' };
    let body = match &row.kind {
        RowKind::Folder { open: true } => format!("v {}", row.label),
        RowKind::Folder { open: false } => format!("> {}", row.label),
        RowKind::File => format!("  {}", row.label),
        RowKind::Link { .. } => format!("~ {}", row.label),
        RowKind::More(count) => format!("+ {count} more"),
        RowKind::Reading => "… reading".to_string(),
        RowKind::Empty => "(empty)".to_string(),
        RowKind::Unreadable(failure) => format!("(can't read: {failure:?})"),
    };
    format!("{mark}{indent}{body}")
}
