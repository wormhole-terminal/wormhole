//! The stable terminal list in a real window: a new terminal or a split moves no other terminal and
//! changes no other ⌘ number, and the classic settings keep what they did (see `view_tests` for
//! how each test runs in a child process with scratch data).
use std::collections::BTreeMap;

use gpui_kit::TestAppContext;
use wormhole_core::panes::Axis;
use wormhole_core::sidebar::Grouping;

use super::app::OpenWindow;
use super::view_tests::{drive, read, start, use_stable};

fn isolate(name: &str) -> bool {
    super::view_tests::isolate_in(module_path!(), name)
}

/// Each numbered terminal's id and ⌘ number.
fn numbers(cx: &mut TestAppContext, window: &OpenWindow) -> BTreeMap<i64, usize> {
    read(cx, window, |_| ());
    cx.update(|cx| {
        window.workspace.read_with(cx, |ws, cx| {
            ws.session_listing(cx)
                .numbered()
                .into_iter()
                .enumerate()
                .map(|(at, session)| (ws.sessions[session].id, at + 1))
                .collect()
        })
    })
    .expect("the window is open")
}

/// The terminals' ids in the order the window keeps them.
fn kept(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<i64> {
    read(cx, window, |ws| ws.sessions.iter().map(|session| session.id).collect())
}

#[gpui_kit::test]
fn a_new_split_pane_moves_no_terminal_and_no_number_when_stable(cx: &mut TestAppContext) {
    if isolate("a_new_split_pane_moves_no_terminal_and_no_number_when_stable") {
        return;
    }
    let window = start(cx, 3);
    use_stable(cx);
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    let before = numbers(cx, &window);
    drive(cx, &window, |ws, window, cx| ws.split_pane(Axis::Row, window, cx));
    let ids = kept(cx, &window);
    assert_eq!(ids.len(), 4);
    assert_eq!(ids[..3], [1, 2, 3], "the new terminal is last, as it was made");
    let after = numbers(cx, &window);
    assert!(before.iter().all(|(id, n)| after.get(id) == Some(n)), "{before:?} -> {after:?}");
    // The list still shows the split's rows together.
    let order = cx
        .update(|cx| window.workspace.read_with(cx, |ws, cx| ws.session_order(cx)))
        .expect("the window is open");
    assert_eq!(order[..2], [0, 3], "gathered in the list, not moved in the window");
}

#[gpui_kit::test]
fn a_new_split_pane_takes_its_place_beside_the_split_when_classic(cx: &mut TestAppContext) {
    if isolate("a_new_split_pane_takes_its_place_beside_the_split_when_classic") {
        return;
    }
    let window = start(cx, 3);
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    drive(cx, &window, |ws, window, cx| ws.split_pane(Axis::Row, window, cx));
    let ids = kept(cx, &window);
    assert_eq!(ids[0], 1);
    assert_eq!(ids[2..], [2, 3], "the new terminal went in beside its split");
}

#[gpui_kit::test]
fn a_new_terminal_in_a_folder_renumbers_nobody_when_stable(cx: &mut TestAppContext) {
    if isolate("a_new_terminal_in_a_folder_renumbers_nobody_when_stable") {
        return;
    }
    let window = start(cx, 1);
    use_stable(cx);
    drive(cx, &window, |ws, _, cx| ws.set_grouping(Grouping::Folder, cx));
    for folder in ["/wormhole-test/a", "/wormhole-test/b", "/wormhole-test/a"] {
        drive(cx, &window, |ws, window, cx| {
            ws.open_session(None, Some(folder.into()), window, cx);
        });
    }
    let before = numbers(cx, &window);
    assert_eq!(before.len(), 4);
    // Into the first band, above two terminals in the list.
    drive(cx, &window, |ws, window, cx| {
        ws.open_session(None, Some("/wormhole-test/a".into()), window, cx);
    });
    let after = numbers(cx, &window);
    assert!(before.iter().all(|(id, n)| after.get(id) == Some(n)), "{before:?} -> {after:?}");
    assert_eq!(after.len(), 5);
}
