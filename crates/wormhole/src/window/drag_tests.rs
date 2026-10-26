//! Dragging terminals between the sidebar and the panes in a real window (GPUI's test
//! platform; see `test_harness` for how each test runs in a child process with scratch data). The
//! pointer does what a person's does: it goes down on a row or a pane's header, moves with the
//! button held (a frame between, so the overlay over the panes is drawn before the next move
//! reaches it) and lets go. What each gesture does to the panes and the list is core's; these tests
//! check that the gestures reach it.
use gpui_kit::{
    Bounds, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px, size,
};
use wormhole_core::view::panes::Zone;
use wormhole_core::view::sidebar::Entry;

use crate::app_state::OpenWindow;
use crate::sidebar::terminal_drag::{Over, PaneHover};
use crate::window::test_harness::{draw, drive, isolate_in, read, sidebar_of, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window of `sessions` terminals, `width` × `height` points, drawn twice (the first frame
/// measures the area the panes share, the next one is laid out with it).
pub(super) fn open(cx: &mut TestAppContext, sessions: i64, width: f32, height: f32) -> OpenWindow {
    let window = start(cx, sessions);
    cx.simulate_window_resize(window.handle, size(px(width), px(height)));
    draw(cx, &window);
    draw(cx, &window);
    window
}

pub(super) fn pointer(cx: &mut TestAppContext, window: &OpenWindow) -> VisualTestContext {
    VisualTestContext::from_window(window.handle, cx)
}

/// Where terminal `id`'s row is, as the last frame drew it.
pub(super) fn row_bounds(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Bounds<Pixels> {
    cx.update(|cx| {
        window.workspace.read_with(cx, |ws, cx| {
            let at = ws
                .session_listing(cx)
                .entries
                .iter()
                .position(
                    |entry| matches!(entry, Entry::Row { session, .. } if ws.sessions[*session].id == id),
                )
                .expect("the terminal is listed");
            ws.sidebar.read(cx).scroll().bounds_for_item(at).expect("the row was drawn")
        })
    })
    .expect("the window is open")
}

/// Where pane `id` is on screen, as the last frame laid the selected split out.
fn pane_bounds(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Bounds<Pixels> {
    read(cx, window, |ws| {
        let (_, rect) =
            ws.drop_rects().into_iter().find(|(pane, _)| *pane == id).expect("the pane is on screen");
        Bounds::new(
            point(ws.pane_area.origin.x + px(rect.x), ws.pane_area.origin.y + px(rect.y)),
            size(px(rect.w), px(rect.h)),
        )
    })
}

/// Where pane `id`'s header says its name: somewhere to press that is no button.
fn header_point(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Point<Pixels> {
    // A selector is a `&'static str`; a test may leak its few.
    let name: &'static str = Box::leak(format!("pane-name-{id}").into_boxed_str());
    pointer(cx, window).debug_bounds(name).expect("the header is drawn").center()
}

/// Press at `from` and move with the button held to each of `via` in turn, a frame after each.
/// The button stays down.
pub(super) fn drag(cx: &mut TestAppContext, window: &OpenWindow, from: Point<Pixels>, via: &[Point<Pixels>]) {
    let mut pointer = pointer(cx, window);
    pointer.simulate_mouse_move(from, None, Modifiers::none());
    pointer.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for &at in via {
        draw(cx, window);
        pointer.simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
    }
    draw(cx, window);
}

pub(super) fn release(cx: &mut TestAppContext, window: &OpenWindow, at: Point<Pixels>) {
    pointer(cx, window).simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    draw(cx, window);
}

/// What the pointer is over, as the drag in flight knows it.
fn hover(cx: &mut TestAppContext, window: &OpenWindow) -> Option<PaneHover> {
    read(cx, window, |ws| ws.terminal_drag.as_ref().and_then(|drag| drag.pane))
}

/// The selected terminal's panes in reading order, its focused pane and its zoomed one.
fn panes(cx: &mut TestAppContext, window: &OpenWindow) -> (Vec<i64>, i64) {
    read(cx, window, |ws| match ws.active_split() {
        Some(split) => (split.layout.panes(), split.layout.focused()),
        None => (vec![ws.active_session().id], ws.active_session().id),
    })
}

/// The terminals' ids top to bottom as the sidebar lists them.
fn ids(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<i64> {
    cx.update(|cx| {
        window
            .workspace
            .read_with(cx, |ws, cx| ws.session_order(cx).into_iter().map(|ix| ws.sessions[ix].id).collect())
    })
    .expect("the window is open")
}

/// The terminal whose row the pointer is over while something is dragged.
fn onto(cx: &mut TestAppContext, window: &OpenWindow) -> Option<i64> {
    sidebar_of(cx, window, |sidebar| match sidebar.over() {
        Some(Over::Row { id, .. }) => Some(id),
        _ => None,
    })
}

fn in_a_split(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<bool> {
    read(cx, window, |ws| ws.sessions.iter().map(|session| session.split.is_some()).collect())
}

/// Terminals 1 and 2 as a split with terminal 1 selected.
fn two_in_a_split(cx: &mut TestAppContext, window: &OpenWindow) {
    drive(cx, window, |ws, window, cx| {
        ws.join_split(1, 1, Zone::Right, window, cx).expect("room for two");
        ws.activate(0, window, cx);
    });
    draw(cx, window);
    draw(cx, window);
}

#[gpui_kit::test]
fn a_row_dragged_onto_a_pane_edge_joins_the_split_there(cx: &mut TestAppContext) {
    if isolate("a_row_dragged_onto_a_pane_edge_joins_the_split_there") {
        return;
    }
    let window = open(cx, 3, 1800.0, 900.0);
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    draw(cx, &window);
    assert_eq!(hover(cx, &window), None, "nothing is dragged");

    let row = row_bounds(cx, &window, 3).center();
    let pane = pane_bounds(cx, &window, 1);
    let edge = point(pane.right() - px(12.0), pane.center().y);
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), pane.center(), edge]);
    assert_eq!(hover(cx, &window), Some(PaneHover { target: 1, zone: Zone::Right }), "the pointer's side");
    release(cx, &window, edge);
    assert_eq!(panes(cx, &window), (vec![1, 3], 3), "3 is on the right of 1, with the keyboard");
    assert_eq!(ids(cx, &window), [1, 3, 2], "its row comes next to 1's");
    assert_eq!(hover(cx, &window), None, "the drag is over");
}

#[gpui_kit::test]
fn a_row_dragged_onto_the_left_edge_of_a_pane_lands_on_its_left(cx: &mut TestAppContext) {
    if isolate("a_row_dragged_onto_the_left_edge_of_a_pane_lands_on_its_left") {
        return;
    }
    let window = open(cx, 3, 1800.0, 900.0);
    two_in_a_split(cx, &window);
    // 3 goes to the left of pane 2, which is on the right of the split.
    let row = row_bounds(cx, &window, 3).center();
    let pane = pane_bounds(cx, &window, 2);
    let edge = point(pane.left() + px(12.0), pane.center().y);
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), pane.center(), edge]);
    assert_eq!(hover(cx, &window), Some(PaneHover { target: 2, zone: Zone::Left }));
    release(cx, &window, edge);
    assert_eq!(panes(cx, &window), (vec![1, 3, 2], 3));
}

#[gpui_kit::test]
fn a_row_dragged_onto_a_row_makes_a_split_and_between_rows_only_reorders(cx: &mut TestAppContext) {
    if isolate("a_row_dragged_onto_a_row_makes_a_split_and_between_rows_only_reorders") {
        return;
    }
    let window = open(cx, 3, 1800.0, 900.0);
    // The top quarter of terminal 1's row: 3 goes in front of it and nothing is split.
    let row = row_bounds(cx, &window, 3).center();
    let target = row_bounds(cx, &window, 1);
    let top = point(target.center().x, target.top() + target.size.height * 0.1);
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), top]);
    release(cx, &window, top);
    assert_eq!(ids(cx, &window), [3, 1, 2]);
    assert_eq!(in_a_split(cx, &window), [false; 3], "dropped between rows, it only moved");

    // The middle of terminal 1's row: the two are a split.
    let row = row_bounds(cx, &window, 2).center();
    let middle = row_bounds(cx, &window, 1).center();
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), middle]);
    assert_eq!(onto(cx, &window), Some(1));
    release(cx, &window, middle);
    assert_eq!(panes(cx, &window), (vec![1, 2], 2), "2 is shown beside 1");
    assert_eq!(in_a_split(cx, &window), [true, true, false]);
}

