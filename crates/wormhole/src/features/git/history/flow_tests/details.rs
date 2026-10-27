//! What the History tab shows of an opened commit, through the window: its details, a parent followed, the
//! copies, ⌘C, the compact message and what a read that fails does.

use gpui_kit::{Action as _, TestAppContext};
use wormhole_core::config::TabId;

use super::super::super::testing::{git, go, repo, write};
use super::super::HistoryRow;
use super::super::copy::CopyWhat;
use super::super::state::CommitInfo;
use super::{body_expanded, expanded, history, sha_at, toggle_body, toggle_commit, with_state};
use crate::app_state::OpenWindow;
use crate::window::test_harness::{draw, drive, isolate_in, start, with_window};

// ---------------------------------------------------------------- a commit's details

/// A repository whose newest commit is a merge (`Merge branch 'topic'`, tagged `v1`) of a commit with a
/// two-paragraph message and a co-author, and the window's terminal in it on the History tab.
fn merged(cx: &mut TestAppContext, window: &OpenWindow, name: &str) {
    let root = repo(name, &[("a.txt", "one\n")]);
    git(&root, &["checkout", "-q", "-b", "topic"]);
    write(&root, &[("t.txt", "t\n")]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "on topic"]);
    git(&root, &["checkout", "-q", "main"]);
    write(&root, &[("m.txt", "m\nm2\n"), ("a.txt", "one\ntwo\n")]);
    git(&root, &["add", "-A"]);
    git(
        &root,
        &[
            "commit",
            "-q",
            "-m",
            "feat(x): a wide change",
            "-m",
            "The body wraps\nover two lines.",
            "-m",
            "Co-authored-by: Bob <bob@example.com>",
        ],
    );
    git(&root, &["merge", "-q", "--no-ff", "topic", "-m", "Merge branch 'topic'"]);
    git(&root, &["tag", "v1"]);
    drive(cx, window, |ws, _, cx| {
        go(ws, &root, cx);
        ws.show_tab(TabId::History, cx);
    });
    cx.run_until_parked();
}

/// The details of the commit `sha`, once read.
fn details_of(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    sha: &str,
) -> Option<std::rc::Rc<super::super::shown::Shown>> {
    with_state(cx, window, |state, _| state.details_of(sha).cloned())
}

/// The repository the tab shows.
fn root_of(cx: &mut TestAppContext, window: &OpenWindow) -> std::path::PathBuf {
    with_state(cx, window, |state, _| state.root.clone().expect("a root"))
}

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

#[gpui_kit::test]
fn an_opened_commit_shows_its_whole_message_who_wrote_it_and_what_points_at_it(cx: &mut TestAppContext) {
    if isolate("an_opened_commit_shows_its_whole_message_who_wrote_it_and_what_points_at_it") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "details");
    let (merge, wide) = (sha_at(cx, &window, 0), sha_at(cx, &window, 1));
    // Not read until a commit is opened.
    with_state(cx, &window, |state, _| assert!(state.details.is_empty()));
    toggle_commit(cx, &window, &merge);
    let shown = details_of(cx, &window, &merge).expect("read when the commit opened");
    let details = &shown.details;
    assert_eq!(details.subject(), "Merge branch 'topic'");
    assert_eq!(details.parents.len(), 2);
    assert_eq!(shown.parents.len(), 2);
    assert_eq!(shown.parents[0].short.len(), 7, "as git abbreviates them");
    let refs: Vec<_> = details.refs.iter().map(|r| r.name.as_str()).collect();
    assert!(refs.contains(&"HEAD") && refs.contains(&"main") && refs.contains(&"v1"), "{refs:?}");
    // The row under the commit is the details, then its files.
    with_state(cx, &window, |state, _| {
        let rows = state.list.items();
        assert!(matches!(rows[1], HistoryRow::Commit(0)));
        assert!(matches!(rows[2], HistoryRow::Details(_)), "{}", rows.len());
    });
    draw(cx, &window);

    toggle_commit(cx, &window, &wide);
    let shown = details_of(cx, &window, &wide).expect("read");
    let details = &shown.details;
    assert_eq!(details.subject(), "feat(x): a wide change");
    assert_eq!(details.body(), "The body wraps\nover two lines.\n\nCo-authored-by: Bob <bob@example.com>");
    assert_eq!(details.co_authors.len(), 1);
    assert!(!details.committer_differs());
    draw(cx, &window);
    // Opened again, they are read again, so a tag made since shows; the copy kept is on screen until then.
    toggle_commit(cx, &window, &wide);
    git(&root_of(cx, &window), &["tag", "later", &wide]);
    toggle_commit(cx, &window, &wide);
    assert!(details_of(cx, &window, &wide).is_some(), "the old copy stays while the new one is read");
    cx.run_until_parked();
    let again = details_of(cx, &window, &wide).expect("read again");
    assert!(!std::rc::Rc::ptr_eq(&shown, &again));
    assert!(again.details.refs.iter().any(|r| r.name == "later"), "{:?}", again.details.refs);
}

