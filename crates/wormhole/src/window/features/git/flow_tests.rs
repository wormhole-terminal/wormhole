//! What the Changes tab and the branch picker do through the window, pinned from outside: a terminal
//! in a repository lists its changed files, a folder outside one says so, the keys walk the list and
//! open diffs, "compare with" picks a branch, and the tab's words are VoiceOver's. The state is read
//! through the helpers at the top of each section, which are the only places that name where it is
//! kept.

use gpui_kit::{Context, Keystroke, TestAppContext, Window};

use crate::window::Workspace;
use wormhole_core::config::TabId;
use wormhole_core::git::ChangeKind;
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::view::spoken::{self, PanelTab};

use super::picker::PickerHost;
use super::testing::{
    change_of, changed_paths, changes, diff_path, folder, git, go, open_change, read_repo, repo, walkable,
    write,
};
use crate::window::view_tests::{draw, drive, isolate_in, read, read_cx, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// The Changes tab's rows the keys can be on, as the window asks the table.
fn rows(cx: &mut TestAppContext, window: &crate::app_state::OpenWindow) -> usize {
    // The list is kept up to date by the frame.
    draw(cx, window);
    read_cx(cx, window, |ws, cx| walkable(ws, cx).len())
}

/// The path of the diff on view.
fn open_path(cx: &mut TestAppContext, window: &crate::app_state::OpenWindow) -> Option<String> {
    read_cx(cx, window, diff_path)
}

/// The branch the Changes tab compares with.
fn compared_with(cx: &mut TestAppContext, window: &crate::app_state::OpenWindow) -> Option<String> {
    read(cx, window, |ws| ws.active_session().git.compare.clone())
}

/// A repository with `a.txt` modified, `b.txt` untracked, and `main` plus `other` (a commit ahead of
/// where it left `main`) as branches.
fn dirty_repo(
    cx: &mut TestAppContext,
    window: &crate::app_state::OpenWindow,
    name: &str,
) -> std::path::PathBuf {
    let root = repo(name, &[("a.txt", "one\ntwo\n"), ("src/c.txt", "c\n")]);
    git(&root, &["checkout", "-q", "-b", "other"]);
    write(&root, &[("only-on-other.txt", "x\n")]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "on other"]);
    git(&root, &["checkout", "-q", "main"]);
    write(&root, &[("a.txt", "one\ntwo\nthree\n"), ("b.txt", "new\n")]);
    drive(cx, window, |ws, _, cx| go(ws, &root, cx));
    read_repo(cx, window);
    root
}

// ---------------------------------------------------------------- the list

#[gpui_kit::test]
fn a_terminal_in_a_repository_lists_what_changed(cx: &mut TestAppContext) {
    if isolate("a_terminal_in_a_repository_lists_what_changed") {
        return;
    }
    let window = start(cx, 1);
    let root = dirty_repo(cx, &window, "listed");
    assert_eq!(read(cx, &window, changed_paths), Some(vec!["a.txt".to_string(), "b.txt".to_string()]));
    read(cx, &window, |ws| {
        assert_eq!(change_of(ws, "a.txt").kind, ChangeKind::Modified);
        assert_eq!(change_of(ws, "b.txt").kind, ChangeKind::Untracked);
        let status = ws.active_session().git.status.as_ref().expect("a status");
        assert_eq!((status.root.clone(), status.branch.as_deref()), (root, Some("main")));
    });
    // The sidebar's count and the tab's number and words come from the same answer.
    assert_eq!(read_cx(cx, &window, |ws, cx| ws.active_changes(cx)), Some(2));
    drive(cx, &window, |ws, _, cx| {
        let (at, tab) = ws.tab_entry(TabId::Changes).expect("Changes is in the table");
        let entity = ws.feature_entity(at).expect("made with the window").clone();
        let facts = ws.facts(cx);
        assert_eq!((tab.count.expect("counts"))(&entity, &facts, cx), Some(2));
        assert_eq!(
            (tab.spoken)(&entity, &facts, cx),
            spoken::panel_tab(PanelTab::Changes { files: Some(2) })
        );
        assert_eq!(tab.action, ActionId::ShowChanges);
    });
    // A band for the section, then the two files (the untracked one in a section of its own or not).
    assert!(rows(cx, &window) >= 3);
}

