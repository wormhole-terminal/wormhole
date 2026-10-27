//! Where a new terminal in a group starts, in a real window: the group's folder, else where the group
//! was last used, and the question about a folder a group without one is asked (answered by hand here:
//! a test window has no native sheet, so the + never blocks on one).
use std::path::PathBuf;

use gpui_kit::TestAppContext;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_core::view::panes::Zone;
use wormhole_core::view::sidebar::group_folder::real_folder;
use wormhole_core::view::sidebar::{Landing, Part};

use crate::app_state::OpenWindow;
use crate::window::group_folder::FolderAnswer;
use crate::window::test_harness::{drive, read, read_cx, sidebar_of, start_with};

fn isolate(name: &str) -> bool {
    crate::window::test_harness::isolate_in(module_path!(), name)
}

/// Make `names` as folders in the test's scratch folder (inside its home, so a folder is never the
/// home folder or above it), and a window with one terminal in each, ids 1.. in that order.
fn window_in(cx: &mut TestAppContext, names: &[&str]) -> (OpenWindow, Vec<PathBuf>) {
    let base = std::env::current_dir().expect("the scratch folder").join("home").join("work");
    let mut folders: Vec<PathBuf> = Vec::new();
    for name in names {
        let folder = base.join(name);
        std::fs::create_dir_all(&folder).expect("a folder");
        // As the window knows it: a temp dir is behind a link on macOS.
        folders.push(real_folder(&folder).expect("a real folder"));
    }
    let sessions = (1i64..)
        .zip(&folders)
        .map(|(id, folder)| SessionRecord::new(id, None, Some(folder.to_string_lossy().into_owned())))
        .collect();
    let window = start_with(cx, WindowRecord { id: 1, sessions, ..Default::default() });
    (window, folders)
}

/// The folder of the newest terminal.
fn newest_folder(cx: &mut TestAppContext, window: &OpenWindow) -> Option<PathBuf> {
    read_cx(cx, window, |ws, cx| ws.sessions.last().and_then(|s| s.terminal().read(cx).cwd.clone()))
}

/// The id of the newest terminal.
fn newest_id(cx: &mut TestAppContext, window: &OpenWindow) -> i64 {
    read(cx, window, |ws| ws.sessions.last().map(|s| s.id).expect("a terminal"))
}

fn select(cx: &mut TestAppContext, window: &OpenWindow, id: i64) {
    drive(cx, window, |ws, window, cx| {
        let ix = ws.find_session(id).expect("a row");
        ws.activate(ix, window, cx);
    });
}

/// Terminal `id` is now in `folder`, as its shell would report with a prompt that says where it is.
fn cd(cx: &mut TestAppContext, window: &OpenWindow, id: i64, folder: &std::path::Path) {
    let folder = folder.to_path_buf();
    drive(cx, window, |ws, _, cx| {
        let ix = ws.find_session(id).expect("a row");
        ws.sessions[ix].terminal().update(cx, |terminal, cx| {
            terminal.cwd = Some(folder);
            cx.notify();
        });
    });
}

fn group_of(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Option<u32> {
    sidebar_of(cx, window, |sidebar| sidebar.view().groups.group_of(id))
}

#[gpui_kit::test]
fn the_plus_on_a_group_with_a_folder_opens_there(cx: &mut TestAppContext) {
    if isolate("the_plus_on_a_group_with_a_folder_opens_there") {
        return;
    }
    let (window, folders) = window_in(cx, &["a", "b", "own"]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1, 2], cx);
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[2].to_str(), cx));
        group
    });
    select(cx, &window, 3);
    drive(cx, &window, |ws, window, cx| ws.new_session_in_group(group, window, cx));
    assert_eq!(newest_folder(cx, &window), Some(folders[2].clone()));
    let newest = newest_id(cx, &window);
    assert_eq!(group_of(cx, &window, newest), Some(group), "and it joins the group");
}