#[gpui_kit::test]
fn the_middle_of_a_row_in_another_group_makes_a_split_in_that_group(cx: &mut TestAppContext) {
    if isolate("the_middle_of_a_row_in_another_group_makes_a_split_in_that_group") {
        return;
    }
    // Two terminals in two groups of their own.
    let window = open(cx, 2, 1800.0, 900.0);
    drive(cx, &window, |ws, _, cx| {
        ws.group_terminals(&[1], cx);
        ws.group_terminals(&[2], cx);
    });
    draw(cx, &window);
    let groups = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.session_listing(cx).bands.len()));
    assert_eq!(groups.expect("open"), 2, "one group each");

    let row = row_bounds(cx, &window, 2).center();
    let middle = row_bounds(cx, &window, 1).center();
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), middle]);
    assert_eq!(onto(cx, &window), Some(1));
    release(cx, &window, middle);
    assert_eq!(panes(cx, &window), (vec![1, 2], 2), "shown side by side");
    // The dragged terminal joins the split's group, so the split stands in one card: no chain mark.
    let (same_group, chained) = cx
        .update(|cx| {
            window.workspace.read_with(cx, |ws, cx| {
                let groups = &ws.sidebar_view(cx).groups;
                let same = groups.group_of(1).is_some() && groups.group_of(1) == groups.group_of(2);
                let chained = ws
                    .session_listing(cx)
                    .entries
                    .iter()
                    .filter_map(|entry| match entry {
                        Entry::Row { marks, .. } => Some(marks.chain),
                        Entry::Band { .. } => None,
                    })
                    .collect::<Vec<_>>();
                (same, chained)
            })
        })
        .expect("open");
    assert!(same_group, "the dragged terminal is in the split's group");
    assert_eq!(chained, [false, false], "one card, not a chain");
}

