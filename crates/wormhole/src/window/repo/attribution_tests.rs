//! A terminal's row counts the files it changed (`attribution`): through a real repository in the
//! test's scratch dir (the harness gives every test a data dir and a `HOME` of its own), the way a
//! window reads it, end to end.

use std::path::{Path, PathBuf};
use std::process::Command;

use gpui_kit::{Context, TestAppContext};
use wormhole_core::config::{ChangesScope, SharedChanges};
use wormhole_core::storage::store::Store;

use super::changes_of;
use crate::app_state::{OpenWindow, app, app_mut};
use crate::settings::update_settings;
use crate::window::Workspace;
use crate::window::test_harness::{draw, drive, hook, isolate_in, read, read_cx, sidebar_of, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("runs git");
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

/// A repository on `main` in the scratch dir, with two files committed.
fn repo(name: &str) -> PathBuf {
    let root = std::env::current_dir().expect("the scratch dir").join(name);
    std::fs::create_dir_all(&root).expect("mkdir");
    let root = root.canonicalize().expect("canonical");
    git(&root, &["init", "-q", "-b", "main"]);
    write(&root, "a.txt");
    write(&root, "b.txt");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "first"]);
    root
}

fn write(root: &Path, name: &str) {
    std::fs::write(root.join(name), name).expect("write");
}

/// Every terminal of the window is in `dir`.
fn all_go(ws: &mut Workspace, dir: &Path, cx: &mut Context<Workspace>) {
    for session in &ws.sessions {
        session.terminal().update(cx, |terminal, cx| {
            terminal.cwd = Some(dir.to_path_buf());
            cx.notify();
        });
    }
    ws.sync_summaries(cx);
}

/// The window asks git about terminal `ix`, and the answer is in.
fn read_repo(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) {
    drive(cx, window, |ws, _, cx| ws.refresh_git(ix, cx));
}

/// The tick's part: every terminal says where it is and whether it works.
fn tick(cx: &mut TestAppContext, window: &OpenWindow) {
    drive(cx, window, |ws, _, cx| ws.track_changes(cx));
}

/// Two terminals in a repository that git has answered for, with a baseline taken; an agent works in
/// terminal 1.
fn working_agent(cx: &mut TestAppContext, name: &str) -> (OpenWindow, PathBuf) {
    let root = repo(name);
    let window = start(cx, 2);
    drive(cx, &window, |ws, _, cx| all_go(ws, &root, cx));
    read_repo(cx, &window, 0);
    read_repo(cx, &window, 1);
    tick(cx, &window);
    hook(cx, &window, 1, "UserPromptSubmit", None);
    tick(cx, &window);
    (window, root)
}

fn counts(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<Option<usize>> {
    read_cx(cx, window, |ws, cx| ws.row_infos(cx).iter().map(|row| row.changes).collect())
}

/// Two terminals in one repository: while an agent works in the first, files appear; its row counts
/// them and the other terminal's does not, in the list the sidebar was told without a draw.
#[gpui_kit::test]
fn a_row_counts_the_files_its_own_terminal_changed(cx: &mut TestAppContext) {
    if isolate("a_row_counts_the_files_its_own_terminal_changed") {
        return;
    }
    let (window, root) = working_agent(cx, "rows");
    assert_eq!(counts(cx, &window), [None, None], "what was changed before is nobody's");

    write(&root, "new.txt");
    std::fs::write(root.join("a.txt"), "edited").expect("write");
    read_repo(cx, &window, 1);
    // The same repository, so git reports the same two files for both: only the agent's has them.
    assert_eq!(counts(cx, &window), [Some(2), None]);
    let told =
        sidebar_of(cx, &window, |sidebar| sidebar.input().rows.iter().map(|r| r.changes).collect::<Vec<_>>());
    assert_eq!(told, [Some(2), None], "the sidebar was told without a draw");
    draw(cx, &window);

    // Committing them takes them off the row.
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "second"]);
    read_repo(cx, &window, 0);
    assert_eq!(counts(cx, &window), [None, None]);
}