#[gpui_kit::test]
fn a_group_without_a_folder_opens_where_it_was_last_used_not_where_the_selection_is(cx: &mut TestAppContext) {
    if isolate("a_group_without_a_folder_opens_where_it_was_last_used_not_where_the_selection_is") {
        return;
    }
    let (window, folders) = window_in(cx, &["a", "b", "elsewhere"]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1, 2], cx);
        // It said no to a folder, so the + never asks.
        ws.edit_sidebar_view(cx, |view| view.groups.decline_folder(group));
        group
    });
    select(cx, &window, 1);
    select(cx, &window, 3);
    drive(cx, &window, |ws, window, cx| ws.new_session_in_group(group, window, cx));
    assert_eq!(newest_folder(cx, &window), Some(folders[0].clone()), "terminal 1 was the group's last");
    // The new terminal is the one in use now; the group goes on from there.
    select(cx, &window, 2);
    select(cx, &window, 3);
    drive(cx, &window, |ws, window, cx| ws.new_session_in_group(group, window, cx));
    assert_eq!(newest_folder(cx, &window), Some(folders[1].clone()));
}

#[gpui_kit::test]
fn a_selection_made_by_closing_a_terminal_is_the_one_the_group_was_last_on(cx: &mut TestAppContext) {
    if isolate("a_selection_made_by_closing_a_terminal_is_the_one_the_group_was_last_on") {
        return;
    }
    let (window, folders) = window_in(cx, &["a", "b", "c", "elsewhere"]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1, 2, 3], cx);
        ws.edit_sidebar_view(cx, |view| view.groups.decline_folder(group));
        group
    });
    // Terminal 2 is selected and closed: the selection passes to its neighbour without `select`.
    select(cx, &window, 2);
    drive(cx, &window, |ws, window, cx| {
        let ix = ws.find_session(2).expect("a row");
        ws.close_session(ix, window, cx);
    });
    select(cx, &window, 4);
    drive(cx, &window, |ws, window, cx| ws.new_session_in_group(group, window, cx));
    let heir = newest_folder(cx, &window);
    assert!(heir == Some(folders[0].clone()) || heir == Some(folders[2].clone()), "{heir:?}");
    // And it is the one the selection was on, which the list was told, not merely the newest member.
    let last = sidebar_of(cx, &window, |sidebar| sidebar.last_selected(Some(group)));
    assert!(last.is_some_and(|id| id != 2), "{last:?}");
}

#[gpui_kit::test]
fn without_a_sheet_the_plus_opens_at_once_and_remembers_nothing(cx: &mut TestAppContext) {
    if isolate("without_a_sheet_the_plus_opens_at_once_and_remembers_nothing") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "app/src"]);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1, 2], cx));
    let asked = read_cx(cx, &window, |ws, cx| ws.group_folder_question(group, cx));
    assert_eq!(asked, Some(folders[0].clone()), "the folder that holds them both is on offer");
    select(cx, &window, 1);
    drive(cx, &window, |ws, window, cx| ws.new_session_in_group(group, window, cx));
    assert_eq!(newest_folder(cx, &window), Some(folders[0].clone()));
    sidebar_of(cx, &window, |sidebar| {
        let found = sidebar.view().groups.get(group).expect("still there");
        assert!(found.folder.is_none() && !found.folder_declined, "not asked, so not answered");
    });
}

#[gpui_kit::test]
fn not_now_is_remembered_and_the_terminal_opens_where_the_group_was_last_used(cx: &mut TestAppContext) {
    if isolate("not_now_is_remembered_and_the_terminal_opens_where_the_group_was_last_used") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "app/src"]);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1, 2], cx));
    select(cx, &window, 2);
    drive(cx, &window, |ws, window, cx| {
        ws.answer_group_folder(group, "Group 1", &folders[0], FolderAnswer::NotNow, window, cx)
    });
    assert_eq!(newest_folder(cx, &window), Some(folders[1].clone()));
    let asked = read_cx(cx, &window, |ws, cx| ws.group_folder_question(group, cx));
    assert_eq!(asked, None, "never asked again");
    sidebar_of(cx, &window, |sidebar| {
        let found = sidebar.view().groups.get(group).expect("still there");
        assert!(found.folder.is_none() && found.folder_declined);
    });
}

