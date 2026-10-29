//! Terminals and the splits they are shown in, in a real window (GPUI's test platform; see
//! `test_harness` for how each test runs in a child process with scratch data).
use gpui_kit::{Focusable as _, TestAppContext, VisualTestContext};
use wormhole_core::storage::store::{SessionRecord, SplitRecord, WindowRecord};
use wormhole_core::view::panes::{Axis, DropError, Preset, TabPanes, Zone};
use wormhole_core::view::sidebar::{Bracket, Drop, Landing, Part};

use crate::app_state::OpenWindow;
use crate::terminal_view::{Status, TerminalEvent};
use crate::window::test_harness::{
    draw, drive, hook, isolate_in, press, read, start, start_with, with_window,
};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// The selected terminal's panes in reading order, its focused pane and its zoomed one. A terminal
/// that is in no split is its own single pane.
fn panes(cx: &mut TestAppContext, window: &OpenWindow) -> (Vec<i64>, i64, Option<i64>) {
    read(cx, window, |ws| match ws.active_split() {
        Some(split) => (split.layout.panes(), split.layout.focused(), split.layout.zoomed()),
        None => (vec![ws.active_session().id], ws.active_session().id, None),
    })
}

/// The ids of the terminals as the window keeps them, in order.
/// The terminals' ids top to bottom as the sidebar lists them.
fn ids(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<i64> {
    cx.update(|cx| {
        window
            .workspace
            .read_with(cx, |ws, cx| ws.session_order(cx).into_iter().map(|ix| ws.sessions[ix].id).collect())
    })
    .expect("the window is open")
}

/// The id of the selected terminal.
fn selected(cx: &mut TestAppContext, window: &OpenWindow) -> i64 {
    read(cx, window, |ws| ws.active_session().id)
}

/// The sidebar's list as it is built now.
fn listing(cx: &mut TestAppContext, window: &OpenWindow) -> wormhole_core::view::sidebar::Listing {
    cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.session_listing(cx))).expect("open")
}

/// Whether the last frame drew pane `id`'s header (its name).
fn has_header(drawn: &mut VisualTestContext, id: i64) -> bool {
    // A selector is a `&'static str`; a test may leak its few.
    let name: &'static str = Box::leak(format!("pane-name-{id}").into_boxed_str());
    drawn.debug_bounds(name).is_some()
}

/// Press a key as the keyboard would, after a frame (the keys go where the last frame's focus is).
fn key(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
    draw(cx, window);
    press(cx, window, keys);
}

/// A window of terminals 1 and 2 saved as one split with `layout`.
fn split_of_two(layout: &TabPanes) -> WindowRecord {
    let member = |id| SessionRecord { split_id: Some(100), ..SessionRecord::new(id, None, None) };
    WindowRecord {
        id: 1,
        sessions: vec![member(1), member(2)],
        splits: vec![SplitRecord { id: 100, layout: Some(layout.to_json()) }],
        ..Default::default()
    }
}