#[gpui_kit::test]
fn a_folder_outside_any_repository_is_not_one(cx: &mut TestAppContext) {
    if isolate("a_folder_outside_any_repository_is_not_one") {
        return;
    }
    let window = start(cx, 1);
    let plain = folder("plain");
    drive(cx, &window, |ws, _, cx| go(ws, &plain, cx));
    read_repo(cx, &window);
    read(cx, &window, |ws| {
        let git = &ws.active_session().git;
        assert!(git.status.is_none() && git.cwd.as_deref() == Some(plain.as_path()), "answered: none");
        assert!(git.answered_for(&plain) && git.is_absent_for(&plain), "and not asked about again");
    });
    assert_eq!(read_cx(cx, &window, |ws, cx| ws.active_changes(cx)), None);
    assert_eq!(rows(cx, &window), 0);
    draw(cx, &window);
}

#[gpui_kit::test]
fn a_clean_repository_has_a_list_with_nothing_in_it(cx: &mut TestAppContext) {
    if isolate("a_clean_repository_has_a_list_with_nothing_in_it") {
        return;
    }
    let window = start(cx, 1);
    let clean = repo("clean", &[("a.txt", "one\n")]);
    drive(cx, &window, |ws, _, cx| go(ws, &clean, cx));
    read_repo(cx, &window);
    assert_eq!(read(cx, &window, changed_paths), Some(Vec::new()));
    assert_eq!(read_cx(cx, &window, |ws, cx| ws.active_changes(cx)), None, "no count for no changes");
    draw(cx, &window);
}

#[gpui_kit::test]
fn the_list_follows_the_working_tree_at_each_refresh(cx: &mut TestAppContext) {
    if isolate("the_list_follows_the_working_tree_at_each_refresh") {
        return;
    }
    let window = start(cx, 1);
    let root = dirty_repo(cx, &window, "follows");
    write(&root, &[("src/c.txt", "changed\n")]);
    read_repo(cx, &window);
    assert_eq!(
        read(cx, &window, changed_paths),
        Some(vec!["a.txt".to_string(), "b.txt".to_string(), "src/c.txt".to_string()])
    );
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "second"]);
    read_repo(cx, &window);
    assert_eq!(read(cx, &window, changed_paths), Some(Vec::new()), "committed");
}

// ---------------------------------------------------------------- the keys

#[gpui_kit::test]
fn the_keys_walk_the_list_and_return_opens_a_diff(cx: &mut TestAppContext) {
    if isolate("the_keys_walk_the_list_and_return_opens_a_diff") {
        return;
    }
    let window = start(cx, 1);
    dirty_repo(cx, &window, "keys");
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::Changes, cx));
    draw(cx, &window);
    let list = read_cx(cx, &window, walkable);
    assert!(list[0].band, "the section's band leads the list");
    let first_file = list.iter().find(|row| !row.band && row.item.fold.is_none()).expect("a file row").row;
    // ↓ moves the cursor onto the first row, then on.
    drive(cx, &window, |ws, window, cx| {
        assert!(ws.panel_key(&Keystroke::parse("down").expect("a key"), window, cx));
        assert!(ws.panel_key(&Keystroke::parse("down").expect("a key"), window, cx));
        assert_eq!(
            ws.areas.panel_at.map(|(tab, row)| (tab, row > list[0].row)),
            Some((TabId::Changes, true))
        );
    });
    // ↩ on a file opens its diff and keeps the keys in the list.
    drive(cx, &window, |ws, window, cx| {
        ws.areas.panel_at = Some((TabId::Changes, first_file));
        assert!(ws.panel_key(&Keystroke::parse("enter").expect("a key"), window, cx));
    });
    assert!(open_path(cx, &window).is_some(), "the diff is open");
    // ↩ on the band does nothing; Space folds it, and the rows change shape.
    let before = rows(cx, &window);
    drive(cx, &window, |ws, window, cx| {
        ws.areas.panel_at = Some((TabId::Changes, list[0].row));
        assert!(ws.panel_key(&Keystroke::parse("space").expect("a key"), window, cx));
    });
    assert_ne!(rows(cx, &window), before, "the section folded");
}

// ---------------------------------------------------------------- compare with