#[gpui_kit::test]
fn an_answer_for_a_group_that_went_opens_a_plain_terminal_and_changes_nothing(cx: &mut TestAppContext) {
    if isolate("an_answer_for_a_group_that_went_opens_a_plain_terminal_and_changes_nothing") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "other"]);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1], cx));
    drive(cx, &window, |ws, _, cx| ws.with_sidebar(cx, |sidebar, cx| sidebar.ungroup(group, cx)));
    // A new group may even take the id: it is another group by its name.
    let again = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2], cx));
    drive(cx, &window, |ws, window, cx| {
        ws.answer_group_folder(group, "Gone", &folders[0], FolderAnswer::NotNow, window, cx);
        ws.answer_group_folder(
            again,
            "Gone",
            &folders[0],
            FolderAnswer::Use { only_in_folder: true },
            window,
            cx,
        )
    });
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 4, "a terminal for each +");
    let (first, second) = (newest_id(cx, &window) - 1, newest_id(cx, &window));
    assert_eq!(
        (group_of(cx, &window, first), group_of(cx, &window, second)),
        (None, None),
        "both stand loose"
    );
    sidebar_of(cx, &window, |sidebar| {
        let found = sidebar.view().groups.get(again).expect("there");
        assert!(found.folder.is_none() && !found.folder_declined && !found.only_in_folder);
    });
}

#[gpui_kit::test]
fn use_folder_sets_it_and_with_the_box_keeps_only_the_terminals_in_it(cx: &mut TestAppContext) {
    if isolate("use_folder_sets_it_and_with_the_box_keeps_only_the_terminals_in_it") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "app/src", "other"]);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1, 2, 3], cx));
    drive(cx, &window, |ws, window, cx| {
        ws.answer_group_folder(
            group,
            "Group 1",
            &folders[0],
            FolderAnswer::Use { only_in_folder: true },
            window,
            cx,
        )
    });
    assert_eq!(newest_folder(cx, &window), Some(folders[0].clone()), "the new terminal starts in it");
    let newest = newest_id(cx, &window);
    assert_eq!(group_of(cx, &window, newest), Some(group));
    assert_eq!(group_of(cx, &window, 3), None, "terminal 3 is in another folder, so it left");
    assert_eq!(group_of(cx, &window, 1), Some(group));
    assert_eq!(group_of(cx, &window, 2), Some(group), "a subfolder stays");
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 4, "nothing closed");
    sidebar_of(cx, &window, |sidebar| {
        let found = sidebar.view().groups.get(group).expect("still there");
        assert_eq!(found.folder.as_deref(), folders[0].to_str());
        assert!(found.pinned && found.only_in_folder && !found.folder_declined);
    });
}

#[gpui_kit::test]
fn use_folder_without_the_box_keeps_everyone_until_the_item_is_switched_on(cx: &mut TestAppContext) {
    if isolate("use_folder_without_the_box_keeps_everyone_until_the_item_is_switched_on") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "other"]);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1, 2], cx));
    drive(cx, &window, |ws, window, cx| {
        ws.answer_group_folder(
            group,
            "Group 1",
            &folders[0],
            FolderAnswer::Use { only_in_folder: false },
            window,
            cx,
        )
    });
    assert_eq!(group_of(cx, &window, 2), Some(group), "the box was not ticked");
    drive(cx, &window, |ws, _, cx| ws.set_only_in_folder(group, true, cx));
    assert_eq!(group_of(cx, &window, 2), None, "switched on, it applies at once");
    assert_eq!(group_of(cx, &window, 1), Some(group));
}

/// Terminals 1 and 2 in one split (1 in `app`, 2 in `other`), 3 in `app`, and 4 and 5 in one split in
/// folders outside it; all in one group that keeps only `app`'s terminals.
fn fenced_group(cx: &mut TestAppContext) -> (OpenWindow, Vec<PathBuf>, u32) {
    let (window, folders) = window_in(cx, &["app", "other", "app/src", "else", "else/two"]);
    let group = drive(cx, &window, |ws, window, cx| {
        let group = ws.group_terminals(&[1, 2, 3, 4, 5], cx);
        let first = ws.find_session(2).expect("a row");
        ws.join_split(first, 1, Zone::Right, window, cx).expect("room for two");
        let other = ws.find_session(5).expect("a row");
        ws.join_split(other, 4, Zone::Right, window, cx).expect("room for two");
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
        group
    });
    (window, folders, group)
}

