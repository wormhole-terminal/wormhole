//! What the History tab and its branch picker do through the window, pinned from outside: the commits
//! of the terminal's branch, a commit's files and their diffs, another branch's log, the palette
//! revealing a commit, and the tab's refresh on the tick. The helpers at the top are the only place a
//! test names where the state is kept.

use gpui_kit::TestAppContext;
use wormhole_core::config::TabId;
use wormhole_core::git::Commit;

use super::super::picker::PickerHost;
use super::super::testing::{changes, folder, git, go, repo, write};
use crate::app_state::OpenWindow;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};

use crate::window::Workspace;
use crate::window::view_tests::{draw, drive, isolate_in, read, read_cx, start, start_with};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// The subjects of the commits the tab lists, newest first.
fn subjects(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    read(cx, window, |ws| ws.active_session().history.commits.iter().map(|c| c.subject.clone()).collect())
}

fn history_branch(cx: &mut TestAppContext, window: &OpenWindow) -> Option<String> {
    read(cx, window, |ws| ws.active_session().history.rev.clone())
}

fn expanded(cx: &mut TestAppContext, window: &OpenWindow) -> Option<String> {
    read(cx, window, |ws| ws.active_session().history.expanded.clone())
}

fn first_commit(cx: &mut TestAppContext, window: &OpenWindow) -> Commit {
    read(cx, window, |ws| ws.active_session().history.commits.first().cloned().expect("a commit"))
}

/// A repository of two commits (`first`, `second`) with `other` one commit further (`on other`), and the
/// window's terminal in it on the History tab.
fn two_commits(cx: &mut TestAppContext, window: &OpenWindow, name: &str) {
    let root = make_two_commits(name);
    drive(cx, window, |ws, _, cx| {
        go(ws, &root, cx);
        ws.show_tab(TabId::History, cx);
    });
    cx.run_until_parked();
}

/// The repository of [`two_commits`], not yet visited.
fn make_two_commits(name: &str) -> std::path::PathBuf {
    let root = repo(name, &[("a.txt", "one\n")]);
    write(&root, &[("a.txt", "one\ntwo\n"), ("src/b.txt", "b\n")]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "second"]);
    git(&root, &["checkout", "-q", "-b", "other"]);
    write(&root, &[("c.txt", "c\n")]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "on other"]);
    git(&root, &["checkout", "-q", "main"]);
    root
}

// ---------------------------------------------------------------- the log

#[gpui_kit::test]
fn the_history_tab_lists_the_commits_of_the_terminals_branch(cx: &mut TestAppContext) {
    if isolate("the_history_tab_lists_the_commits_of_the_terminals_branch") {
        return;
    }
    let window = start(cx, 1);
    two_commits(cx, &window, "log");
    assert_eq!(subjects(cx, &window), ["second", "first"]);
    assert_eq!(history_branch(cx, &window), None, "HEAD");
    assert!(read(cx, &window, |ws| ws.active_session().history.root.is_some()));
    draw(cx, &window);
    let words = drive(cx, &window, |ws, _, cx| {
        let (at, tab) = ws.tab_entry(TabId::History).map_or((0, None), |(at, tab)| (at, Some(tab)));
        (ws.panel_tab_list(cx).contains(&TabId::History), at, tab.map(|tab| tab.label))
    });
    assert!(words.0, "History is offered");
}

#[gpui_kit::test]
fn a_folder_outside_any_repository_has_no_history(cx: &mut TestAppContext) {
    if isolate("a_folder_outside_any_repository_has_no_history") {
        return;
    }
    let window = start(cx, 1);
    let plain = folder("plain-history");
    drive(cx, &window, |ws, _, cx| {
        go(ws, &plain, cx);
        ws.show_tab(TabId::History, cx);
    });
    cx.run_until_parked();
    read(cx, &window, |ws| {
        let history = &ws.active_session().history;
        assert!(history.root.is_none() && history.commits.is_empty());
        assert!(history.answered_for(&plain), "answered, so the tab says so rather than reading");
    });
    draw(cx, &window);
}

#[gpui_kit::test]
fn a_commit_opens_to_its_files_and_a_file_opens_its_diff(cx: &mut TestAppContext) {
    if isolate("a_commit_opens_to_its_files_and_a_file_opens_its_diff") {
        return;
    }
    let window = start(cx, 1);
    two_commits(cx, &window, "files");
    let second = first_commit(cx, &window);
    drive(cx, &window, |ws, _, cx| ws.toggle_commit(second.sha.clone(), cx));
    cx.run_until_parked();
    assert_eq!(expanded(cx, &window), Some(second.sha.clone()));
    let files: Vec<String> = read(cx, &window, |ws| {
        let history = &ws.active_session().history;
        history.loaded(&second.sha).map(|f| f.iter().map(|f| f.path.clone()).collect()).unwrap_or_default()
    });
    assert_eq!(files, ["a.txt", "src/b.txt"]);

    // A file opens its diff, with the commit's short name before the path; it is not the working tree's.
    drive(cx, &window, |ws, _, cx| {
        let file = ws.active_session().history.loaded(&second.sha).and_then(|f| f.first().cloned());
        ws.open_commit_file(second.clone(), file.expect("a file"), cx);
    });
    cx.run_until_parked();
    read_cx(cx, &window, |ws, cx| {
        let entity = changes(ws);
        let on_view = entity.read(cx).diff.as_ref().expect("a diff").on_view();
        // A commit's file, not the working tree's; the arrows step through the commit's two files.
        assert_eq!(on_view, ("a.txt".to_string(), false, 2));
    });
    // Closing the commit again.
    drive(cx, &window, |ws, _, cx| ws.toggle_commit(second.sha.clone(), cx));
    assert_eq!(expanded(cx, &window), None);
}