#[gpui_kit::test]
fn comparing_with_a_branch_lists_the_changes_against_it_and_closes_the_diff(cx: &mut TestAppContext) {
    if isolate("comparing_with_a_branch_lists_the_changes_against_it_and_closes_the_diff") {
        return;
    }
    let window = start(cx, 1);
    dirty_repo(cx, &window, "compare");
    draw(cx, &window);
    drive(cx, &window, |ws, _, cx| open_change(ws, "a.txt", cx));
    cx.run_until_parked();
    assert!(open_path(cx, &window).is_some());

    // The picker opens, asks for the branches, and shows them once they are in.
    drive(cx, &window, toggle_compare);
    cx.run_until_parked();
    read_cx(cx, &window, |ws, cx| {
        let entity = changes(ws);
        let changes = entity.read(cx);
        assert!(changes.picker().is_open());
        let mut branches = changes.picker().branches().to_vec();
        branches.sort();
        assert_eq!(branches, ["main", "other"]);
    });
    // Picking one reads the status against it and drops the diff (its file may be gone from the list).
    drive(cx, &window, |ws, _, cx| pick_compare(ws, Some("other".into()), cx));
    cx.run_until_parked();
    assert_eq!(compared_with(cx, &window).as_deref(), Some("other"));
    assert!(open_path(cx, &window).is_none(), "the diff closed");
    read_cx(cx, &window, |ws, cx| {
        assert!(!changes(ws).read(cx).picker().is_open());
        let status = ws.active_session().git.status.as_ref().expect("read again");
        assert_eq!(status.compare.as_deref(), Some("other"));
    });
    // Back to HEAD.
    drive(cx, &window, |ws, _, cx| pick_compare(ws, None, cx));
    cx.run_until_parked();
    assert_eq!(compared_with(cx, &window), None);
    read(cx, &window, |ws| {
        assert_eq!(ws.active_session().git.status.as_ref().and_then(|s| s.compare.clone()), None);
    });
}

#[gpui_kit::test]
fn a_picker_belongs_to_the_terminal_and_the_tab_it_was_opened_in(cx: &mut TestAppContext) {
    if isolate("a_picker_belongs_to_the_terminal_and_the_tab_it_was_opened_in") {
        return;
    }
    let window = start(cx, 2);
    dirty_repo(cx, &window, "picker-owner");
    let compare_open =
        |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| changes(ws).read(cx).picker().is_open());
    drive(cx, &window, toggle_compare);
    cx.run_until_parked();
    assert!(compare_open(cx));
    // Another tab: the picker is gone.
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::History, cx));
    assert!(!compare_open(cx));
    // The History tab's picker: gone with the tab too.
    drive(cx, &window, |ws, window, cx| {
        let (session, terminal) = (ws.active_session().id, ws.active_terminal());
        let history = ws.feature::<super::history::History>().expect("in the table");
        history.update(cx, |history, cx| history.picker_mut().toggle(session, &terminal, window, cx));
    });
    let history_open = |cx: &mut TestAppContext| {
        read_cx(cx, &window, |ws, cx| {
            ws.feature::<super::history::History>().is_some_and(|h| h.read(cx).picker().is_open())
        })
    };
    assert!(history_open(cx));
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::Changes, cx));
    assert!(!history_open(cx));
    // Another terminal: the same for the compare picker, and an open diff goes too.
    drive(cx, &window, toggle_compare);
    assert!(compare_open(cx));
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    assert!(!compare_open(cx));
    assert_eq!(open_path(cx, &window), None);
}

// ---------------------------------------------------------------- keys and menus

#[gpui_kit::test]
fn the_history_key_shows_the_tab_and_the_menu_checks_it(cx: &mut TestAppContext) {
    use crate::actions::ShowHistory;
    use crate::window::view_tests::with_window;
    if isolate("the_history_key_shows_the_tab_and_the_menu_checks_it") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.focus_terminal(window, cx));
    draw(cx, &window);
    with_window(cx, &window, |window, cx| window.dispatch_action(Box::new(ShowHistory), cx));
    cx.run_until_parked();
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::History);
    let checked = read_cx(cx, &window, |ws, cx| ws.menu_state(cx).checked);
    assert!(checked.contains(&ActionId::ShowHistory) && !checked.contains(&ActionId::ShowChanges));
    // The menu line and the palette list it, enabled.
    let state = read_cx(cx, &window, |ws, cx| ws.menu_state(cx));
    assert!(
        !state.hidden.contains(&ActionId::ShowHistory) && !state.disabled.contains(&ActionId::ShowHistory)
    );
}

#[gpui_kit::test]
fn the_worktree_action_is_bound_only_where_the_folder_is_in_a_repository(cx: &mut TestAppContext) {
    use crate::actions::NewTerminalInWorktree;
    use crate::window::view_tests::with_window;
    if isolate("the_worktree_action_is_bound_only_where_the_folder_is_in_a_repository") {
        return;
    }
    let window = start(cx, 1);
    let plain = folder("worktree-key-plain");
    drive(cx, &window, |ws, window, cx| {
        go(ws, &plain, cx);
        ws.focus_terminal(window, cx);
    });
    read_repo(cx, &window);
    draw(cx, &window);
    with_window(cx, &window, |window, cx| window.dispatch_action(Box::new(NewTerminalInWorktree), cx));
    cx.run_until_parked();
    assert!(read(cx, &window, |ws| ws.worktree_ask.is_none()), "no repository: not handled, so greyed");
    let root = repo("worktree-key-repo", &[("a.txt", "one\n")]);
    drive(cx, &window, |ws, _, cx| go(ws, &root, cx));
    read_repo(cx, &window);
    draw(cx, &window);
    with_window(cx, &window, |window, cx| window.dispatch_action(Box::new(NewTerminalInWorktree), cx));
    cx.run_until_parked();
    assert!(read(cx, &window, |ws| ws.worktree_ask.is_some()), "in one: the dialog");
}