#[gpui_kit::test]
fn a_split_with_a_pane_in_the_folder_stays_and_one_with_none_leaves_whole(cx: &mut TestAppContext) {
    if isolate("a_split_with_a_pane_in_the_folder_stays_and_one_with_none_leaves_whole") {
        return;
    }
    let (window, _, group) = fenced_group(cx);
    drive(cx, &window, |ws, _, cx| ws.set_only_in_folder(group, true, cx));
    assert_eq!(group_of(cx, &window, 1), Some(group));
    assert_eq!(group_of(cx, &window, 2), Some(group), "its split has a pane inside, so it stays whole");
    assert_eq!(group_of(cx, &window, 3), Some(group));
    assert_eq!(group_of(cx, &window, 4), None, "no pane of this split is inside");
    assert_eq!(group_of(cx, &window, 5), None, "so it left whole");
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 5, "nothing closed");
}

#[gpui_kit::test]
fn a_terminal_that_goes_to_another_folder_leaves_and_one_that_comes_in_is_not_added(cx: &mut TestAppContext) {
    if isolate("a_terminal_that_goes_to_another_folder_leaves_and_one_that_comes_in_is_not_added") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "app/src", "other"]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1, 2], cx);
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
        ws.set_only_in_folder(group, true, cx);
        group
    });
    cd(cx, &window, 2, &folders[1]);
    assert_eq!(group_of(cx, &window, 2), Some(group), "deeper in the folder is still in it");
    cd(cx, &window, 2, &folders[2]);
    assert_eq!(group_of(cx, &window, 2), None, "out of the folder, it leaves");
    cd(cx, &window, 2, &folders[0]);
    assert_eq!(group_of(cx, &window, 2), None, "and coming back doesn't add it again");
    assert_eq!(group_of(cx, &window, 1), Some(group));
    // With the item off, nothing leaves.
    drive(cx, &window, |ws, _, cx| ws.set_only_in_folder(group, false, cx));
    cd(cx, &window, 1, &folders[2]);
    assert_eq!(group_of(cx, &window, 1), Some(group));
}

#[gpui_kit::test]
fn a_terminal_from_outside_cannot_be_put_in_a_group_that_keeps_only_its_folder(cx: &mut TestAppContext) {
    if isolate("a_terminal_from_outside_cannot_be_put_in_a_group_that_keeps_only_its_folder") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "other", "app/src"]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1], cx);
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
        ws.set_only_in_folder(group, true, cx);
        group
    });
    // The menu's way, and a drop on the group's head: refused for the outsider, taken for the insider.
    let put = |cx: &mut TestAppContext, id: i64| {
        drive(cx, &window, |ws, _, cx| {
            ws.with_sidebar(cx, |sidebar, cx| sidebar.move_to_group(id, group, cx))
        })
    };
    assert!(!put(cx, 2), "terminal 2 is in another folder");
    assert_eq!(group_of(cx, &window, 2), None);
    let plan = |cx: &mut TestAppContext, dragged: i64| {
        drive(cx, &window, |ws, _, cx| {
            let input = ws.sidebar_input(cx);
            let sidebar = ws.sidebar.read(cx);
            let listing = sidebar.listing_of(&input, cx);
            let over = crate::sidebar::terminal_drag::Over::Head {
                group,
                part: wormhole_core::view::sidebar::Part::Middle,
            };
            sidebar.terminal_drop_plan(&input, &listing, dragged, over)
        })
    };
    assert!(plan(cx, 2).is_none(), "no landing for it");
    assert!(
        matches!(
            plan(cx, 3),
            Some(wormhole_core::view::sidebar::Drop::Move { landing: Landing::Group { .. }, .. })
        ),
        "a terminal in the folder lands"
    );
    assert!(put(cx, 3));
    assert_eq!(group_of(cx, &window, 3), Some(group));
}

