//! Helpers for the git features' window tests: real repositories in the test's scratch dir (the
//! harness gives every test a data dir and a `HOME` of its own, so no git config and no repository of
//! the person's is ever read or written), and the few places a test names where a feature keeps its
//! state, so that the tests say what happens and not how it is held.

use std::path::{Path, PathBuf};
use std::process::Command;

use gpui_kit::{App, Context, Entity, TestAppContext};
use wormhole_core::git::FileChange;

use super::changes::Changes;
use crate::app_state::OpenWindow;
use crate::window::Workspace;
use crate::window::focus::Walkable;
use crate::window::view_tests::drive;

/// Runs git in `dir`, as a test's fixture (the app itself never writes to a repository this way).
pub(super) fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("runs git");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A folder in the test's scratch dir.
pub(super) fn folder(name: &str) -> PathBuf {
    let dir = std::env::current_dir().expect("the scratch dir").join(name);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir.canonicalize().expect("canonical")
}

/// A repository on `main` with these files committed.
pub(super) fn repo(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = folder(name);
    git(&root, &["init", "-q", "-b", "main"]);
    write(&root, files);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "first"]);
    root
}

/// Writes these files (making their folders) under `root`.
pub(super) fn write(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }
}

/// The terminal in front is in `dir`.
pub(super) fn go(ws: &mut Workspace, dir: &Path, cx: &mut Context<Workspace>) {
    let dir = dir.to_path_buf();
    ws.active_terminal().update(cx, |terminal, _| terminal.cwd = Some(dir));
}

/// The window asks git about the terminal in front, and the answer is in.
pub(super) fn read_repo(cx: &mut TestAppContext, window: &OpenWindow) {
    drive(cx, window, |ws, _, cx| {
        let ix = ws.active;
        ws.refresh_git(ix, cx);
    });
    cx.run_until_parked();
}

/// The paths git lists as changed for the terminal in front, sorted; `None` before it has answered
/// for a repository.
pub(super) fn changed_paths(ws: &Workspace) -> Option<Vec<String>> {
    let mut paths: Vec<String> =
        ws.active_session().git.status.as_ref()?.files.iter().map(|f| f.path.clone()).collect();
    paths.sort();
    Some(paths)
}

/// The changed file called `path`, as git described it.
pub(super) fn change_of(ws: &Workspace, path: &str) -> FileChange {
    let status = ws.active_session().git.status.as_ref().expect("git has answered");
    status.files.iter().find(|f| f.path == path).cloned().expect("a changed file")
}

/// The window's Changes.
pub(super) fn changes(ws: &Workspace) -> Entity<Changes> {
    ws.feature::<Changes>().expect("the window has the Changes feature")
}

/// The path of the diff on view.
pub(super) fn diff_path(ws: &Workspace, cx: &App) -> Option<String> {
    changes(ws).read(cx).diff.as_ref().map(|view| view.file().path.clone())
}

/// A click on the changed file `path` of the terminal in front in the list: its diff opens.
pub(super) fn open_change(ws: &mut Workspace, path: &str, cx: &mut Context<Workspace>) {
    let file = change_of(ws, path);
    let session = ws.active_session().id;
    let target = changes(ws).read(cx).target(session).expect("the list has been drawn");
    changes(ws).update(cx, |changes, cx| changes.open_change(session, file, &target, cx));
}

/// The Changes list's rows the keys can be on, as the window asks the table.
pub(super) fn walkable(ws: &Workspace, cx: &App) -> Vec<Walkable> {
    let (at, tab) = ws.tab_entry(wormhole_core::config::TabId::Changes).expect("in the table");
    let entity = ws.feature_entity(at).expect("made with the window");
    (tab.walk.as_ref().expect("has a list").walkable)(entity, &ws.facts(cx), cx)
}

/// Whether the New Terminal in New Worktree dialog is up.
pub(super) fn worktree_dialog_up(ws: &Workspace, cx: &App) -> bool {
    ws.feature::<super::worktree::Worktrees>().is_some_and(|worktrees| worktrees.read(cx).is_asking())
}

/// The menu item, or its key: the dialog for the terminal in front.
pub(super) fn ask_for_worktree(
    ws: &mut Workspace,
    window: &mut gpui_kit::Window,
    cx: &mut Context<Workspace>,
) {
    let folder = ws.facts(cx).active().and_then(|active| active.band_folder(cx));
    let worktrees = ws.feature::<super::worktree::Worktrees>().expect("in the table");
    let profile = ws.profile().clone();
    worktrees.update(cx, |worktrees, cx| worktrees.new_terminal_in_worktree(&profile, folder, window, cx));
}