#[gpui_kit::test]
fn the_message_is_compact_until_asked_and_compact_again_on_another_commit(cx: &mut TestAppContext) {
    if isolate("the_message_is_compact_until_asked_and_compact_again_on_another_commit") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "compact");
    let (merge, wide) = (sha_at(cx, &window, 0), sha_at(cx, &window, 1));
    toggle_commit(cx, &window, &wide);
    draw(cx, &window);
    assert!(!body_expanded(cx, &window), "a commit opens compact");
    toggle_body(cx, &window);
    draw(cx, &window);
    assert!(body_expanded(cx, &window));
    // The full message is still what Copy Message copies, compact or not.
    toggle_body(cx, &window);
    assert!(!body_expanded(cx, &window));
    toggle_body(cx, &window);
    // Another commit starts compact again, and so does this one when it is opened again.
    toggle_commit(cx, &window, &merge);
    assert!(!body_expanded(cx, &window));
    toggle_commit(cx, &window, &wide);
    assert!(!body_expanded(cx, &window));
    // ↩ on the details row, which the keys can be on, opens their menu: "Show more" among the copies.
    draw(cx, &window);
    let row = with_state(cx, &window, |state, _| {
        state.list.items().iter().position(|r| matches!(r, HistoryRow::Details(_))).expect("details row")
    });
    drive(cx, &window, |ws, window, cx| {
        ws.areas_mut().panel_at = Some((TabId::History, row));
        assert!(ws.panel_key(&gpui_kit::Keystroke::parse("enter").expect("a key"), window, cx));
    });
    let menu = crate::platform::record::open().expect("↩ opened the menu of the details");
    let outline = menu.outline();
    assert_eq!(outline[0], "- Show more", "{outline:?}");
    assert!(outline.contains(&"- Copy Message".to_string()), "{outline:?}");
    let (_, callbacks) = crate::platform::record::pick("Show more").expect("the item is in the menu");
    with_window(cx, &window, |window, cx| callbacks.iter().for_each(|run| run(window, cx)));
    cx.run_until_parked();
    assert!(body_expanded(cx, &window));
}

#[gpui_kit::test]
fn a_parent_in_the_log_opens_from_the_details(cx: &mut TestAppContext) {
    if isolate("a_parent_in_the_log_opens_from_the_details") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "parents");
    let (merge, first_parent) = (sha_at(cx, &window, 0), sha_at(cx, &window, 1));
    toggle_commit(cx, &window, &merge);
    drive(cx, &window, |ws, _, cx| {
        let session = ws.active_session().id;
        history(ws).update(cx, |history, cx| history.reveal_parent(session, first_parent.clone(), cx));
    });
    cx.run_until_parked();
    draw(cx, &window);
    assert_eq!(expanded(cx, &window), Some(sha_at(cx, &window, 1)));
    // The pending scroll was used by the draw.
    with_state(cx, &window, |state, _| assert!(state.reveal.is_none()));
}

#[gpui_kit::test]
fn every_copy_of_a_commit_reaches_the_clipboard(cx: &mut TestAppContext) {
    if isolate("every_copy_of_a_commit_reaches_the_clipboard") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "copies");
    let sha = sha_at(cx, &window, 1);
    let copy = |cx: &mut TestAppContext, what| {
        drive(cx, &window, |ws, window, cx| {
            let session = ws.active_session().id;
            history(ws).update(cx, |history, cx| history.copy_commit(session, &sha, what, window, cx));
        });
        cx.run_until_parked();
        cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default()
    };
    assert_eq!(copy(cx, CopyWhat::Hash), sha);
    assert_eq!(copy(cx, CopyWhat::ShortHash), sha[..7]);
    assert_eq!(copy(cx, CopyWhat::Subject), "feat(x): a wide change");
    // Never opened: the message is read first, and kept for the next copy.
    with_state(cx, &window, |state, _| assert!(state.details_of(&sha).is_none()));
    assert_eq!(
        copy(cx, CopyWhat::Message),
        "feat(x): a wide change\n\nThe body wraps\nover two lines.\n\nCo-authored-by: Bob <bob@example.com>"
    );
    with_state(cx, &window, |state, _| assert!(state.details_of(&sha).is_some()));
    assert_eq!(copy(cx, CopyWhat::Author), "Test <test@example.com>");
}