#[gpui_kit::test]
fn the_keys_split_zoom_and_close_panes_and_the_focus_follows(cx: &mut TestAppContext) {
    if isolate("the_keys_split_zoom_and_close_panes_and_the_focus_follows") {
        return;
    }
    let window = start(cx, 1);
    key(cx, &window, "cmd-d");
    let (order, focused, _) = panes(cx, &window);
    assert_eq!(order.len(), 2, "⌘D: a pane to the right");
    assert_eq!(focused, order[1], "the new pane has the keyboard");
    assert_eq!(ids(cx, &window).len(), 2, "the new pane is a terminal of its own");
    assert_eq!(selected(cx, &window), focused, "and its row is the selected one");
    key(cx, &window, "shift-cmd-d");
    let (order, focused, _) = panes(cx, &window);
    assert_eq!((order.len(), focused), (3, order[2]), "⇧⌘D: a pane below the focused one");
    assert_eq!(ids(cx, &window).len(), 3);
    // The focused pane is the one the window's terminal is, and it has the keyboard.
    let focused_terminal = read(cx, &window, |ws| ws.active_terminal());
    draw(cx, &window);
    cx.update(|cx| {
        window
            .handle
            .update(cx, |_, win, cx| {
                assert!(focused_terminal.focus_handle(cx).is_focused(win), "the new pane")
            })
            .expect("the window is open")
    });
    // Every pane wears a header once the terminal is in a split.
    let mut drawn = VisualTestContext::from_window(window.handle, cx);
    for id in &order {
        assert!(has_header(&mut drawn, *id), "pane {id} has a header");
    }

    // ⌥⌘↑ from the bottom right pane goes to the one above it (⌘D's), ⌥⌘← to the first; the
    // selected row goes with the keyboard.
    key(cx, &window, "alt-cmd-up");
    assert_eq!((panes(cx, &window).1, selected(cx, &window)), (order[1], order[1]));
    key(cx, &window, "alt-cmd-left");
    assert_eq!((panes(cx, &window).1, selected(cx, &window)), (order[0], order[0]));
    key(cx, &window, "alt-cmd-left");
    assert_eq!(panes(cx, &window).1, order[0], "at the split's edge the keys stay");

    // ⇧⌘↩ zooms the focused pane; the others' headers go, the focused one's stays.
    key(cx, &window, "shift-cmd-enter");
    assert_eq!(panes(cx, &window).2, Some(order[0]));
    draw(cx, &window);
    let mut drawn = VisualTestContext::from_window(window.handle, cx);
    assert!(has_header(&mut drawn, order[0]));
    assert!(!has_header(&mut drawn, order[1]), "hidden while zoomed");
    key(cx, &window, "shift-cmd-enter");
    assert_eq!(panes(cx, &window).2, None);

    // ⌘W closes the selected terminal, the focused pane's; the keyboard moves on in reading order.
    key(cx, &window, "cmd-w");
    let (left, focused, _) = panes(cx, &window);
    assert_eq!(left, [order[1], order[2]]);
    assert_eq!((focused, selected(cx, &window)), (order[1], order[1]));
    assert_eq!(ids(cx, &window).len(), 2);
    // With one left the split is gone: a plain row without headers.
    key(cx, &window, "cmd-w");
    assert_eq!(panes(cx, &window).0, [order[2]]);
    assert_eq!(read(cx, &window, |ws| (ws.active_split().is_none(), ws.splits.len())), (true, 0));
    draw(cx, &window);
    let mut drawn = VisualTestContext::from_window(window.handle, cx);
    assert!(!has_header(&mut drawn, order[2]));
}

#[gpui_kit::test]
fn a_layout_arranges_the_split_and_sends_the_rest_out_of_it(cx: &mut TestAppContext) {
    if isolate("a_layout_arranges_the_split_and_sends_the_rest_out_of_it") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    drive(cx, &window, |ws, window, cx| ws.apply_layout(Preset::Grid, window, cx));
    let (order, focused, _) = panes(cx, &window);
    assert_eq!(order.len(), 4, "three new terminals fill the grid");
    assert_eq!(focused, 1, "the terminal the layout started from keeps the keyboard");
    read(cx, &window, |ws| {
        let split = ws.active_split().expect("a split");
        assert_eq!(split.layout.preset(), Some(Preset::Grid));
        assert_eq!(ws.sessions.len(), 4);
        assert!(ws.sessions.iter().all(|session| session.split == Some(split.id)));
    });
    // Down to a single pane: the other three leave the split and stand alone, in their order,
    // still running (the same terminals).
    let terminals: Vec<_> =
        read(cx, &window, |ws| order.iter().map(|id| ws.pane_terminal(*id).expect("a terminal")).collect());
    drive(cx, &window, |ws, window, cx| ws.apply_layout(Preset::Single, window, cx));
    read(cx, &window, |ws| {
        assert!(ws.active_split().is_none() && ws.splits.is_empty(), "no split left");
        assert_eq!(ws.sessions.len(), 4);
        assert_eq!(ws.active_session().id, 1);
        for (id, terminal) in order.iter().zip(&terminals) {
            let ix = ws.find_session(*id).expect("still a row");
            assert_eq!(ws.sessions[ix].split, None);
            assert_eq!(ws.sessions[ix].terminal(), terminal, "the terminal stayed, it didn't restart");
        }
    });
}