#[gpui_kit::test]
fn a_header_dragged_into_the_sidebar_leaves_the_split(cx: &mut TestAppContext) {
    if isolate("a_header_dragged_into_the_sidebar_leaves_the_split") {
        return;
    }
    let window = open(cx, 3, 1800.0, 900.0);
    two_in_a_split(cx, &window);
    // Pane 2's header onto the bottom of terminal 3's row: it stands alone below it.
    let header = header_point(cx, &window, 2);
    let target = row_bounds(cx, &window, 3);
    let bottom = point(target.center().x, target.bottom() - px(3.0));
    drag(cx, &window, header, &[header + point(px(-40.0), px(10.0)), bottom]);
    assert_eq!(onto(cx, &window), Some(3));
    release(cx, &window, bottom);
    assert_eq!(ids(cx, &window), [1, 3, 2], "it went where the line was");
    assert_eq!(in_a_split(cx, &window), [false; 3], "a split of one is none");
    // Pressing the header gave the pane the keyboard, and it is the one that stands alone now.
    assert_eq!(panes(cx, &window).0, [2]);
}

#[gpui_kit::test]
fn a_header_dropped_under_the_rows_leaves_the_split_where_it_is(cx: &mut TestAppContext) {
    if isolate("a_header_dropped_under_the_rows_leaves_the_split_where_it_is") {
        return;
    }
    let window = open(cx, 3, 1800.0, 900.0);
    two_in_a_split(cx, &window);
    let header = header_point(cx, &window, 2);
    let last = row_bounds(cx, &window, 3);
    // Well under the last row, in the list's own empty space: the bottom of that row.
    let below = point(last.center().x, last.bottom() + px(200.0));
    drag(cx, &window, header, &[header + point(px(-40.0), px(10.0)), below]);
    release(cx, &window, below);
    assert_eq!(ids(cx, &window), [1, 3, 2], "to the end of the list");
    assert_eq!(in_a_split(cx, &window), [false; 3]);
}