#[gpui_kit::test]
fn clearing_the_folder_turns_the_item_off_and_it_needs_a_folder_to_be_on(cx: &mut TestAppContext) {
    if isolate("clearing_the_folder_turns_the_item_off_and_it_needs_a_folder_to_be_on") {
        return;
    }
    let (window, folders) = window_in(cx, &["app"]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1], cx);
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
        ws.set_only_in_folder(group, true, cx);
        group
    });
    let on = |cx: &mut TestAppContext| {
        sidebar_of(cx, &window, |sidebar| sidebar.view().groups.get(group).is_some_and(|g| g.only_in_folder))
    };
    assert!(on(cx));
    drive(cx, &window, |ws, _, cx| {
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, None, cx))
    });
    assert!(!on(cx), "no folder, nothing to keep terminals in");
    drive(cx, &window, |ws, _, cx| ws.set_only_in_folder(group, true, cx));
    assert!(!on(cx), "and it can't be turned on without one");
}

/// A folder with a `.git` folder, and a linked worktree of it beside, as git writes them (what the list reads
/// to tell a worktree: its `.git` file names `<repo>/.git/worktrees/<name>`).
fn repo_with_worktree(base: &std::path::Path) -> (PathBuf, PathBuf) {
    let (repo, worktree) = (base.join("repo"), base.join("repo-feature"));
    std::fs::create_dir_all(repo.join(".git/worktrees/feature")).expect("repo");
    std::fs::create_dir_all(repo.join("src")).expect("src");
    std::fs::create_dir_all(worktree.join("src")).expect("worktree");
    let pointer = format!("gitdir: {}\n", repo.join(".git/worktrees/feature").display());
    std::fs::write(worktree.join(".git"), pointer).expect("pointer");
    (real_folder(&repo).expect("real"), real_folder(&worktree).expect("real"))
}

#[gpui_kit::test]
fn a_worktree_of_the_groups_repository_stays_in_a_fenced_group_and_other_outsiders_open_loose(
    cx: &mut TestAppContext,
) {
    if isolate("a_worktree_of_the_groups_repository_stays_in_a_fenced_group_and_other_outsiders_open_loose") {
        return;
    }
    let (window, folders) = window_in(cx, &["wt-base", "elsewhere"]);
    let (repo, worktree) = repo_with_worktree(&folders[0]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1], cx);
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, repo.to_str(), cx));
        ws.set_only_in_folder(group, true, cx);
        group
    });
    let open = |cx: &mut TestAppContext, cwd: &std::path::Path| {
        let cwd = cwd.to_string_lossy().into_owned();
        drive(cx, &window, |ws, window, cx| {
            ws.open_session_in(Some(Some(group)), None, Some(cwd), window, cx)
        });
        newest_id(cx, &window)
    };
    // Opened straight into the group (a worktree's terminal, `wh new-terminal --folder`, a Beside target).
    let inside = open(cx, &worktree);
    assert_eq!(group_of(cx, &window, inside), Some(group), "a worktree of the repository is inside");
    cd(cx, &window, inside, &worktree.join("src"));
    assert_eq!(group_of(cx, &window, inside), Some(group), "and so is a folder in it");
    let outside = open(cx, &folders[1]);
    assert_eq!(group_of(cx, &window, outside), None, "an unrelated folder opens loose");
    // A worktree of another repository is not inside.
    let (_, other_worktree) = repo_with_worktree(&folders[1]);
    let stranger = open(cx, &other_worktree);
    assert_eq!(group_of(cx, &window, stranger), None);
}

#[gpui_kit::test]
fn a_new_folder_for_a_fenced_group_applies_at_once(cx: &mut TestAppContext) {
    if isolate("a_new_folder_for_a_fenced_group_applies_at_once") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "other"]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1, 2], cx);
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
        ws.set_only_in_folder(group, true, cx);
        group
    });
    assert_eq!((group_of(cx, &window, 1), group_of(cx, &window, 2)), (Some(group), None));
    drive(cx, &window, |ws, _, cx| {
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[1].to_str(), cx));
    });
    assert_eq!(group_of(cx, &window, 1), None, "outside the new folder, it left");
    sidebar_of(cx, &window, |sidebar| {
        let found = sidebar.view().groups.get(group).expect("still there");
        assert!(found.only_in_folder, "the item stays on");
    });
}