#[gpui_kit::test]
fn a_terminal_moved_to_another_window_keeps_its_count(cx: &mut TestAppContext) {
    if isolate("a_terminal_moved_to_another_window_keeps_its_count") {
        return;
    }
    let (window, root) = working_agent(cx, "moves");
    write(&root, "new.txt");
    read_repo(cx, &window, 1);
    assert_eq!(cx.update(|cx| changes_of(1, SharedChanges::Each, cx)), Some(1));

    // A state that git has not answered for (what a window made for a moved terminal starts with)
    // is not "in no repository".
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].git = Default::default();
        ws.track_changes(cx);
    });
    assert_eq!(
        cx.update(|cx| changes_of(1, SharedChanges::Each, cx)),
        Some(1),
        "not known yet is not the same as outside"
    );

    // And across two windows, with the program running on.
    drive(cx, &window, |ws, window, cx| ws.tear_off_at(1, 600.0, 300.0, window, cx));
    let other = cx
        .update(|cx| app(cx).windows.iter().find(|w| w.id != window.id).cloned())
        .expect("a window of its own");
    assert_eq!(read(cx, &other, |ws| ws.sessions.iter().map(|s| s.id).collect::<Vec<_>>()), [1]);
    tick(cx, &other);
    assert_eq!(cx.update(|cx| changes_of(1, SharedChanges::Each, cx)), Some(1), "its count went with it");
    assert_eq!(counts(cx, &other), [Some(1)], "and the new window's row shows it");
    read_repo(cx, &other, 0);
    tick(cx, &other);
    assert_eq!(
        cx.update(|cx| changes_of(1, SharedChanges::Each, cx)),
        Some(1),
        "once git has answered there too"
    );
}

#[gpui_kit::test]
fn closing_a_window_lets_go_of_its_terminals(cx: &mut TestAppContext) {
    if isolate("closing_a_window_lets_go_of_its_terminals") {
        return;
    }
    let (window, root) = working_agent(cx, "closes");
    write(&root, "new.txt");
    read_repo(cx, &window, 1);
    drive(cx, &window, |ws, window, cx| ws.tear_off_at(1, 600.0, 300.0, window, cx));
    let other = cx
        .update(|cx| app(cx).windows.iter().find(|w| w.id != window.id).cloned())
        .expect("a window of its own");
    tick(cx, &other);
    assert_eq!(cx.update(|cx| changes_of(1, SharedChanges::Each, cx)), Some(1));

    cx.update(|cx| other.handle.update(cx, |_, window, _| window.remove_window())).expect("open");
    cx.run_until_parked();
    assert_eq!(
        cx.update(|cx| changes_of(1, SharedChanges::Each, cx)),
        None,
        "the window's terminals are forgotten with it"
    );
    // The other window's own terminals are not.
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 1);
}

/// What a read does with the picture it brings: kept for the answer that is wanted, not for one that
/// was superseded in flight, and none for a status against a branch.
#[gpui_kit::test]
fn a_read_brings_a_picture_only_when_its_answer_is_applied_and_not_against_a_branch(cx: &mut TestAppContext) {
    if isolate("a_read_brings_a_picture_only_when_its_answer_is_applied_and_not_against_a_branch") {
        return;
    }
    let (window, root) = working_agent(cx, "compares");
    write(&root, "n1.txt");
    read_repo(cx, &window, 0);
    assert_eq!(counts(cx, &window)[0], Some(1));

    // A read in flight when a branch is picked is superseded: its picture (which has `n2`) is
    // dropped with its status, and the one against the branch brings none.
    write(&root, "n2.txt");
    drive(cx, &window, |ws, _, cx| {
        ws.refresh_git(0, cx);
        ws.compare_with(0, Some("main".into()), cx);
    });
    assert_eq!(counts(cx, &window)[0], Some(1));
    // Reads against the branch bring none either.
    write(&root, "n3.txt");
    read_repo(cx, &window, 0);
    assert_eq!(counts(cx, &window)[0], Some(1));
    // Back at HEAD: the next picture sees what happened in the gap, to the one still working.
    drive(cx, &window, |ws, _, cx| ws.compare_with(0, None, cx));
    assert_eq!(counts(cx, &window)[0], Some(3));
}

/// The count can be the repository's instead: every terminal in it would show the same number, so the
/// rows say which repository it is (the list shows it once). Switching back and forth is at once.
#[gpui_kit::test]
fn the_count_can_be_the_repositorys_and_switching_is_at_once(cx: &mut TestAppContext) {
    if isolate("the_count_can_be_the_repositorys_and_switching_is_at_once") {
        return;
    }
    let (window, root) = working_agent(cx, "scope");
    write(&root, "new.txt");
    read_repo(cx, &window, 0);
    read_repo(cx, &window, 1);
    assert_eq!(counts(cx, &window), [Some(1), None], "the terminal's own, by default");

    cx.update(|cx| update_settings(cx, |s| s.sidebar_changes_scope = ChangesScope::Repository));
    cx.run_until_parked();
    assert_eq!(counts(cx, &window), [Some(1), Some(1)], "git's count, on both");
    let told = sidebar_of(cx, &window, |sidebar| {
        sidebar.input().rows.iter().map(|row| (row.changes, row.repo.clone())).collect::<Vec<_>>()
    });
    assert_eq!(told, [(Some(1), Some(root.clone())), (Some(1), Some(root.clone()))], "and the sidebar knows");

    cx.update(|cx| update_settings(cx, |s| s.sidebar_changes_scope = ChangesScope::Terminal));
    cx.run_until_parked();
    let told = sidebar_of(cx, &window, |sidebar| {
        sidebar.input().rows.iter().map(|row| row.changes).collect::<Vec<_>>()
    });
    assert_eq!(told, [Some(1), None], "back at once, without a read");
    draw(cx, &window);
}