// ---------------------------------------------------------------- the branch picker

#[gpui_kit::test]
fn another_branchs_log_is_shown_when_it_is_picked(cx: &mut TestAppContext) {
    if isolate("another_branchs_log_is_shown_when_it_is_picked") {
        return;
    }
    let window = start(cx, 1);
    two_commits(cx, &window, "branch-log");
    drive(cx, &window, |ws, window, cx| {
        let (session, terminal) = (ws.active_session().id, ws.active_terminal());
        ws.history_picker.toggle(session, &terminal, window, cx);
    });
    cx.run_until_parked();
    read(cx, &window, |ws| {
        assert!(ws.history_picker.is_open());
        let mut branches = ws.history_picker.branches().to_vec();
        branches.sort();
        assert_eq!(branches, ["main", "other"]);
    });
    drive(cx, &window, |ws, _, cx| pick(ws, Some("other".into()), cx));
    cx.run_until_parked();
    assert_eq!(history_branch(cx, &window).as_deref(), Some("other"));
    assert_eq!(subjects(cx, &window), ["on other", "second", "first"]);
    assert!(!read(cx, &window, |ws| ws.history_picker.is_open()));
    // Back to HEAD.
    drive(cx, &window, |ws, _, cx| pick(ws, None, cx));
    cx.run_until_parked();
    assert_eq!(subjects(cx, &window), ["second", "first"]);
}

#[gpui_kit::test]
fn a_commit_found_in_the_palette_is_revealed_on_its_branch_or_on_head(cx: &mut TestAppContext) {
    if isolate("a_commit_found_in_the_palette_is_revealed_on_its_branch_or_on_head") {
        return;
    }
    let window = start(cx, 1);
    two_commits(cx, &window, "reveal");
    let second = first_commit(cx, &window);
    let root = read(cx, &window, |ws| ws.active_session().history.root.clone().expect("a root"));
    // On another tab, with the log on `other`: `second` is on it, so the branch stays.
    drive(cx, &window, |ws, _, cx| {
        pick(ws, Some("other".into()), cx);
        ws.show_tab(TabId::Notes, cx);
    });
    cx.run_until_parked();
    drive(cx, &window, |ws, _, cx| ws.reveal_commit(root.clone(), second.clone(), cx));
    cx.run_until_parked();
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::History);
    assert_eq!(expanded(cx, &window), Some(second.sha.clone()));
    assert_eq!(history_branch(cx, &window).as_deref(), Some("other"));
    // A commit that is not on the branch shown puts the log back on HEAD.
    let on_other = {
        drive(cx, &window, |ws, _, cx| pick(ws, Some("other".into()), cx));
        cx.run_until_parked();
        first_commit(cx, &window)
    };
    drive(cx, &window, |ws, _, cx| pick(ws, None, cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, _, cx| {
        pick(ws, Some("main".into()), cx);
    });
    cx.run_until_parked();
    drive(cx, &window, |ws, _, cx| ws.reveal_commit(root.clone(), on_other.clone(), cx));
    cx.run_until_parked();
    assert_eq!(history_branch(cx, &window), None, "not on main: the log goes back to HEAD");
    assert_eq!(expanded(cx, &window), Some(on_other.sha));
}

// ---------------------------------------------------------------- the tick

#[gpui_kit::test]
fn the_tab_reads_the_log_again_on_the_tick_while_it_shows(cx: &mut TestAppContext) {
    if isolate("the_tab_reads_the_log_again_on_the_tick_while_it_shows") {
        return;
    }
    // The terminal starts in the repository: the tick asks the terminal where it is, which a harness
    // shell that never reports a folder would answer with its own.
    let root = make_two_commits("ticks");
    let record = WindowRecord {
        id: 1,
        sessions: vec![SessionRecord::new(1, None, Some(root.to_string_lossy().into_owned()))],
        ..Default::default()
    };
    let window = start_with(cx, record);
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::History, cx));
    cx.run_until_parked();
    assert_eq!(subjects(cx, &window), ["second", "first"]);
    write(&root, &[("late.txt", "x\n")]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "third"]);
    // Every 12th tick (about 5 s), not each one.
    drive(cx, &window, |ws, window, cx| {
        ws.ticks = 11;
        ws.tick(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(subjects(cx, &window), ["third", "second", "first"]);
    // On another tab the log is left alone.
    write(&root, &[("later.txt", "x\n")]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "fourth"]);
    drive(cx, &window, |ws, window, cx| {
        ws.show_tab(TabId::Notes, cx);
        ws.ticks = 22;
        ws.tick(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(subjects(cx, &window).len(), 3);
}

/// The person picks a branch (`None`: HEAD) in the History picker.
fn pick(ws: &mut Workspace, branch: Option<String>, cx: &mut gpui_kit::Context<Workspace>) {
    let session = ws.active_session().id;
    ws.history_picker.close();
    ws.pick(session, branch, cx);
}
