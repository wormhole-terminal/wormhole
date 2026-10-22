//! Groups in a real window: one goes with its last terminal unless it is pinned, closing one closes
//! its terminals, a terminal is renamed in its row, and one moves with its terminals to a window of
//! its own (see `view_tests` for how each test runs in a child process with scratch data).
use gpui_kit::TestAppContext;

use crate::window::view_tests::{draw, drive, read, start};

fn isolate(name: &str) -> bool {
    crate::window::view_tests::isolate_in(module_path!(), name)
}

fn group_names(cx: &mut TestAppContext, window: &crate::app_state::OpenWindow) -> Vec<(String, bool, usize)> {
    read(cx, window, |ws| {
        ws.sidebar.view.groups.iter().map(|g| (g.name.clone(), g.pinned, g.members.len())).collect()
    })
}

#[gpui_kit::test]
fn a_group_goes_with_its_last_terminal_unless_it_is_pinned(cx: &mut TestAppContext) {
    if isolate("a_group_goes_with_its_last_terminal_unless_it_is_pinned") {
        return;
    }
    let window = start(cx, 4);
    let (docs, review) = drive(cx, &window, |ws, _, cx| {
        let docs = ws.group_terminals(&[1], cx);
        let review = ws.group_terminals(&[2, 3], cx);
        ws.toggle_group_pin(docs, cx);
        (docs, review)
    });
    assert_eq!(group_names(cx, &window).len(), 2);
    // The pinned one stays as a line when its only terminal closes; the other goes with its last.
    drive(cx, &window, |ws, window, cx| {
        let one = ws.find_session(1).expect("a row");
        ws.close_session(one, window, cx);
        let two = ws.find_session(2).expect("a row");
        ws.close_session(two, window, cx);
    });
    assert_eq!(group_names(cx, &window).len(), 2, "the second group still has terminal 3");
    drive(cx, &window, |ws, window, cx| {
        let three = ws.find_session(3).expect("a row");
        ws.close_session(three, window, cx);
    });
    read(cx, &window, |ws| {
        assert!(ws.sidebar.view.groups.get(docs).is_some_and(|g| g.pinned && g.members.is_empty()));
        assert!(ws.sidebar.view.groups.get(review).is_none(), "unpinned, it went with its last terminal");
    });
    draw(cx, &window);
    let listed = cx
        .update(|cx| window.workspace.read_with(cx, |ws, cx| ws.session_listing(cx).entries.len()))
        .expect("open");
    assert_eq!(listed, 2, "the pinned empty group's line, and terminal 4");
}

#[gpui_kit::test]
fn closing_a_group_closes_its_terminals(cx: &mut TestAppContext) {
    if isolate("closing_a_group_closes_its_terminals") {
        return;
    }
    let window = start(cx, 4);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2, 3], cx));
    drive(cx, &window, |ws, window, cx| ws.close_group(group, window, cx));
    let ids: Vec<i64> = read(cx, &window, |ws| ws.sessions.iter().map(|s| s.id).collect());
    assert_eq!(ids, [1, 4], "nothing runs in them, so no question");
    assert!(group_names(cx, &window).is_empty());
}

#[gpui_kit::test]
fn a_terminal_is_renamed_in_its_row_and_an_empty_name_gives_it_back(cx: &mut TestAppContext) {
    if isolate("a_terminal_is_renamed_in_its_row_and_an_empty_name_gives_it_back") {
        return;
    }
    let window = start(cx, 2);
    let rename = |cx: &mut TestAppContext, text: &'static str| {
        drive(cx, &window, |ws, window, cx| {
            ws.start_terminal_rename(2, window, cx);
            let input = ws.sidebar.renaming.as_ref().map(|r| r.input.clone()).expect("renaming");
            input.update(cx, |input, cx| input.set_value(text, window, cx));
            ws.finish_terminal_rename(true, window, cx);
        });
        read(cx, &window, |ws| ws.sessions[1].custom_name.clone())
    };
    assert_eq!(rename(cx, "release notes").as_deref(), Some("release notes"));
    assert_eq!(rename(cx, "  ").as_deref(), None, "an empty name is what runs in it again");
}

#[gpui_kit::test]
fn a_group_moves_with_its_terminals_and_their_split_to_a_window_of_its_own(cx: &mut TestAppContext) {
    if isolate("a_group_moves_with_its_terminals_and_their_split_to_a_window_of_its_own") {
        return;
    }
    let window = start(cx, 4);
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2], cx));
    // 2 is in a split with 3, which is in no group: the split goes along.
    drive(cx, &window, |ws, window, cx| {
        let three = ws.find_session(3).expect("a row");
        ws.join_split(three, 2, wormhole_core::view::panes::Zone::Right, window, cx).expect("room");
    });
    drive(cx, &window, |ws, window, cx| ws.move_group_to_window(group, window, cx));
    cx.run_until_parked();
    let windows = cx.update(|cx| crate::app_state::app(cx).windows.len());
    assert_eq!(windows, 2, "a window of its own");
    let here: Vec<i64> = read(cx, &window, |ws| ws.sessions.iter().map(|s| s.id).collect());
    assert_eq!(here, [1, 4], "the group's terminal and its split's other member left");
    assert!(read(cx, &window, |ws| ws.splits.is_empty()), "the split went with them");
}
