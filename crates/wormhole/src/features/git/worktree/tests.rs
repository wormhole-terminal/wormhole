//! The worktree flow through the window, pinned from outside, in repositories made in the test's
//! scratch dir (the worktree base is a folder there too, so nothing is ever made next to a repository
//! of the person's): the menu item only opens a dialog; a worktree is made when the person confirms
//! it and never in a project that is not trusted; the command line only gets the dialog opened; and a
//! terminal in a worktree Wormhole made asks about the worktree when it closes.

use std::path::PathBuf;

use gpui_kit::{Context, Entity, TestAppContext, Window};
use wormhole_core::git::worktree::Worktree;
use wormhole_core::ipc::control::{Asked, Error, ErrorCode};
use wormhole_core::project::ProjectFolder;
use wormhole_core::storage::paths::ProfileId;

use super::super::testing::{folder, go, read_repo, repo, write};
use super::Worktrees;
use crate::app_state::{OpenWindow, app};
use crate::features::project::{self, hub_mut};
use crate::window::Workspace;
use crate::window::feature::CloseOf;
use crate::window::view_tests::{drive, isolate_in, read, read_cx, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// Whether the dialog is up.
fn asking(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read_cx(cx, window, |ws, cx| worktrees(ws).read(cx).asking.is_some())
}

/// The window's worktree feature.
fn worktrees(ws: &Workspace) -> Entity<Worktrees> {
    ws.feature::<Worktrees>().expect("the window has the worktree feature")
}

/// The menu item, or its key: the dialog for the terminal in front.
fn menu_item(ws: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let folder = ws.facts(cx).active().and_then(|active| active.band_folder(cx));
    worktrees(ws).update(cx, |worktrees, cx| {
        worktrees.new_terminal_in_worktree(&ProfileId::default_profile(), folder, window, cx)
    });
}

/// The command line's request, as the control socket's handler makes it.
fn command_line(
    ws: &mut Workspace,
    folder: &std::path::Path,
    branch: &str,
    command: Option<String>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Result<Asked, Error> {
    let folder = Some(folder.to_string_lossy().into_owned());
    super::api::control_ask(ws, folder, branch, command, window, cx)
}

fn sessions(cx: &mut TestAppContext, window: &OpenWindow) -> usize {
    read(cx, window, |ws| ws.sessions.len())
}

/// The worktree Wormhole made that terminal `ix` is in.
fn made(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) -> Option<Worktree> {
    read_cx(cx, window, |ws, cx| {
        let cwd = ws.sessions[ix].terminal().read(cx).cwd.clone()?;
        app(cx).worktrees.find(&cwd).cloned()
    })
}

/// The person types `branch` in the dialog and presses Create; whether the dialog closed.
fn create(cx: &mut TestAppContext, window: &OpenWindow, branch: &str) -> bool {
    let closed = drive(cx, window, |ws, window, cx| {
        worktrees(ws).update(cx, |worktrees, cx| {
            let input = worktrees.asking.as_ref().expect("the dialog is up").input.clone();
            input.update(cx, |input, cx| input.set_value(branch.to_string(), window, cx));
            worktrees.confirm_worktree(window, cx)
        })
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

/// What closing the second terminal's group adds to the question: its words, and whether it offers to
/// remove the worktree too.
fn group_question(cx: &mut TestAppContext, window: &OpenWindow) -> (String, bool) {
    read_cx(cx, window, |ws, cx| {
        let extras = ws.close_extras(&[ws.sessions[1].id], CloseOf::Group, cx);
        let extra = extras.into_iter().next().expect("the terminal is in a worktree Wormhole made");
        (extra.note, extra.also.is_some())
    })
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
    drive(cx, &window, menu_item);
    assert!(asking(cx, &window), "the dialog is up");
    assert!(
        std::fs::read_dir(&base).expect("the base").next().is_none() && sessions(cx, &window) == 1,
        "and nothing was made"
    );
    // A name git won't take keeps the dialog up, with the reason.
    assert!(!create(cx, &window, "no good"));
    assert!(read_cx(cx, &window, |ws, cx| worktrees(ws)
        .read(cx)
        .asking
        .as_ref()
        .is_some_and(|ask| ask.error.is_some())));
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

    // A terminal started in it gets its ports in its environment; one started in the repository does
    // not.
    let (in_worktree, in_repo) = cx.update(|cx| {
        let table = crate::features::TABLE;
        (
            Workspace::spawn_adds(&ProfileId::default_profile(), Some(worktree.folder.as_path()), table, cx),
            Workspace::spawn_adds(&ProfileId::default_profile(), Some(root.as_path()), table, cx),
        )
    });
    let ports: std::collections::BTreeMap<String, String> =
        worktree.env().into_iter().map(|(name, value)| (name.to_string(), value)).collect();
    assert!(!ports.is_empty());
    assert_eq!(in_worktree.env, ports);
    assert!(in_repo.env.is_empty());
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
    drive(cx, &window, menu_item);
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
    drive(cx, &window, menu_item);
    assert!(!asking(cx, &window), "the project's settings would run in the new terminal");
    let asked = drive(cx, &window, |ws, window, cx| command_line(ws, &root, "x", None, window, cx));
    assert_eq!(asked.expect_err("refused").code, ErrorCode::Refused);
    assert!(!asking(cx, &window));
    assert_eq!(
        cx.update(|cx| project::api::worktree_setup(&ProfileId::default_profile(), &root, cx)),
        None,
        "and no setup runs"
    );

    // Trusted, the dialog opens for either, and the command line makes nothing by itself.
    trust(cx, &root);
    let asked = drive(cx, &window, |ws, window, cx| {
        command_line(ws, &root, "feature/x", Some("echo hi".into()), window, cx)
    });
    let asked = asked.expect("the dialog is opened");
    assert!(asked.asked && asked.branch == "feature/x");
    assert!(asking(cx, &window) && sessions(cx, &window) == 1);
    assert!(!std::path::Path::new(&asked.folder).exists(), "only the dialog was opened");
    assert_eq!(
        cx.update(|cx| project::api::worktree_setup(&ProfileId::default_profile(), &root, cx)).as_deref(),
        Some("make setup"),
        "a trusted project's setup runs in the new terminal"
    );
    // A second ask while one is up is refused.
    let again = drive(cx, &window, |ws, window, cx| command_line(ws, &root, "other", None, window, cx));
    assert_eq!(again.expect_err("refused").code, ErrorCode::Failed);
}

/// A dialog that is up when its window moves to another profile follows that profile's approvals: Create
/// reads the project's setup as the window's profile is now, not as it was when the dialog opened.
#[gpui_kit::test]
fn a_dialog_follows_the_profile_its_window_moves_to(cx: &mut TestAppContext) {
    if isolate("a_dialog_follows_the_profile_its_window_moves_to") {
        return;
    }
    use wormhole_core::config::profile::{NewProfile, StartFrom};
    let window = start(cx, 1);
    base(cx, "worktrees-moves");
    let root = in_repo(cx, &window, "moves");
    with_project(&root);
    trust(cx, &root);
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            let id = crate::app_state::AppState::create_profile(new, cx)?;
            crate::app_state::AppState::ensure_open(&id, cx);
            Ok::<ProfileId, String>(id)
        })
        .expect("makes the profile");
    drive(cx, &window, menu_item);
    let asked_as =
        |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| worktrees(ws).read(cx).asked_as().cloned());
    assert_eq!(asked_as(cx), Some(ProfileId::default_profile()));

    drive(cx, &window, |ws, _, cx| ws.rehome(&work, cx).expect("the window moves"));
    assert_eq!(asked_as(cx), Some(work.clone()), "the dialog is Work's now");
    assert_eq!(
        cx.update(|cx| project::api::worktree_setup(&work, &root, cx)),
        None,
        "Work does not trust the project, so no setup runs for it"
    );
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
        let folder = folder.to_path_buf();
        drive(cx, &window, move |ws, window, cx| command_line(ws, &folder, branch, None, window, cx))
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
    drive(cx, &window, menu_item);
    assert!(create(cx, &window, "topic"));
    let worktree = made(cx, &window, 1).expect("made");

    // Clean: the group question offers to remove it too.
    drive(cx, &window, |ws, _, cx| {
        let ix = ws.active;
        ws.refresh_git(ix, cx);
    });
    cx.run_until_parked();
    let (note, removable) = group_question(cx, &window);
    assert!(removable, "{note}");
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
    let (note, removable) = group_question(cx, &window);
    assert!(!removable && note.contains("stay"), "{note}");
}

/// Create refused by git (the branch is checked out in the repository): the dialog closes, the reason
/// shows in a sheet, and once that is read the keys are in the terminal again, not nowhere.
#[gpui_kit::test]
fn a_worktree_that_git_refuses_leaves_the_keys_in_the_terminal(cx: &mut TestAppContext) {
    use gpui_kit::component::dialog::Confirm;
    use std::time::Duration;

    use crate::window::view_tests::with_window;
    if isolate("a_worktree_that_git_refuses_leaves_the_keys_in_the_terminal") {
        return;
    }
    let window = start(cx, 1);
    base(cx, "worktrees-refused");
    in_repo(cx, &window, "refused");
    drive(cx, &window, menu_item);
    drive(cx, &window, |ws, window, cx| {
        worktrees(ws).update(cx, |worktrees, cx| {
            let input = worktrees.asking.as_ref().expect("the dialog is up").input.clone();
            input.update(cx, |input, cx| input.set_value("main".to_string(), window, cx));
        });
    });
    // Create, as the key does: the dialog closes and git is asked off the UI thread.
    let press = |cx: &mut TestAppContext| {
        with_window(cx, &window, |window, cx| {
            window.dispatch_action(Box::new(Confirm { secondary: false }), cx)
        });
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
    };
    press(cx);
    assert!(!asking(cx, &window), "the dialog closed");
    assert_eq!(sessions(cx, &window), 1, "and nothing was made");
    // The sheet that says why; reading it is OK.
    press(cx);
    let terminal = read(cx, &window, |ws| ws.active_terminal());
    let focused = with_window(cx, &window, |window, cx| terminal.read(cx).focus_handle.is_focused(window));
    assert!(focused, "the terminal has the keys");
}