/// Terminals 1, 2 and 3 as three columns of one split, terminal 1 selected.
fn three_columns(cx: &mut TestAppContext, window: &OpenWindow) {
    two_in_a_split(cx, window);
    drive(cx, window, |ws, window, cx| {
        ws.join_split(2, 2, Zone::Right, window, cx).expect("room for three");
        ws.activate(0, window, cx);
    });
    draw(cx, window);
    draw(cx, window);
    assert_eq!(panes(cx, window).0, [1, 2, 3]);
}

#[gpui_kit::test]
fn a_header_dropped_on_another_panes_edge_moves_it_there(cx: &mut TestAppContext) {
    if isolate("a_header_dropped_on_another_panes_edge_moves_it_there") {
        return;
    }
    let window = open(cx, 3, 1800.0, 900.0);
    three_columns(cx, &window);
    // Pane 3's header onto the bottom edge of pane 1: 3 is under 1, and 2 stays on the right.
    let header = header_point(cx, &window, 3);
    let pane = pane_bounds(cx, &window, 1);
    let edge = point(pane.center().x, pane.bottom() - px(12.0));
    drag(cx, &window, header, &[header + point(px(-40.0), px(10.0)), pane.center(), edge]);
    assert_eq!(hover(cx, &window), Some(PaneHover { target: 1, zone: Zone::Bottom }));
    release(cx, &window, edge);
    assert_eq!(panes(cx, &window), (vec![1, 2, 3], 3), "the keyboard is in the moved pane");
    let (one, three) = (pane_bounds(cx, &window, 1), pane_bounds(cx, &window, 3));
    assert_eq!(three.origin.x, one.origin.x, "3 is in 1's column");
    assert!(three.origin.y > one.origin.y, "and under it");
    assert!(pane_bounds(cx, &window, 2).origin.x > one.origin.x, "2 is still on the right");

    // Dropped on its own pane, a header changes nothing and shows no zone.
    let own = pane_bounds(cx, &window, 3);
    let header = header_point(cx, &window, 3);
    drag(cx, &window, header, &[header + point(px(-40.0), px(10.0)), own.center()]);
    assert_eq!(hover(cx, &window), None);
    release(cx, &window, own.center());
    assert_eq!(pane_bounds(cx, &window, 3), three);
}

#[gpui_kit::test]
fn the_list_follows_the_layout_when_a_pane_is_moved(cx: &mut TestAppContext) {
    if isolate("the_list_follows_the_layout_when_a_pane_is_moved") {
        return;
    }
    let window = open(cx, 3, 1800.0, 900.0);
    three_columns(cx, &window);
    // Pane 1's header onto the right edge of pane 3: the columns are 2, 3, 1 now, and the rows too.
    let header = header_point(cx, &window, 1);
    let pane = pane_bounds(cx, &window, 3);
    let edge = point(pane.right() - px(12.0), pane.center().y);
    drag(cx, &window, header, &[header + point(px(40.0), px(10.0)), pane.center(), edge]);
    assert_eq!(hover(cx, &window), Some(PaneHover { target: 3, zone: Zone::Right }));
    release(cx, &window, edge);
    assert_eq!(panes(cx, &window), (vec![2, 3, 1], 1));
    assert_eq!(ids(cx, &window), [2, 3, 1], "the sidebar's order follows the layout");
    assert_eq!(read(cx, &window, |ws| ws.active_session().id), 1, "the moved pane's row is selected");
}