#[gpui_kit::test]
fn a_pane_leaves_its_split_with_its_terminal(cx: &mut TestAppContext) {
    if isolate("a_pane_leaves_its_split_with_its_terminal") {
        return;
    }
    let window = start(cx, 1);
    key(cx, &window, "cmd-d");
    let (order, _, _) = panes(cx, &window);
    let moving = read(cx, &window, |ws| ws.pane_terminal(order[0]).expect("a terminal"));
    // Move Pane Out of Split: the two become plain rows, the selection stays where it was.
    key(cx, &window, "alt-cmd-left");
    assert_eq!(selected(cx, &window), order[0]);
    drive(cx, &window, |ws, _, cx| assert!(ws.leave_split(ws.active, cx)));
    read(cx, &window, |ws| {
        assert!(ws.splits.is_empty(), "a split of one is none");
        assert!(ws.sessions.iter().all(|session| session.split.is_none()));
        assert_eq!(ws.active_session().id, order[0]);
        assert_eq!(ws.active_terminal(), moving, "the terminal went on running");
        assert_eq!(ws.sessions.len(), 2);
    });
    // A terminal in no split has nothing to leave.
    drive(cx, &window, |ws, _, cx| assert!(!ws.leave_split(ws.active, cx)));
}

#[gpui_kit::test]
fn a_split_is_saved_with_its_layout_and_comes_back(cx: &mut TestAppContext) {
    if isolate("a_split_is_saved_with_its_layout_and_comes_back") {
        return;
    }
    let window = start(cx, 1);
    key(cx, &window, "cmd-d");
    key(cx, &window, "shift-cmd-enter");
    let (order, focused, zoomed) = panes(cx, &window);
    let record = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.window_record(cx))).expect("open");
    let [split] = record.splits.as_slice() else { panic!("one split: {:?}", record.splits) };
    let saved =
        TabPanes::from_json(split.layout.as_deref().expect("a split saves its layout")).expect("parses");
    assert_eq!((saved.panes(), saved.focused(), saved.zoomed()), (order.clone(), focused, zoomed));
    // Both terminals name it, next to each other; the one that was never split names none.
    assert_eq!(record.sessions.iter().map(|s| s.split_id).collect::<Vec<_>>(), [Some(split.id); 2]);
    assert_eq!(record.sessions.iter().map(|s| s.id).collect::<Vec<_>>(), order);
    drive(cx, &window, |ws, window, cx| ws.new_session(window, cx));
    let record = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.window_record(cx))).expect("open");
    assert_eq!(record.sessions[2].split_id, None);
    assert_eq!(record.active, 2);
}

#[gpui_kit::test]
fn a_saved_split_opens_split_and_a_hook_finds_its_pane(cx: &mut TestAppContext) {
    if isolate("a_saved_split_opens_split_and_a_hook_finds_its_pane") {
        return;
    }
    let mut layout = TabPanes::single(1);
    assert!(layout.split(Axis::Row, 2) && layout.focus(1));
    let window = start_with(cx, split_of_two(&layout));
    assert_eq!(panes(cx, &window), (vec![1, 2], 1, None));
    read(cx, &window, |ws| {
        assert_eq!((ws.sessions.len(), ws.active), (2, 0));
        assert!(ws.on_screen(0) && ws.on_screen(1), "both panes are on screen with the selected one");
    });
    // The hook names the terminal, which needs input once it asks; the other row stays quiet.
    hook(cx, &window, 2, "Notification", Some("Allow Bash?"));
    let statuses: Vec<Status> = cx
        .update(|cx| {
            window.workspace.read_with(cx, |ws, cx| ws.sessions.iter().map(|s| s.status(cx)).collect())
        })
        .expect("the window is open");
    assert_eq!(statuses, [Status::Idle, Status::Attention], "each row shows its own status");
    assert_eq!(panes(cx, &window).1, 1, "the keyboard stays where it was");
}

