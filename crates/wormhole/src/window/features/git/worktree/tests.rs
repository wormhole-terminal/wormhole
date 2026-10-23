//! The worktree flow through the window, pinned from outside, in repositories made in the test's
//! scratch dir (the worktree base is a folder there too, so nothing is ever made next to a repository
//! of the person's): the menu item only opens a dialog; a worktree is made when the person confirms
//! it and never in a project that is not trusted; the command line only gets the dialog opened; and a
//! terminal in a worktree Wormhole made asks about the worktree when it closes.

use std::path::PathBuf;

use gpui_kit::TestAppContext;
use wormhole_core::git::worktree::Worktree;
use wormhole_core::ipc::control::ErrorCode;
use wormhole_core::project::ProjectFolder;

use super::super::testing::{folder, go, read_repo, repo, write};
use crate::app_state::{OpenWindow, app};
use crate::window::features::project::{self, hub_mut};
use crate::window::view_tests::{drive, isolate_in, read, read_cx, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// Whether the dialog is up.
fn asking(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read(cx, window, |ws| ws.worktree_ask.is_some())
}

fn sessions(cx: &mut TestAppContext, window: &OpenWindow) -> usize {
    read(cx, window, |ws| ws.sessions.len())
}

/// The worktree Wormhole made that terminal `ix` is in.
fn made(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) -> Option<Worktree> {
    read_cx(cx, window, |ws, cx| ws.made_worktree_of(ix, cx))
}

/// The person types `branch` in the dialog and presses Create; whether the dialog closed.
fn create(cx: &mut TestAppContext, window: &OpenWindow, branch: &str) -> bool {
    let closed = drive(cx, window, |ws, window, cx| {
        let input = ws.worktree_ask.as_ref().expect("the dialog is up").input.clone();
        input.update(cx, |input, cx| input.set_value(branch.to_string(), window, cx));
        ws.confirm_worktree(window, cx)
    });
    cx.run_until_parked();
    closed
}

/// Where worktrees go for this test: a folder of its own in the scratch dir.
fn base(cx: &mut TestAppContext, name: &str) -> PathBuf {
    let base = folder(name);
    cx.update(|cx| {
        let text = base.to_string_lossy().into_owned();
        crate::settings::update_settings(cx, |settings| settings.worktree_base = text);
    });
    base
}

/// A repository in the scratch dir with the window's terminal in it.
fn in_repo(cx: &mut TestAppContext, window: &OpenWindow, name: &str) -> PathBuf {
    let root = repo(name, &[("a.txt", "one\n")]);
    drive(cx, window, |ws, _, cx| go(ws, &root, cx));
    read_repo(cx, window);
    root
}

/// Makes the repository's `.wormhole/` project folder, which is not trusted.
fn with_project(root: &std::path::Path) {
    write(root, &[(".wormhole/config.json", r#"{ "worktree": { "setup": "make setup" } }"#)]);
}

fn trust(cx: &mut TestAppContext, root: &std::path::Path) {
    cx.update(|cx| {
        let folder = ProjectFolder::open(root).expect("a project");
        hub_mut(cx).trust.trust(&folder);
    });
}

// ---------------------------------------------------------------- the flow

#[gpui_kit::test]
fn the_menu_item_asks_and_makes_nothing_until_the_person_confirms(cx: &mut TestAppContext) {
    if isolate("the_menu_item_asks_and_makes_nothing_until_the_person_confirms") {
        return;
    }
    let window = start(cx, 1);
    let base = base(cx, "worktrees");
    let root = in_repo(cx, &window, "asks");
    drive(cx, &window, |ws, window, cx| ws.new_terminal_in_worktree(window, cx));
    assert!(asking(cx, &window), "the dialog is up");
    assert!(
        std::fs::read_dir(&base).expect("the base").next().is_none() && sessions(cx, &window) == 1,
        "and nothing was made"
    );
    // A name git won't take keeps the dialog up, with the reason.
    assert!(!create(cx, &window, "no good"));
    assert!(read(cx, &window, |ws| ws.worktree_ask.as_ref().is_some_and(|ask| ask.error.is_some())));
    assert!(std::fs::read_dir(&base).expect("the base").next().is_none());

    // Create makes the worktree, opens a terminal in it and remembers it as one of Wormhole's.
    assert!(create(cx, &window, "fix/login"));
    assert!(!asking(cx, &window));
    let folder = base.join(root.file_name().expect("a name")).join("fix-login");
    let made_folder =
        read_cx(cx, &window, |ws, cx| ws.sessions.last().and_then(|s| s.terminal().read(cx).cwd.clone()));
    let worktree = made(cx, &window, 1).expect("the new terminal is in a worktree Wormhole made");
    assert_eq!(worktree.branch, "fix/login");
    assert_eq!(made_folder.as_deref(), Some(worktree.folder.as_path()));
    assert!(worktree.folder.join(".git").exists(), "{}", worktree.folder.display());
    assert!(worktree.folder.starts_with(&base) && worktree.repo == root);
    let _ = folder;
    assert_eq!(sessions(cx, &window), 2);
    assert_eq!(read(cx, &window, |ws| ws.active), 1, "selected");
    assert!(cx.update(|cx| app(cx).worktrees.find(&worktree.folder).is_some()));
    assert!(worktree.port.is_some(), "with ports of its own");
}

#[gpui_kit::test]
fn nothing_is_made_without_a_repository(cx: &mut TestAppContext) {
    if isolate("nothing_is_made_without_a_repository") {
        return;
    }
    let window = start(cx, 1);
    let plain = folder("not-a-repo");
    drive(cx, &window, |ws, _, cx| go(ws, &plain, cx));
    read_repo(cx, &window);
    drive(cx, &window, |ws, window, cx| ws.new_terminal_in_worktree(window, cx));
    assert!(!asking(cx, &window));
}

#[gpui_kit::test]
fn a_project_that_is_not_trusted_gets_no_worktree_from_the_menu_or_the_command_line(cx: &mut TestAppContext) {
    if isolate("a_project_that_is_not_trusted_gets_no_worktree_from_the_menu_or_the_command_line") {
        return;
    }
    let window = start(cx, 1);
    base(cx, "worktrees-untrusted");
    let root = in_repo(cx, &window, "untrusted");
    with_project(&root);
    drive(cx, &window, |ws, window, cx| ws.new_terminal_in_worktree(window, cx));
    assert!(!asking(cx, &window), "the project's settings would run in the new terminal");
    let asked = drive(cx, &window, |ws, window, cx| {
        ws.control_ask_worktree(Some(root.to_string_lossy().into_owned()), "x", None, window, cx)
    });
    assert_eq!(asked.expect_err("refused").code, ErrorCode::Refused);
    assert!(!asking(cx, &window));
    assert_eq!(cx.update(|cx| project::api::worktree_setup(&root, cx)), None, "and no setup runs");

    // Trusted, the dialog opens for either, and the command line makes nothing by itself.
    trust(cx, &root);
    let asked = drive(cx, &window, |ws, window, cx| {
        ws.control_ask_worktree(
            Some(root.to_string_lossy().into_owned()),
            "feature/x",
            Some("echo hi".into()),
            window,
            cx,
        )
    });
    let asked = asked.expect("the dialog is opened");
    assert!(asked.asked && asked.branch == "feature/x");
    assert!(asking(cx, &window) && sessions(cx, &window) == 1);
    assert!(!std::path::Path::new(&asked.folder).exists(), "only the dialog was opened");
    assert_eq!(
        cx.update(|cx| project::api::worktree_setup(&root, cx)).as_deref(),
        Some("make setup"),
        "a trusted project's setup runs in the new terminal"
    );
    // A second ask while one is up is refused.
    let again = drive(cx, &window, |ws, window, cx| {
        ws.control_ask_worktree(Some(root.to_string_lossy().into_owned()), "other", None, window, cx)
    });
    assert_eq!(again.expect_err("refused").code, ErrorCode::Failed);
}

#[gpui_kit::test]
fn the_command_line_names_what_is_wrong_with_the_folder_or_the_branch(cx: &mut TestAppContext) {
    if isolate("the_command_line_names_what_is_wrong_with_the_folder_or_the_branch") {
        return;
    }
    let window = start(cx, 1);
    base(cx, "worktrees-cli");
    let root = in_repo(cx, &window, "cli");
    let plain = folder("cli-plain");
    let ask = |cx: &mut TestAppContext, folder: &std::path::Path, branch: &'static str| {
        let folder = folder.to_string_lossy().into_owned();
        drive(cx, &window, move |ws, window, cx| {
            ws.control_ask_worktree(Some(folder), branch, None, window, cx)
        })
    };
    assert_eq!(ask(cx, &plain, "x").expect_err("no repository").code, ErrorCode::NotFound);
    assert_eq!(ask(cx, &root, "-bad").expect_err("a bad name").code, ErrorCode::BadRequest);
    assert!(!asking(cx, &window));
}

#[gpui_kit::test]
fn closing_a_terminal_in_a_made_worktree_asks_and_keeps_a_worktree_with_changes(cx: &mut TestAppContext) {
    if isolate("closing_a_terminal_in_a_made_worktree_asks_and_keeps_a_worktree_with_changes") {
        return;
    }
    let window = start(cx, 1);
    base(cx, "worktrees-close");
    in_repo(cx, &window, "closing");
    drive(cx, &window, |ws, window, cx| ws.new_terminal_in_worktree(window, cx));
    assert!(create(cx, &window, "topic"));
    let worktree = made(cx, &window, 1).expect("made");

    // Clean: the group question offers to remove it too.
    drive(cx, &window, |ws, _, cx| {
        let ix = ws.active;
        ws.refresh_git(ix, cx);
    });
    cx.run_until_parked();
    let (note, removable) = read_cx(cx, &window, |ws, cx| {
        let group = ws.made_worktrees_of(&[ws.sessions[1].id], cx);
        (group.note(), group.removable())
    });
    assert_eq!(removable.len(), 1, "{note}");
    assert!(note.contains("topic") && note.contains("Close and Remove Worktree"), "{note}");

    // The question comes first: the terminal is still there.
    drive(cx, &window, |ws, window, cx| ws.request_close_session(1, window, cx));
    assert_eq!(sessions(cx, &window), 2);
    assert!(read(cx, &window, |ws| ws.closing_session_prompt), "asked");

    // With changes in it the worktree stays, and the question says so.
    write(&worktree.folder, &[("new.txt", "x\n")]);
    drive(cx, &window, |ws, _, cx| {
        let ix = ws.active;
        ws.refresh_git(ix, cx);
    });
    cx.run_until_parked();
    let (note, removable) = read_cx(cx, &window, |ws, cx| {
        let group = ws.made_worktrees_of(&[ws.sessions[1].id], cx);
        (group.note(), group.removable())
    });
    assert!(removable.is_empty() && note.contains("stay"), "{note}");
}