#[gpui_kit::test]
fn command_c_on_a_commit_copies_its_hash_and_on_a_file_goes_on(cx: &mut TestAppContext) {
    if isolate("command_c_on_a_commit_copies_its_hash_and_on_a_file_goes_on") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "command-c");
    let sha = sha_at(cx, &window, 1);
    toggle_commit(cx, &window, &sha);
    draw(cx, &window);
    let (commit_row, file_row) = with_state(cx, &window, |state, _| {
        let rows = state.list.items();
        let commit = rows.iter().position(|r| matches!(r, HistoryRow::Commit(1))).expect("the commit");
        let file = rows.iter().position(|r| matches!(r, HistoryRow::File(_))).expect("a file");
        (commit, file)
    });
    let pressed = drive(cx, &window, |ws, window, cx| {
        ws.areas_mut().panel_at = Some((TabId::History, file_row));
        let on_file = ws.panel_copy(window, cx);
        ws.areas_mut().panel_at = Some((TabId::History, commit_row));
        (on_file, ws.panel_copy(window, cx))
    });
    cx.run_until_parked();
    assert_eq!(pressed, (false, true));
    assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()), Some(sha));
}

#[gpui_kit::test]
fn a_copy_that_waited_is_dropped_when_the_person_went_elsewhere(cx: &mut TestAppContext) {
    if isolate("a_copy_that_waited_is_dropped_when_the_person_went_elsewhere") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "copy-moved-on");
    let (merge, wide) = (sha_at(cx, &window, 0), sha_at(cx, &window, 1));
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("what the person copied".to_string()));
    // Copy Message of a commit never opened needs a read; before it lands the person opens another.
    drive(cx, &window, |ws, window, cx| {
        let session = ws.active_session().id;
        history(ws).update(cx, |history, cx| {
            history.copy_commit(session, &wide, CopyWhat::Message, window, cx);
            history.toggle_commit(session, merge.clone(), cx);
        });
    });
    cx.run_until_parked();
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()).as_deref(),
        Some("what the person copied"),
        "the late answer did not overwrite the clipboard"
    );
    // Asked again and left alone, it is made, and only one read was started for it.
    drive(cx, &window, |ws, window, cx| {
        let session = ws.active_session().id;
        history(ws)
            .update(cx, |history, cx| history.copy_commit(session, &wide, CopyWhat::Message, window, cx));
    });
    cx.run_until_parked();
    let copied = cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default();
    assert!(copied.starts_with("feat(x): a wide change"), "{copied}");
}

/// Edit → Copy (and ⌘C with it) is the text field's `Copy` action. With the keys on a commit in the list it
/// copies the commit's hash; with the terminal focused it is the terminal's, and the list is not asked.
#[gpui_kit::test]
fn the_edit_menus_copy_reaches_the_list_and_leaves_the_terminal_its_own(cx: &mut TestAppContext) {
    use gpui_kit::component::input as text_input;
    use wormhole_core::view::focus::Area;
    if isolate("the_edit_menus_copy_reaches_the_list_and_leaves_the_terminal_its_own") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "edit-copy");
    let sha = sha_at(cx, &window, 1);
    toggle_commit(cx, &window, &sha);
    draw(cx, &window);
    let commit_row = with_state(cx, &window, |state, _| {
        state.list.items().iter().position(|r| matches!(r, HistoryRow::Commit(1))).expect("the commit")
    });
    let copy = |cx: &mut TestAppContext, area: Option<Area>| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".to_string()));
        drive(cx, &window, |ws, window, cx| {
            ws.areas_mut().panel_at = Some((TabId::History, commit_row));
            match area {
                Some(area) => ws.focus_area_of(area, window, cx),
                None => ws.focus_terminal(window, cx),
            }
        });
        draw(cx, &window);
        with_window(cx, &window, |window, cx| window.dispatch_action(text_input::Copy.boxed_clone(), cx));
        cx.run_until_parked();
        cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default()
    };
    assert_eq!(copy(cx, Some(Area::PanelContent)), sha, "the keys are in the list: the commit's hash");
    assert_eq!(copy(cx, None), "sentinel", "the terminal has the keys: the list does not copy");
}

