//! What the History tab and its branch picker do through the window, pinned from outside: the commits
//! of the terminal's branch, a commit's files and their diffs, another branch's log, the palette
//! revealing a commit, and the tab's refresh on the tick. The helpers at the top are the only place a
//! test names where the state is kept.

use gpui_kit::{Context, Entity, TestAppContext, Window};
use wormhole_core::config::TabId;
use wormhole_core::git::Commit;

use super::super::changes::Changes;
use super::super::picker::PickerHost;
use super::super::testing::{changes, folder, git, go, repo, write};
use super::{History, HistoryState};
use crate::app_state::OpenWindow;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};

use crate::window::Workspace;
use crate::window::view_tests::{draw, drive, isolate_in, read, read_cx, start, start_with};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// The window's History.
fn history(ws: &Workspace) -> Entity<History> {
    ws.feature::<History>().expect("the window has the History feature")
}

/// What the tab keeps for the terminal in front.
fn with_state<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&HistoryState, &gpui_kit::App) -> R,
) -> R {
    read_cx(cx, window, |ws, cx| {
        let entity = history(ws);
        let state = entity.read(cx).of.get(&ws.active_session().id).expect("a state for the terminal");
        f(state, cx)
    })
}

/// The subjects of the commits the tab lists, newest first.
fn subjects(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    with_state(cx, window, |state, _| state.commits.iter().map(|c| c.subject.clone()).collect())
}

fn history_branch(cx: &mut TestAppContext, window: &OpenWindow) -> Option<String> {
    with_state(cx, window, |state, _| state.rev.clone())
}

fn expanded(cx: &mut TestAppContext, window: &OpenWindow) -> Option<String> {
    with_state(cx, window, |state, _| state.expanded.clone())
}

fn first_commit(cx: &mut TestAppContext, window: &OpenWindow) -> Commit {
    with_state(cx, window, |state, _| state.commits.first().cloned().expect("a commit"))
}

/// Whether the History picker is open.
fn picker_open(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read_cx(cx, window, |ws, cx| history(ws).read(cx).picker().is_open())
}

/// The person opens the History picker of the terminal in front.
fn open_picker(ws: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let (session, terminal) = (ws.active_session().id, ws.active_terminal());
    history(ws).update(cx, |history, cx| history.picker_mut().toggle(session, &terminal, window, cx));
}

/// The person opens (or closes) a commit in the list.
fn toggle_commit(cx: &mut TestAppContext, window: &OpenWindow, sha: &str) {
    drive(cx, window, |ws, _, cx| {
        let session = ws.active_session().id;
        history(ws).update(cx, |history, cx| history.toggle_commit(session, sha.to_string(), cx));
    });
    cx.run_until_parked();
}

/// A click on the first file of `commit`, which is open: its diff opens.
fn open_first_file(cx: &mut TestAppContext, window: &OpenWindow, commit: &Commit) {
    drive(cx, window, |ws, _, cx| {
        let session = ws.active_session().id;
        let changes = ws.feature::<Changes>().expect("in the table");
        history(ws).update(cx, |history, cx| {
            let file = history.of[&session].loaded(&commit.sha).and_then(|f| f.first().cloned());
            history.open_commit_file(session, commit.clone(), file.expect("a file"), &changes, cx);
        });
    });
    cx.run_until_parked();
}