#[gpui_kit::test]
fn files_changed_at_once_count_for_each_or_for_nobody_by_the_setting(cx: &mut TestAppContext) {
    if isolate("files_changed_at_once_count_for_each_or_for_nobody_by_the_setting") {
        return;
    }
    let root = repo("shared");
    let window = start(cx, 2);
    drive(cx, &window, |ws, _, cx| all_go(ws, &root, cx));
    read_repo(cx, &window, 0);
    read_repo(cx, &window, 1);
    tick(cx, &window);
    // Both agents work while the file appears.
    hook(cx, &window, 1, "UserPromptSubmit", None);
    hook(cx, &window, 2, "UserPromptSubmit", None);
    tick(cx, &window);
    write(&root, "new.txt");
    read_repo(cx, &window, 0);
    assert_eq!(counts(cx, &window), [Some(1), Some(1)], "each, by default");

    cx.update(|cx| update_settings(cx, |s| s.sidebar_changes_shared = SharedChanges::None));
    cx.run_until_parked();
    assert_eq!(counts(cx, &window), [None, None], "nobody's, at once");
    cx.update(|cx| update_settings(cx, |s| s.sidebar_changes_shared = SharedChanges::Each));
    cx.run_until_parked();
    assert_eq!(counts(cx, &window), [Some(1), Some(1)]);
}

/// What a terminal changed is kept in the profile's database while "Remember after restart" is on: back
/// with the terminal after a restart (here, the model forgetting it and the window reading the database
/// again), pruned by the first picture of the repository, and deleted when the setting goes off.
#[gpui_kit::test]
fn what_a_terminal_changed_is_kept_across_a_restart_while_the_setting_is_on(cx: &mut TestAppContext) {
    if isolate("what_a_terminal_changed_is_kept_across_a_restart_while_the_setting_is_on") {
        return;
    }
    let (window, root) = working_agent(cx, "remember");
    cx.update(|cx| app_mut(cx).store = Some(Store::open().expect("opens the scratch database")));
    write(&root, "new.txt");
    write(&root, "other.txt");
    read_repo(cx, &window, 0);
    assert_eq!(counts(cx, &window)[0], Some(2));
    let kept = |cx: &mut TestAppContext| -> Vec<String> {
        cx.update(|cx| {
            let profile = app(cx).active_profile().clone();
            let store = app(cx).store_of(&profile).expect("a store");
            store.claims(&[1, 2]).expect("reads").into_iter().map(|claim| claim.path).collect()
        })
    };
    assert!(kept(cx).is_empty(), "written when it has been quiet, not at once");
    drive(cx, &window, |ws, _, cx| ws.keep_changes_now(cx));
    assert_eq!(kept(cx), ["new.txt", "other.txt"]);

    // A restart: the terminal comes back (same id, from its saved session) and the model has nothing.
    drive(cx, &window, |ws, _, cx| {
        // Every terminal of the repository, so that the model has nothing of it, as after a launch.
        ws.forget_changes(1, cx);
        ws.forget_changes(2, cx);
        ws.restore_changes(cx);
    });
    assert_eq!(counts(cx, &window)[0], Some(2), "its count is back before git has been asked");
    let told =
        sidebar_of(cx, &window, |sidebar| sidebar.input().rows.iter().map(|r| r.changes).collect::<Vec<_>>());
    assert_eq!(told[0], Some(2), "and the sidebar was told, not only the window's reading");
    // While the app was closed, `other.txt` was committed. The first picture prunes it, and takes
    // nothing new: `later.txt` is nobody's.
    git(&root, &["add", "other.txt"]);
    git(&root, &["commit", "-q", "-m", "other"]);
    write(&root, "later.txt");
    drive(cx, &window, |ws, _, cx| ws.track_changes(cx));
    read_repo(cx, &window, 0);
    assert_eq!(counts(cx, &window)[0], Some(1), "pruned to `new.txt`");
    drive(cx, &window, |ws, _, cx| ws.keep_changes_now(cx));
    assert_eq!(kept(cx), ["new.txt"], "and what was committed is not kept either");

    // With the count itself switched off nothing is written for it: what is kept stays as it is.
    cx.update(|cx| update_settings(cx, |s| s.sidebar_changes = false));
    write(&root, "hidden.txt");
    read_repo(cx, &window, 0);
    drive(cx, &window, |ws, _, cx| ws.keep_changes_now(cx));
    assert_eq!(kept(cx), ["new.txt"], "nothing is written while the count is not shown");
    cx.update(|cx| update_settings(cx, |s| s.sidebar_changes = true));

    // Off: nothing is written and what is kept goes.
    cx.update(|cx| update_settings(cx, |s| s.sidebar_changes_remember = false));
    drive(cx, &window, |ws, _, cx| ws.keep_changes(cx));
    assert!(kept(cx).is_empty());
    write(&root, "again.txt");
    read_repo(cx, &window, 0);
    drive(cx, &window, |ws, _, cx| ws.keep_changes_now(cx));
    assert!(kept(cx).is_empty(), "nothing is written while it is off");
}