/// A message past what Wormhole reads is cut in the copy too, and the block and the notice say so rather than
/// claim the whole message; an emoji sequence in a subject reads the same in the row, the heading and
/// both copies.
#[gpui_kit::test]
fn a_cut_message_says_so_and_an_emoji_subject_reads_the_same_everywhere(cx: &mut TestAppContext) {
    if isolate("a_cut_message_says_so_and_an_emoji_subject_reads_the_same_everywhere") {
        return;
    }
    let window = start(cx, 1);
    let root = repo("cut-and-emoji", &[("a.txt", "one\n")]);
    let subject = "feat: \u{1F9D1}\u{200D}\u{1F4BB} pair";
    let file = root.join("..").join("cut-and-emoji-message.txt");
    std::fs::write(&file, format!("{subject}\n\n{}\n", "long line of a message\n".repeat(5_000)))
        .expect("write");
    git(&root, &["commit", "-q", "--allow-empty", "-F", file.to_str().expect("a utf-8 path")]);
    drive(cx, &window, |ws, _, cx| {
        go(ws, &root, cx);
        ws.show_tab(TabId::History, cx);
    });
    cx.run_until_parked();
    let sha = sha_at(cx, &window, 0);
    assert_eq!(with_state(cx, &window, |state, _| state.commits[0].subject.clone()), subject, "the row");
    toggle_commit(cx, &window, &sha);
    let shown = details_of(cx, &window, &sha).expect("read");
    assert_eq!(shown.subject, subject, "the heading");
    assert!(shown.details.message_cut && shown.message_cut.is_some(), "past what is read");
    assert!(shown.body_cut, "and past what is drawn");
    draw(cx, &window);
    let copy = |cx: &mut TestAppContext, what| {
        drive(cx, &window, |ws, window, cx| {
            let session = ws.active_session().id;
            history(ws).update(cx, |history, cx| history.copy_commit(session, &sha, what, window, cx));
        });
        cx.run_until_parked();
        cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default()
    };
    assert_eq!(copy(cx, CopyWhat::Subject), subject);
    let message = copy(cx, CopyWhat::Message);
    assert!(
        message.starts_with(subject) && message.len() <= wormhole_core::git::MESSAGE_CAP,
        "{}",
        message.len()
    );
}

/// A copy waiting for a read that the log's change made out of date asks again, and is made.
#[gpui_kit::test]
fn a_copy_waiting_for_details_is_asked_again_when_the_log_changes(cx: &mut TestAppContext) {
    if isolate("a_copy_waiting_for_details_is_asked_again_when_the_log_changes") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "copy-asks-again");
    let wide = sha_at(cx, &window, 1);
    drive(cx, &window, |ws, window, cx| {
        let session = ws.active_session().id;
        history(ws).update(cx, |history, cx| {
            history.copy_commit(session, &wide, CopyWhat::Message, window, cx);
            // The log changes before the read lands: the read in flight is dropped and asked again.
            let state = history.of.get_mut(&session).expect("a state");
            let again = state.details_stale();
            let root = state.root.clone().expect("a root");
            assert_eq!(again.len(), 1, "the commit the copy waits for");
            for (sha, request) in again {
                history.fetch_details(session, root.clone(), sha, request, cx);
            }
        });
    });
    cx.run_until_parked();
    let copied = cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default();
    assert!(copied.starts_with("feat(x): a wide change"), "{copied}");
}

#[gpui_kit::test]
fn details_that_cannot_be_read_say_so_and_the_commit_still_opens(cx: &mut TestAppContext) {
    if isolate("details_that_cannot_be_read_say_so_and_the_commit_still_opens") {
        return;
    }
    let window = start(cx, 1);
    merged(cx, &window, "unreadable");
    let sha = sha_at(cx, &window, 1);
    // A hash git does not know: the read fails as it would for a commit gone after a force push.
    drive(cx, &window, |ws, _, cx| {
        let session = ws.active_session().id;
        history(ws).update(cx, |history, cx| {
            let state = history.of.get_mut(&session).expect("a state");
            state.expanded = Some("0".repeat(40));
            let request = state.want_details(&"0".repeat(40)).expect("asked");
            let root = state.root.clone().expect("a root");
            history.fetch_details(session, root, "0".repeat(40), request, cx);
        });
    });
    cx.run_until_parked();
    with_state(cx, &window, |state, _| {
        assert!(matches!(state.details.get(&"0".repeat(40)), Some(CommitInfo::Failed(_))));
    });
    toggle_commit(cx, &window, &sha);
    assert!(details_of(cx, &window, &sha).is_some(), "the next commit opens as usual");
    draw(cx, &window);
}