/// The palette shows a commit.
fn reveal(cx: &mut TestAppContext, window: &OpenWindow, root: &std::path::Path, commit: &Commit) {
    drive(cx, window, |ws, _, cx| super::api::reveal_commit(ws, root.to_path_buf(), commit.clone(), cx));
    cx.run_until_parked();
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
    assert!(with_state(cx, &window, |state, _| state.root.is_some()));
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
    with_state(cx, &window, |history, _| {
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
    toggle_commit(cx, &window, &second.sha);
    assert_eq!(expanded(cx, &window), Some(second.sha.clone()));
    let files: Vec<String> = with_state(cx, &window, |history, _| {
        history.loaded(&second.sha).map(|f| f.iter().map(|f| f.path.clone()).collect()).unwrap_or_default()
    });
    assert_eq!(files, ["a.txt", "src/b.txt"]);

    // A file opens its diff, with the commit's short name before the path; it is not the working tree's.
    open_first_file(cx, &window, &second);
    read_cx(cx, &window, |ws, cx| {
        let entity = changes(ws);
        let on_view = entity.read(cx).diff.as_ref().expect("a diff").on_view();
        // A commit's file, not the working tree's; the arrows step through the commit's two files.
        assert_eq!(on_view, ("a.txt".to_string(), false, 2));
    });
    // Closing the commit again.
    toggle_commit(cx, &window, &second.sha);
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
    drive(cx, &window, open_picker);
    cx.run_until_parked();
    read_cx(cx, &window, |ws, cx| {
        let entity = history(ws);
        let history = entity.read(cx);
        assert!(history.picker().is_open());
        let mut branches = history.picker().branches().to_vec();
        branches.sort();
        assert_eq!(branches, ["main", "other"]);
    });
    drive(cx, &window, |ws, _, cx| pick(ws, Some("other".into()), cx));
    cx.run_until_parked();
    assert_eq!(history_branch(cx, &window).as_deref(), Some("other"));
    assert_eq!(subjects(cx, &window), ["on other", "second", "first"]);
    assert!(!picker_open(cx, &window));
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
    let root = with_state(cx, &window, |state, _| state.root.clone().expect("a root"));
    // On another tab, with the log on `other`: `second` is on it, so the branch stays.
    drive(cx, &window, |ws, _, cx| {
        pick(ws, Some("other".into()), cx);
        ws.show_tab(TabId::Notes, cx);
    });
    cx.run_until_parked();
    reveal(cx, &window, &root, &second);
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
    reveal(cx, &window, &root, &on_other);
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

/// A folder that becomes a repository (`git init`) has its log on the next tick, not at the next period.
#[gpui_kit::test]
fn a_folder_that_becomes_a_repository_gets_its_log_at_once(cx: &mut TestAppContext) {
    if isolate("a_folder_that_becomes_a_repository_gets_its_log_at_once") {
        return;
    }
    let plain = folder("becomes-a-repository");
    let record = WindowRecord {
        id: 1,
        sessions: vec![SessionRecord::new(1, None, Some(plain.to_string_lossy().into_owned()))],
        ..Default::default()
    };
    let window = start_with(cx, record);
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::History, cx));
    cx.run_until_parked();
    with_state(cx, &window, |state, _| assert!(state.root.is_none() && state.answered_for(&plain)));
    git(&plain, &["init", "-q", "-b", "main"]);
    write(&plain, &[("a.txt", "one\n")]);
    git(&plain, &["add", "-A"]);
    git(&plain, &["commit", "-q", "-m", "first"]);
    // The window asks git about the folder (its own poll), then the tick that is not the 12th.
    drive(cx, &window, |ws, _, cx| {
        let ix = ws.active;
        ws.refresh_git(ix, cx);
    });
    cx.run_until_parked();
    drive(cx, &window, |ws, window, cx| {
        ws.ticks = 1;
        ws.tick(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(subjects(cx, &window), ["first"]);
}

/// A restart puts a new terminal view under the same id: History reads the log in the folder of the one
/// that lives now, not of the dead one it first saw.
#[gpui_kit::test]
fn the_tab_follows_a_terminal_that_was_restarted(cx: &mut TestAppContext) {
    if isolate("the_tab_follows_a_terminal_that_was_restarted") {
        return;
    }
    let first = make_two_commits("restart-first");
    let other = repo("restart-other", &[("z.txt", "z\n")]);
    let record = WindowRecord {
        id: 1,
        sessions: vec![SessionRecord::new(1, None, Some(first.to_string_lossy().into_owned()))],
        ..Default::default()
    };
    let window = start_with(cx, record);
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::History, cx));
    cx.run_until_parked();
    assert_eq!(subjects(cx, &window), ["second", "first"]);
    let old = read(cx, &window, |ws| ws.active_terminal());
    // The shell goes to another repository and is restarted there; the dead view stays where it was.
    drive(cx, &window, |ws, window, cx| {
        go(ws, &other, cx);
        let id = ws.active_session().id;
        ws.on_terminal_event(id, crate::terminal_view::TerminalEvent::Restart, window, cx)
    });
    assert_ne!(read(cx, &window, |ws| ws.active_terminal()), old, "a new view under the same id");
    cx.update(|cx| old.update(cx, |terminal, _| terminal.cwd = Some(first.clone())));
    drive(cx, &window, |ws, window, cx| {
        ws.ticks = 11;
        ws.tick(window, cx);
    });
    cx.run_until_parked();
    assert_eq!(subjects(cx, &window), ["first"], "the log of the folder the live terminal is in");
}

/// The person picks a branch (`None`: HEAD) in the History picker.
fn pick(ws: &mut Workspace, branch: Option<String>, cx: &mut gpui_kit::Context<Workspace>) {
    let session = ws.active_session().id;
    history(ws).update(cx, |history, cx| {
        history.picker_mut().close();
        history.pick(session, branch, cx);
    });
}

// ---------------------------------------------------------------- the frame

/// Allocations the third draw of a one-terminal window on History (two commits, the newest open with its
/// two files) may make. Measured: 3,792 allocations (1.8 MB), the same on every run (3,790 before History
/// moved into its entity). The budget is that plus
/// about a hundredth; the tab adds a
/// row for each commit and file in view, so a cost per commit or per file per frame shows at once.
const HISTORY_FRAME_ALLOCATIONS: u64 = 3_830;

#[gpui_kit::test]
fn a_window_frame_on_history_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    if isolate("a_window_frame_on_history_stays_within_its_allocation_budget") {
        return;
    }
    let window = start(cx, 1);
    two_commits(cx, &window, "history-frame");
    let second = first_commit(cx, &window);
    toggle_commit(cx, &window, &second.sha);
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::History);
    // The first draws build what a window keeps; the third is a frame.
    draw(cx, &window);
    draw(cx, &window);
    let before = thread_counts();
    draw(cx, &window);
    let drawn = thread_counts().since(before);
    eprintln!("window frame on History: {} allocations, {} bytes", drawn.allocations, drawn.bytes);
    assert!(
        drawn.allocations <= HISTORY_FRAME_ALLOCATIONS,
        "a window frame on History made {} allocations (budget {HISTORY_FRAME_ALLOCATIONS})",
        drawn.allocations
    );
}