#[gpui_kit::test]
fn a_row_dragged_out_of_its_bracket_leaves_the_split(cx: &mut TestAppContext) {
    if isolate("a_row_dragged_out_of_its_bracket_leaves_the_split") {
        return;
    }
    let window = open(cx, 3, 1800.0, 900.0);
    two_in_a_split(cx, &window);
    // Row 2, the second of the bracket, onto the bottom of the plain row 3 under it.
    let row = row_bounds(cx, &window, 2).center();
    let target = row_bounds(cx, &window, 3);
    let bottom = point(target.center().x, target.bottom() - px(3.0));
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), bottom]);
    release(cx, &window, bottom);
    assert_eq!(ids(cx, &window), [1, 3, 2]);
    assert_eq!(in_a_split(cx, &window), [false; 3]);

    // Inside a bracket a row only changes places in the list: the layout stays. Terminals 1 and 3
    // are next to each other now; 3 goes above 1.
    drive(cx, &window, |ws, window, cx| {
        let three = ws.find_session(3).expect("a row");
        ws.join_split(three, 1, Zone::Right, window, cx).expect("room for two");
        ws.activate(0, window, cx);
    });
    draw(cx, &window);
    draw(cx, &window);
    let layout = read(cx, &window, |ws| ws.active_split().map(|split| split.layout.clone()));
    assert_eq!(ids(cx, &window), [1, 3, 2]);
    let row = row_bounds(cx, &window, 3).center();
    let first = row_bounds(cx, &window, 1);
    // The rows must be where they were measured when they are pressed: the test windows hold the list's
    // slide still (`test_harness`), so no row is moving.
    assert!(!sidebar_of(cx, &window, |sidebar| sidebar.rows_moving()), "no row is sliding");
    let above = point(first.center().x, first.top() + px(3.0));
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), above]);
    release(cx, &window, above);
    assert_eq!(ids(cx, &window), [3, 1, 2], "the rows swapped");
    assert_eq!(in_a_split(cx, &window), [true, false, true]);
    assert_eq!(read(cx, &window, |ws| ws.active_split().map(|split| split.layout.clone())), layout);
}

#[gpui_kit::test]
fn a_member_dropped_just_outside_its_bracket_leaves_it_without_moving(cx: &mut TestAppContext) {
    if isolate("a_member_dropped_just_outside_its_bracket_leaves_it_without_moving") {
        return;
    }
    // Terminals 2 and 3 are a split with 1 above it and 4 under it.
    let window = open(cx, 4, 1800.0, 900.0);
    let split_2_and_3 = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, window, cx| {
            ws.join_split(2, 2, Zone::Right, window, cx).expect("room for two");
        });
        draw(cx, &window);
        draw(cx, &window);
        assert_eq!(in_a_split(cx, &window), [false, true, true, false]);
    };

    // The last member, on the top of the row under the bracket: the line there is outside it.
    split_2_and_3(cx);
    let row = row_bounds(cx, &window, 3).center();
    let under = row_bounds(cx, &window, 4);
    let top = point(under.center().x, under.top() + under.size.height * 0.1);
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), top]);
    release(cx, &window, top);
    assert_eq!(in_a_split(cx, &window), [false; 4], "it left");
    assert_eq!(ids(cx, &window), [1, 2, 3, 4], "and every row is where it was");

    // The first member, on the bottom of the row above the bracket.
    split_2_and_3(cx);
    let row = row_bounds(cx, &window, 2).center();
    let above = row_bounds(cx, &window, 1);
    let bottom = point(above.center().x, above.bottom() - px(3.0));
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), bottom]);
    release(cx, &window, bottom);
    assert_eq!(in_a_split(cx, &window), [false; 4]);
    assert_eq!(ids(cx, &window), [1, 2, 3, 4]);
}

#[gpui_kit::test]
fn no_side_shows_where_the_pane_would_be_too_small(cx: &mut TestAppContext) {
    if isolate("no_side_shows_where_the_pane_would_be_too_small") {
        return;
    }
    // Wide, but not tall enough for a pane above another.
    let window = open(cx, 2, 1800.0, 280.0);
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    draw(cx, &window);
    let row = row_bounds(cx, &window, 2).center();
    let pane = pane_bounds(cx, &window, 1);
    let top = point(pane.center().x, pane.top() + px(8.0));
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), pane.center(), top]);
    assert_eq!(hover(cx, &window), None, "no room for a pane above");
    release(cx, &window, top);
    assert_eq!(panes(cx, &window).0, [1], "nothing was dropped");
    // The right edge has room.
    let edge = point(pane.right() - px(12.0), pane.center().y);
    let row = row_bounds(cx, &window, 2).center();
    drag(cx, &window, row, &[row + point(px(30.0), px(0.0)), edge]);
    assert_eq!(hover(cx, &window), Some(PaneHover { target: 1, zone: Zone::Right }));
}