#[gpui_kit::test]
fn a_split_that_loses_its_inside_pane_leaves_and_an_emptied_fenced_group_stays(cx: &mut TestAppContext) {
    if isolate("a_split_that_loses_its_inside_pane_leaves_and_an_emptied_fenced_group_stays") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "other"]);
    let group = drive(cx, &window, |ws, window, cx| {
        let group = ws.group_terminals(&[1, 2], cx);
        let second = ws.find_session(2).expect("a row");
        ws.join_split(second, 1, Zone::Right, window, cx).expect("room for two");
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
        ws.set_only_in_folder(group, true, cx);
        // Not pinned: only the fence keeps it.
        ws.toggle_group_pin(group, cx);
        group
    });
    assert_eq!((group_of(cx, &window, 1), group_of(cx, &window, 2)), (Some(group), Some(group)));
    // The pane in the folder leaves the split: the other one, outside, has nothing inside it any more.
    drive(cx, &window, |ws, _, cx| {
        let first = ws.find_session(1).expect("a row");
        ws.leave_split(first, cx);
    });
    assert_eq!(group_of(cx, &window, 2), None, "its split lost the pane that held it");
    // Close the one that is left: the emptied group stays, as it keeps the folder.
    drive(cx, &window, |ws, window, cx| {
        let first = ws.find_session(1).expect("a row");
        ws.close_session(first, window, cx);
    });
    sidebar_of(cx, &window, |sidebar| assert!(sidebar.view().groups.get(group).is_some()));
}

#[gpui_kit::test]
fn an_outsider_may_join_a_split_that_is_inside_a_fenced_group(cx: &mut TestAppContext) {
    if isolate("an_outsider_may_join_a_split_that_is_inside_a_fenced_group") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "other"]);
    let group = drive(cx, &window, |ws, _, cx| {
        let group = ws.group_terminals(&[1], cx);
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
        ws.set_only_in_folder(group, true, cx);
        group
    });
    let plan = |cx: &mut TestAppContext, over: crate::sidebar::terminal_drag::Over| {
        drive(cx, &window, |ws, _, cx| {
            let input = ws.sidebar_input(cx);
            let sidebar = ws.sidebar.read(cx);
            let listing = sidebar.listing_of(&input, cx);
            sidebar.terminal_drop_plan(&input, &listing, 2, over)
        })
    };
    use crate::sidebar::terminal_drag::Over;
    use wormhole_core::view::sidebar::Part;
    assert!(plan(cx, Over::Head { group, part: Part::Middle }).is_none(), "onto the group: no landing");
    assert!(
        matches!(
            plan(cx, Over::Row { id: 1, part: Part::Middle }),
            Some(wormhole_core::view::sidebar::Drop::Join { .. })
        ),
        "beside a terminal that is in the folder, it makes a split with it"
    );
}

/// Terminals 1 (in `app`) and 2 (in `other`) in one split; 3 in `app`. Group 1 holds the terminals `members`
/// and keeps only `app`'s.
fn split_window(cx: &mut TestAppContext, members: &[i64]) -> (OpenWindow, Vec<PathBuf>, u32) {
    let (window, folders) = window_in(cx, &["app", "other", "app/src", "else"]);
    let members = members.to_vec();
    let group = drive(cx, &window, |ws, window, cx| {
        let group = ws.group_terminals(&members, cx);
        let second = ws.find_session(2).expect("a row");
        ws.join_split(second, 1, Zone::Right, window, cx).expect("room for two");
        ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
        ws.set_only_in_folder(group, true, cx);
        group
    });
    (window, folders, group)
}