/// A row of a saved split is selected: its split is on screen with its pane focused, and the other
/// pane is on screen beside it; selecting a terminal outside shows it alone.
#[gpui_kit::test]
fn selecting_a_row_shows_its_split_with_that_pane_focused(cx: &mut TestAppContext) {
    if isolate("selecting_a_row_shows_its_split_with_that_pane_focused") {
        return;
    }
    let mut layout = TabPanes::single(1);
    assert!(layout.split(Axis::Row, 2) && layout.focus(1));
    let mut record = split_of_two(&layout);
    record.sessions.push(SessionRecord::new(3, None, None));
    let window = start_with(cx, record);
    draw(cx, &window);
    let rows = listing(cx, &window);
    assert_eq!(rows.marks(0).bracket, Some(Bracket::Start));
    assert_eq!(rows.marks(1).bracket, Some(Bracket::End));
    assert!(rows.marks(1).on_screen && !rows.marks(0).on_screen, "beside the selected one");
    assert_eq!(rows.marks(2).members, 0);
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    assert_eq!((panes(cx, &window).1, selected(cx, &window)), (2, 2), "its pane is focused");
    drive(cx, &window, |ws, window, cx| ws.activate(2, window, cx));
    read(cx, &window, |ws| {
        assert!(ws.active_split().is_none());
        assert!(!ws.on_screen(0) && !ws.on_screen(1) && ws.on_screen(2));
        assert_eq!(ws.splits[0].layout.focused(), 2, "the split remembers its focused pane");
    });
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    assert_eq!(panes(cx, &window), (vec![1, 2], 1, None));
}

/// Zoomed, only the focused pane is on screen and so the only one that counts as seen.
#[gpui_kit::test]
fn a_zoomed_split_shows_and_counts_only_its_focused_pane(cx: &mut TestAppContext) {
    if isolate("a_zoomed_split_shows_and_counts_only_its_focused_pane") {
        return;
    }
    let mut layout = TabPanes::single(1);
    assert!(layout.split(Axis::Row, 2) && layout.focus(1) && layout.toggle_zoom());
    let window = start_with(cx, split_of_two(&layout));
    read(cx, &window, |ws| assert!(ws.on_screen(0) && !ws.on_screen(1)));
    // Unzoomed, the other pane is on screen again.
    drive(cx, &window, |ws, _, cx| ws.toggle_zoom(cx));
    read(cx, &window, |ws| assert!(ws.on_screen(0) && ws.on_screen(1)));
}