// ---------------------------------------------------------------- the compare picker

/// The person opens (or closes) the "compare with" picker of the terminal in front.
fn toggle_compare(ws: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let (session, terminal) = (ws.active_session().id, ws.active_terminal());
    changes(ws).update(cx, |changes, cx| changes.picker_mut().toggle(session, &terminal, window, cx));
}

/// The person picks a branch (`None`: HEAD) in the picker.
fn pick_compare(ws: &mut Workspace, branch: Option<String>, cx: &mut Context<Workspace>) {
    let session = ws.active_session().id;
    changes(ws).update(cx, |changes, cx| {
        changes.picker_mut().close();
        changes.pick(session, branch, cx);
    });
}

// ---------------------------------------------------------------- History off

#[gpui_kit::test]
fn history_off_hides_its_tab_key_menu_line_and_commits_and_never_changes_or_worktrees(
    cx: &mut TestAppContext,
) {
    use crate::actions::ShowHistory;
    use crate::window::view_tests::with_window;
    use wormhole_core::config::FeatureId;
    if isolate("history_off_hides_its_tab_key_menu_line_and_commits_and_never_changes_or_worktrees") {
        return;
    }
    let window = start(cx, 1);
    let root = dirty_repo(cx, &window, "history-off");
    // The log is read once while History is on, and kept while it is off.
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::History, cx));
    cx.run_until_parked();
    let commits = |cx: &mut TestAppContext| {
        read_cx(cx, &window, |ws, cx| {
            let history = ws.feature::<super::history::History>().expect("in the table");
            history.read(cx).commit_count(ws.active_session().id)
        })
    };
    assert_eq!(commits(cx), 1, "the one commit on main");

    cx.update(|cx| crate::settings::switch_feature(cx, FeatureId::History, false));
    let (tabs, state) = read_cx(cx, &window, |ws, cx| (ws.panel_tab_list(cx), ws.menu_state(cx)));
    assert!(!tabs.contains(&TabId::History) && tabs.contains(&TabId::Changes), "{tabs:?}");
    assert!(state.hidden.contains(&ActionId::ShowHistory), "its menu line is hidden");
    assert!(!state.hidden.contains(&ActionId::ShowChanges), "Changes' is not");
    assert!(!state.hidden.contains(&ActionId::NewTerminalInWorktree), "nor the worktree's");

    // Its key does nothing, and showing the tab shows Changes.
    drive(cx, &window, |ws, window, cx| ws.focus_terminal(window, cx));
    draw(cx, &window);
    with_window(cx, &window, |window, cx| window.dispatch_action(Box::new(ShowHistory), cx));
    cx.run_until_parked();
    assert_ne!(read(cx, &window, |ws| ws.right_tab), TabId::History);
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::History, cx));
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Changes);
    // The palette's commits have nowhere to go.
    let second = read_cx(cx, &window, |ws, cx| {
        let history = ws.feature::<super::history::History>().expect("in the table");
        history.read(cx).first_commit(ws.active_session().id).expect("a commit")
    });
    drive(cx, &window, |ws, _, cx| super::history::api::reveal_commit(ws, root.clone(), second.clone(), cx));
    cx.run_until_parked();
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Changes);

    // Changes and its compare picker, and the worktree dialog, are as they were.
    draw(cx, &window);
    assert_eq!(read(cx, &window, changed_paths), Some(vec!["a.txt".to_string(), "b.txt".to_string()]));
    drive(cx, &window, toggle_compare);
    cx.run_until_parked();
    assert!(read_cx(cx, &window, |ws, cx| changes(ws).read(cx).picker().is_open()));
    drive(cx, &window, |ws, window, cx| ws.new_terminal_in_worktree(window, cx));
    assert!(read(cx, &window, |ws| ws.worktree_ask.is_some()));

    // On again: the log is there as it was left, and the tab and the key are back.
    cx.update(|cx| crate::settings::switch_feature(cx, FeatureId::History, true));
    assert_eq!(commits(cx), 1);
    let (tabs, state) = read_cx(cx, &window, |ws, cx| (ws.panel_tab_list(cx), ws.menu_state(cx)));
    assert!(tabs.contains(&TabId::History) && !state.hidden.contains(&ActionId::ShowHistory));
}