fn split_of(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Option<i64> {
    read(cx, window, |ws| ws.find_session(id).and_then(|ix| ws.sessions[ix].split))
}

#[gpui_kit::test]
fn a_pane_dragged_out_of_its_split_into_a_fenced_group_is_judged_alone(cx: &mut TestAppContext) {
    if isolate("a_pane_dragged_out_of_its_split_into_a_fenced_group_is_judged_alone") {
        return;
    }
    // Scenario A: 1 and 2 are a split outside the group, which holds 3; 2, in `other`, leaves the split to
    // land in the group alone: no.
    let (window, _, group) = split_window(cx, &[3]);
    let dropped = drive(cx, &window, |ws, window, cx| ws.row_drop(2, 3, Part::Top, window, cx));
    assert!(matches!(dropped, Ok(None)), "refused: {dropped:?}");
    assert_eq!(group_of(cx, &window, 2), None);
    assert_eq!(split_of(cx, &window, 2), split_of(cx, &window, 1), "its split is as it was");
    assert!(split_of(cx, &window, 1).is_some());
    // The pane that is in the folder may.
    let dropped = drive(cx, &window, |ws, window, cx| ws.row_drop(1, 3, Part::Top, window, cx));
    assert!(matches!(dropped, Ok(Some(_))), "{dropped:?}");
    assert_eq!(group_of(cx, &window, 1), Some(group));
}

#[gpui_kit::test]
fn a_pane_of_the_group_dragged_out_of_its_split_inside_it_is_judged_alone_too(cx: &mut TestAppContext) {
    if isolate("a_pane_of_the_group_dragged_out_of_its_split_inside_it_is_judged_alone_too") {
        return;
    }
    // Scenario B: 2, in `other`, is in the group only for its split with 1; dragging it within the group
    // out of that split would leave it outside alone, so it stays as it is, in its split.
    let (window, _, group) = split_window(cx, &[1, 2, 3]);
    assert_eq!(group_of(cx, &window, 2), Some(group), "held by its split");
    let dropped = drive(cx, &window, |ws, window, cx| ws.row_drop(2, 3, Part::Top, window, cx));
    assert!(matches!(dropped, Ok(None)), "refused: {dropped:?}");
    assert_eq!(group_of(cx, &window, 2), Some(group));
    assert_eq!(split_of(cx, &window, 2), split_of(cx, &window, 1));
}

#[gpui_kit::test]
fn a_split_that_loses_its_inside_pane_by_a_join_or_a_move_leaves_the_group(cx: &mut TestAppContext) {
    if isolate("a_split_that_loses_its_inside_pane_by_a_join_or_a_move_leaves_the_group") {
        return;
    }
    // Joining terminal 1 to another split takes the pane that held 2 inside.
    let (window, _, _) = split_window(cx, &[1, 2]);
    drive(cx, &window, |ws, window, cx| {
        let first = ws.find_session(1).expect("a row");
        ws.join_split(first, 4, Zone::Right, window, cx).expect("room for two");
    });
    // (Terminal 1 goes to the group of the terminal it joined, which is none.)
    assert_eq!(group_of(cx, &window, 2), None, "the pane that held it inside is gone");
    // Taking it out of the window (what a tear-off and a move to another window do) does the same.
    let (window, _, _) = split_window(cx, &[1, 2]);
    drive(cx, &window, |ws, window, cx| {
        let first = ws.find_session(1).expect("a row");
        ws.leave_layout(first, cx);
        let departing = ws.take_out(&[1], window, cx);
        assert!(departing.is_some());
    });
    assert_eq!(group_of(cx, &window, 2), None);
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), 3);
}

#[gpui_kit::test]
fn closing_a_fenced_group_ends_it_unless_it_is_pinned(cx: &mut TestAppContext) {
    if isolate("closing_a_fenced_group_ends_it_unless_it_is_pinned") {
        return;
    }
    let (window, folders) = window_in(cx, &["app", "other"]);
    let (loose, pinned) = drive(cx, &window, |ws, _, cx| {
        let (loose, pinned) = (ws.group_terminals(&[1], cx), ws.group_terminals(&[2], cx));
        for group in [loose, pinned] {
            ws.sidebar.update(cx, |sidebar, cx| sidebar.set_group_folder(group, folders[0].to_str(), cx));
            ws.set_only_in_folder(group, true, cx);
        }
        ws.toggle_group_pin(loose, cx);
        (loose, pinned)
    });
    sidebar_of(cx, &window, |sidebar| {
        assert!(
            sidebar.view().groups.get(loose).is_some_and(|g| !g.pinned),
            "set_folder pinned it, then not"
        );
    });
    drive(cx, &window, |ws, window, cx| ws.close_group(loose, window, cx));
    drive(cx, &window, |ws, window, cx| ws.close_group(pinned, window, cx));
    sidebar_of(cx, &window, |sidebar| {
        assert!(sidebar.view().groups.get(loose).is_none(), "ended with its terminals");
        assert!(sidebar.view().groups.get(pinned).is_some(), "a pinned one stays");
    });
}