#[gpui_kit::test]
fn a_terminal_joins_a_split_on_a_side_of_a_pane_or_is_refused(cx: &mut TestAppContext) {
    if isolate("a_terminal_joins_a_split_on_a_side_of_a_pane_or_is_refused") {
        return;
    }
    let window = start(cx, 3);
    // Terminal 3 joins terminal 1, which stood alone: a split of the two, 3 on the left.
    drive(cx, &window, |ws, window, cx| {
        assert_eq!(ws.join_split(2, 1, Zone::Left, window, cx), Ok(()));
    });
    assert_eq!(panes(cx, &window), (vec![3, 1], 3, None));
    assert_eq!(selected(cx, &window), 3, "the joined row is the selected one");
    assert_eq!(ids(cx, &window), [1, 3, 2], "its members are next to each other");
    // It can't join its own split again, nor a terminal that isn't there.
    drive(cx, &window, |ws, window, cx| {
        assert_eq!(ws.join_split(2, 1, Zone::Right, window, cx), Err(DropError::Unknown));
        assert_eq!(ws.join_split(0, 9, Zone::Right, window, cx), Err(DropError::Unknown));
    });
    // Terminal 2 joins the split below terminal 1.
    drive(cx, &window, |ws, window, cx| {
        let two = ws.find_session(2).expect("a row");
        assert_eq!(ws.join_split(two, 1, Zone::Bottom, window, cx), Ok(()));
    });
    let (order, focused, _) = panes(cx, &window);
    assert_eq!((order.len(), focused), (3, 2));
    assert_eq!(read(cx, &window, |ws| ws.splits.len()), 1, "one split of three");
    // Leaving one by one: the last two stay a split, the last one alone dissolves it.
    drive(cx, &window, |ws, _, cx| assert!(ws.leave_split(ws.find_session(2).expect("a row"), cx)));
    assert_eq!(read(cx, &window, |ws| ws.splits.len()), 1);
    drive(cx, &window, |ws, _, cx| assert!(ws.leave_split(ws.find_session(3).expect("a row"), cx)));
    read(cx, &window, |ws| assert!(ws.splits.is_empty() && ws.sessions.iter().all(|s| s.split.is_none())));
}

#[gpui_kit::test]
fn a_drop_on_a_row_joins_reorders_or_leaves(cx: &mut TestAppContext) {
    if isolate("a_drop_on_a_row_joins_reorders_or_leaves") {
        return;
    }
    let window = start(cx, 4);
    // The middle of a row joins the two in a split.
    let done = drive(cx, &window, |ws, window, cx| ws.row_drop(4, 1, Part::Middle, window, cx));
    assert!(matches!(done, Ok(Some(Drop::Join { target: 0 }))), "{done:?}");
    assert_eq!(panes(cx, &window), (vec![1, 4], 4, None));
    assert_eq!(ids(cx, &window), [1, 4, 2, 3], "the member sits right after its split");
    // The bottom of a row of another split lands outside it; the top of one leaves.
    let done = drive(cx, &window, |ws, window, cx| ws.row_drop(2, 3, Part::Bottom, window, cx));
    assert!(matches!(done, Ok(Some(Drop::Move { leave_split: false, .. }))), "{done:?}");
    assert_eq!(ids(cx, &window), [1, 4, 3, 2]);
    let done = drive(cx, &window, |ws, window, cx| ws.row_drop(4, 2, Part::Top, window, cx));
    assert!(
        matches!(done, Ok(Some(Drop::Move { leave_split: true, landing: Landing::Top { .. }, .. }))),
        "{done:?}"
    );
    read(cx, &window, |ws| assert!(ws.splits.is_empty(), "the split of two is gone with one leaving"));
    // Dropping a row on itself changes nothing.
    let done = drive(cx, &window, |ws, window, cx| ws.row_drop(3, 3, Part::Middle, window, cx));
    assert_eq!(done, Ok(None));
}

#[gpui_kit::test]
fn show_beside_offers_the_terminals_that_are_not_in_the_split_yet(cx: &mut TestAppContext) {
    if isolate("show_beside_offers_the_terminals_that_are_not_in_the_split_yet") {
        return;
    }
    let window = start(cx, 3);
    // Room for three columns.
    cx.simulate_window_resize(window.handle, gpui_kit::size(gpui_kit::px(2000.0), gpui_kit::px(900.0)));
    draw(cx, &window);
    let choices = |cx: &mut TestAppContext, id: i64| -> Vec<i64> {
        drive(cx, &window, |ws, _, cx| {
            ws.beside_choices(id, cx).into_iter().map(|(other, _)| other).collect()
        })
    };
    assert_eq!(choices(cx, 1), [2, 3], "every other terminal, in the list's order");
    // Picking 2 from terminal 1's menu makes the two a split, 2 to the right of 1.
    drive(cx, &window, |ws, window, cx| ws.show_beside(2, 1, window, cx));
    assert_eq!(panes(cx, &window), (vec![1, 2], 2, None));
    assert_eq!(choices(cx, 1), [3], "2 is beside it already");
    assert_eq!(choices(cx, 2), [3]);
    assert_eq!(choices(cx, 3), [1, 2], "a terminal on its own can show beside either of them");
    // Another terminal can be shown beside one that is in a split already.
    drive(cx, &window, |ws, window, cx| ws.show_beside(3, 2, window, cx));
    assert_eq!(panes(cx, &window).0.len(), 3);
    assert!(choices(cx, 1).is_empty(), "nobody is left to show beside it");
}