/// A terminal that moves to another window takes with it what is not written yet: the window it is in
/// now writes it, though that window knew nothing of it.
#[gpui_kit::test]
fn a_change_not_written_yet_is_written_by_the_window_the_terminal_moved_to(cx: &mut TestAppContext) {
    if isolate("a_change_not_written_yet_is_written_by_the_window_the_terminal_moved_to") {
        return;
    }
    let (window, root) = working_agent(cx, "tears");
    cx.update(|cx| app_mut(cx).store = Some(Store::open().expect("opens the scratch database")));
    write(&root, "new.txt");
    read_repo(cx, &window, 0);
    assert_eq!(counts(cx, &window)[0], Some(1));
    // Not written: it has not been quiet yet.
    drive(cx, &window, |ws, _, cx| ws.keep_changes(cx));
    let kept = |cx: &mut TestAppContext| -> Vec<String> {
        cx.update(|cx| {
            let profile = app(cx).active_profile().clone();
            let store = app(cx).store_of(&profile).expect("a store");
            store.claims(&[1, 2]).expect("reads").into_iter().map(|claim| claim.path).collect()
        })
    };
    assert!(kept(cx).is_empty());

    drive(cx, &window, |ws, window, cx| ws.tear_off_at(1, 600.0, 300.0, window, cx));
    let other = cx
        .update(|cx| app(cx).windows.iter().find(|w| w.id != window.id).cloned())
        .expect("a window of its own");
    // The old window has nothing of it to write; the new one has.
    drive(cx, &window, |ws, _, cx| ws.keep_changes_now(cx));
    assert!(kept(cx).is_empty(), "the window it left writes none of its rows");
    drive(cx, &other, |ws, _, cx| ws.keep_changes_now(cx));
    assert_eq!(kept(cx), ["new.txt"], "the window it is in writes them");
}

/// A window that moves to another profile keeps its rows in that profile's database from then on: what
/// its terminals changed is written there, the old database having lost them with the window's rows.
#[gpui_kit::test]
fn what_a_terminal_changed_follows_its_window_to_another_profile(cx: &mut TestAppContext) {
    if isolate("what_a_terminal_changed_follows_its_window_to_another_profile") {
        return;
    }
    use wormhole_core::config::profile::{NewProfile, StartFrom};
    let (window, root) = working_agent(cx, "rehomes");
    cx.update(|cx| app_mut(cx).store = Some(Store::open().expect("opens the scratch database")));
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            let id = crate::app_state::AppState::create_profile(new, cx)?;
            crate::app_state::AppState::ensure_open(&id, cx);
            Ok::<_, String>(id)
        })
        .expect("makes the profile");
    // The window is in the old database, as it is once it has been saved.
    drive(cx, &window, |ws, _, cx| ws.save(cx));
    write(&root, "new.txt");
    read_repo(cx, &window, 0);
    drive(cx, &window, |ws, _, cx| ws.keep_changes_now(cx));
    let kept_in =
        |cx: &mut TestAppContext, profile: &wormhole_core::storage::paths::ProfileId| -> Vec<String> {
            cx.update(|cx| {
                let store = app(cx).store_of(profile).expect("a store");
                store.claims(&[1, 2]).expect("reads").into_iter().map(|claim| claim.path).collect()
            })
        };
    let default = wormhole_core::storage::paths::ProfileId::default_profile();
    assert_eq!(kept_in(cx, &default), ["new.txt"]);

    drive(cx, &window, |ws, _, cx| ws.rehome(&work, cx).expect("the window moves"));
    assert!(kept_in(cx, &default).is_empty(), "the old database lost them with the window");
    drive(cx, &window, |ws, _, cx| ws.keep_changes_now(cx));
    assert_eq!(kept_in(cx, &work), ["new.txt"], "the new one has them");
    assert_eq!(counts(cx, &window)[0], Some(1), "and the count is as it was");
}