#[gpui_kit::test]
fn closing_a_terminal_of_a_split_keeps_the_rest_of_it(cx: &mut TestAppContext) {
    if isolate("closing_a_terminal_of_a_split_keeps_the_rest_of_it") {
        return;
    }
    let mut layout = TabPanes::single(1);
    assert!(layout.split(Axis::Row, 2) && layout.split(Axis::Row, 3) && layout.focus(2));
    let member = |id| SessionRecord { split_id: Some(100), ..SessionRecord::new(id, None, None) };
    let record = WindowRecord {
        id: 1,
        active: 1,
        sessions: vec![member(1), member(2), member(3)],
        splits: vec![SplitRecord { id: 100, layout: Some(layout.to_json()) }],
        ..Default::default()
    };
    let window = start_with(cx, record);
    assert_eq!(selected(cx, &window), 2);
    // Closing the selected middle pane: the one after it takes the selection, the split stays.
    drive(cx, &window, |ws, window, cx| ws.close_session(1, window, cx));
    assert_eq!(panes(cx, &window), (vec![1, 3], 3, None));
    assert_eq!(selected(cx, &window), 3);
    // Closing one that isn't selected leaves the selection alone, and the split dissolves.
    drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
    read(cx, &window, |ws| {
        assert!(ws.splits.is_empty());
        assert_eq!((ws.sessions.len(), ws.active_session().id, ws.sessions[0].split), (1, 3, None));
    });
}

/// A right-click on the pane that does not have the keyboard: the terminal asks the window to select it
/// before its menu is built, so Split Right splits the pane that was clicked, not the one focused before.
#[gpui_kit::test]
fn the_context_menus_split_acts_on_the_pane_that_was_clicked(cx: &mut TestAppContext) {
    if isolate("the_context_menus_split_acts_on_the_pane_that_was_clicked") {
        return;
    }
    let mut layout = TabPanes::single(1);
    assert!(layout.split(Axis::Row, 2) && layout.focus(1));
    let window = start_with(cx, split_of_two(&layout));
    cx.simulate_window_resize(window.handle, gpui_kit::size(gpui_kit::px(1500.0), gpui_kit::px(900.0)));
    for _ in 0..3 {
        draw(cx, &window);
        cx.run_until_parked();
    }
    assert_eq!(panes(cx, &window).1, 1, "pane 1 has the keyboard");
    let clicked = read(cx, &window, |ws| ws.pane_terminal(2).expect("pane 2"));
    cx.update(|cx| clicked.update(cx, |_, cx| cx.emit(TerminalEvent::Activate)));
    cx.run_until_parked();
    assert_eq!((panes(cx, &window).1, selected(cx, &window)), (2, 2), "the clicked pane is the selected one");
    draw(cx, &window);
    with_window(cx, &window, |window, cx| window.dispatch_action(Box::new(crate::actions::SplitRight), cx));
    cx.run_until_parked();
    let (order, ..) = panes(cx, &window);
    assert_eq!(order.len(), 3, "{order:?}");
    assert_eq!((order[0], order[1]), (1, 2), "the new pane is beside pane 2: {order:?}");
}
