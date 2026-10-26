//! The list's motion while a terminal is dragged by its row, in a real window (see `test_harness` for how
//! each test runs in a child process with scratch data; `drag_tests` for how the pointer is driven):
//! the rows it passes step aside to open a gap where it would land and it is out of the list while the
//! gap is open, a drop lands there with every row once, a drag that ends elsewhere puts everything
//! back, and the Motion feature off and Reduce Motion on leave the drag as it was. GPUI's springs run
//! on the wall clock, which a test cannot advance, so these check what the list holds (who is pushed,
//! how far, who is lifted) and its timers, which the test clock moves.
use std::time::Duration;

use gpui_kit::{
    Bounds, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px, size,
};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::sidebar::Entry;

use crate::app_state::OpenWindow;
use crate::window::test_harness::{draw, draw_replaying, isolate_in, read_cx, sidebar_of, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window of `sessions` terminals with motion on (the harness starts with Reduce Motion), drawn at rest.
fn open(cx: &mut TestAppContext, sessions: i64) -> OpenWindow {
    let window = start(cx, sessions);
    cx.update(|cx| cx.set_reduce_motion(false));
    cx.simulate_window_resize(window.handle, size(px(1800.0), px(900.0)));
    draw(cx, &window);
    draw(cx, &window);
    window
}

fn pointer(cx: &mut TestAppContext, window: &OpenWindow) -> VisualTestContext {
    VisualTestContext::from_window(window.handle, cx)
}

/// Where terminal `id`'s row is laid out, as the last frame drew it.
fn row_bounds(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Bounds<Pixels> {
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

/// Press at `from` and move with the button held to each of `via`, a frame after each. The button stays down.
fn drag(cx: &mut TestAppContext, window: &OpenWindow, from: Point<Pixels>, via: &[Point<Pixels>]) {
    let mut pointer = pointer(cx, window);
    pointer.simulate_mouse_move(from, None, Modifiers::none());
    pointer.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for &at in via {
        draw(cx, window);
        pointer.simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
    }
    draw(cx, window);
}

fn release(cx: &mut TestAppContext, window: &OpenWindow, at: Point<Pixels>) {
    pointer(cx, window).simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    draw(cx, window);
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

/// Let every timer run out and draw where the list lands.
fn settle(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    draw(cx, window);
    draw(cx, window);
}

fn pushed(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<(i64, f32)> {
    sidebar_of(cx, window, |sidebar| sidebar.pushed_rows())
}

fn lifted(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<i64> {
    sidebar_of(cx, window, |sidebar| sidebar.lifted_rows())
}

fn children(cx: &mut TestAppContext, window: &OpenWindow) -> usize {
    sidebar_of(cx, window, |sidebar| sidebar.scroll().children_count())
}

/// The top quarter of terminal `id`'s row: a drop there lands before it.
fn above(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Point<Pixels> {
    let row = row_bounds(cx, window, id);
    point(row.center().x, row.top() + row.size.height * 0.1)
}

/// The bottom quarter of terminal `id`'s row: a drop there lands after it.
fn below(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Point<Pixels> {
    let row = row_bounds(cx, window, id);
    point(row.center().x, row.bottom() - row.size.height * 0.1)
}

#[gpui_kit::test]
fn the_rows_a_dragged_row_passes_step_aside_and_it_is_out_of_the_list_while_the_gap_is_open(
    cx: &mut TestAppContext,
) {
    if isolate("the_rows_a_dragged_row_passes_step_aside_and_it_is_out_of_the_list_while_the_gap_is_open") {
        return;
    }
    let window = open(cx, 4);
    let (grab, target) = (row_bounds(cx, &window, 4).center(), above(cx, &window, 2));
    let height = f32::from(row_bounds(cx, &window, 4).size.height);
    assert!(pushed(cx, &window).is_empty() && lifted(cx, &window).is_empty(), "at rest nothing is pushed");
    let laid_out = row_bounds(cx, &window, 2);
    // Row 4 held over the top of row 2: the gap opens above row 2, so rows 2 and 3 step down by one row.
    drag(cx, &window, grab, &[grab + point(px(30.0), px(0.0)), target]);
    let by = pushed(cx, &window);
    assert_eq!(by.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [2, 3], "the rows it passes");
    for (id, by) in &by {
        assert!((by - height).abs() < 1.5, "row {id} steps down by the dragged row's height: {by}");
    }
    assert_eq!(lifted(cx, &window), [4], "the dragged row is out of the list while the gap is open");
    // Nothing about the layout moved: the rows are laid out where they were, and drawn once each.
    assert_eq!(row_bounds(cx, &window, 2), laid_out, "row 2 is laid out where it was");
    assert_eq!(ids(cx, &window), [1, 2, 3, 4], "and nothing is dropped yet");

    // Along to the bottom of row 3: the gap moves with it, and row 2 is back where it was.
    let at = below(cx, &window, 3);
    pointer(cx, &window).simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
    draw(cx, &window);
    assert_eq!(
        pushed(cx, &window).iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [] as [i64; 0],
        "no gap: 4 is already below 3"
    );
    // To the top of row 1: every row above the dragged one steps down.
    let at = above(cx, &window, 1);
    pointer(cx, &window).simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
    draw(cx, &window);
    assert_eq!(pushed(cx, &window).iter().map(|(id, _)| *id).collect::<Vec<_>>(), [1, 2, 3]);

    release(cx, &window, at);
    assert_eq!(ids(cx, &window), [4, 1, 2, 3], "it lands where the gap was");
    assert_eq!(children(cx, &window), 5, "every row is drawn once: four, and the space under them");
    settle(cx, &window);
    assert!(!sidebar_of(cx, &window, |s| s.has_gap()), "the gap is gone when everything is at rest");
    assert_eq!(ids(cx, &window), [4, 1, 2, 3]);
    assert_eq!(children(cx, &window), 5);
}

#[gpui_kit::test]
fn a_drag_that_ends_without_a_drop_puts_everything_back(cx: &mut TestAppContext) {
    if isolate("a_drag_that_ends_without_a_drop_puts_everything_back") {
        return;
    }
    let window = open(cx, 4);
    let grab = row_bounds(cx, &window, 4).center();
    let target = above(cx, &window, 1);
    drag(cx, &window, grab, &[grab + point(px(30.0), px(0.0)), target]);
    assert_eq!(pushed(cx, &window).len(), 3, "the gap is open");
    // Let go over nothing that takes it: out of the window's list, at its corner.
    let nowhere = point(px(4.0), px(4.0));
    pointer(&mut *cx, &window).simulate_mouse_move(nowhere, MouseButton::Left, Modifiers::none());
    release(cx, &window, nowhere);
    assert_eq!(ids(cx, &window), [1, 2, 3, 4], "nothing moved");
    assert!(sidebar_of(cx, &window, |s| s.pushed_rows().is_empty()), "every row is told to go back");
    assert!(lifted(cx, &window).is_empty(), "and the dragged row is back in the list");
    assert!(sidebar_of(cx, &window, |s| s.has_gap()), "until they have, the springs are held");
    settle(cx, &window);
    assert!(!sidebar_of(cx, &window, |s| s.has_gap()), "then they are let go");
}

#[gpui_kit::test]
fn the_middle_of_a_row_is_a_split_and_opens_no_gap(cx: &mut TestAppContext) {
    if isolate("the_middle_of_a_row_is_a_split_and_opens_no_gap") {
        return;
    }
    let window = open(cx, 3);
    let grab = row_bounds(cx, &window, 3).center();
    let target = row_bounds(cx, &window, 1).center();
    drag(cx, &window, grab, &[grab + point(px(30.0), px(0.0)), target]);
    assert!(pushed(cx, &window).is_empty(), "a join moves no row");
    assert!(lifted(cx, &window).is_empty(), "and the dragged row stays in the list, dimmed as before");
    release(cx, &window, target);
}

#[gpui_kit::test]
fn with_the_motion_feature_off_a_drag_moves_no_row(cx: &mut TestAppContext) {
    if isolate("with_the_motion_feature_off_a_drag_moves_no_row") {
        return;
    }
    let window = open(cx, 4);
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), false);
        });
    });
    draw(cx, &window);
    let grab = row_bounds(cx, &window, 4).center();
    let target = above(cx, &window, 1);
    drag(cx, &window, grab, &[grab + point(px(30.0), px(0.0)), target]);
    assert!(pushed(cx, &window).is_empty() && lifted(cx, &window).is_empty(), "no gap");
    assert!(!sidebar_of(cx, &window, |s| s.has_gap()), "the list holds nothing of the drag");
    release(cx, &window, target);
    assert_eq!(ids(cx, &window), [4, 1, 2, 3], "the drop does what it always did");
}

#[gpui_kit::test]
fn a_drag_draws_the_list_only_never_the_chrome_beyond_what_a_drag_costs(cx: &mut TestAppContext) {
    if isolate("a_drag_draws_the_list_only_never_the_chrome_beyond_what_a_drag_costs") {
        return;
    }
    let window = open(cx, 4);
    let grab = row_bounds(cx, &window, 4).center();
    let target = above(cx, &window, 1);
    drag(cx, &window, grab, &[grab + point(px(30.0), px(0.0)), target]);
    let regions = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| ws.regions.builds(cx));
    let list = |cx: &mut TestAppContext| sidebar_of(cx, &window, |sidebar| sidebar.builds());
    let (chrome, drawn) = (regions(cx), list(cx));
    // The springs ask for their frames by notifying the list: each draws it and nothing else.
    for _ in 0..5 {
        cx.update(|cx| {
            let sidebar = window.workspace.read_with(cx, |ws, _| ws.sidebar.clone()).expect("open");
            sidebar.update(cx, |_, cx| cx.notify());
        });
        draw_replaying(cx, &window);
    }
    assert_eq!(regions(cx), chrome, "the gap is the list's to draw");
    assert_eq!(list(cx), drawn + 5);
    release(cx, &window, target);
}

fn join_state(cx: &mut TestAppContext, window: &OpenWindow) -> (Option<i64>, Option<i64>, bool, bool) {
    sidebar_of(cx, window, |sidebar| sidebar.join_state())
}

fn in_a_split(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<bool> {
    crate::window::test_harness::read(cx, window, |ws| {
        ws.sessions.iter().map(|s| s.split.is_some()).collect()
    })
}

fn middle(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> Point<Pixels> {
    row_bounds(cx, window, id).center()
}

#[gpui_kit::test]
fn over_the_middle_of_a_row_it_previews_the_split_and_leaving_undoes_it(cx: &mut TestAppContext) {
    if isolate("over_the_middle_of_a_row_it_previews_the_split_and_leaving_undoes_it") {
        return;
    }
    let window = open(cx, 3);
    assert_eq!(join_state(cx, &window), (None, None, false, false));
    let (grab, onto) = (middle(cx, &window, 3), middle(cx, &window, 1));
    drag(cx, &window, grab, &[grab + point(px(30.0), px(0.0)), onto]);
    assert_eq!(
        join_state(cx, &window),
        (Some(1), None, true, false),
        "row 1 takes the preview, the chip snaps"
    );
    // Out to the gap between rows 2 and 3: the preview goes out and the chip comes off.
    let gap = above(cx, &window, 2);
    pointer(cx, &window).simulate_mouse_move(gap, MouseButton::Left, Modifiers::none());
    draw(cx, &window);
    assert_eq!(join_state(cx, &window), (None, Some(1), false, false), "row 1's preview is going out");
    settle(cx, &window);
    assert_eq!(join_state(cx, &window).1, None, "and is gone when it has");
    // Back onto row 1, then onto row 2: each row's preview is its own.
    pointer(cx, &window).simulate_mouse_move(onto, MouseButton::Left, Modifiers::none());
    draw(cx, &window);
    let next = middle(cx, &window, 2);
    pointer(cx, &window).simulate_mouse_move(next, MouseButton::Left, Modifiers::none());
    draw(cx, &window);
    assert_eq!(join_state(cx, &window), (Some(2), Some(1), true, false));
    release(cx, &window, next);
    assert_eq!(join_state(cx, &window).0, None, "the drop ends the target");
}

#[gpui_kit::test]
fn a_drop_on_the_middle_of_a_row_makes_a_split_whose_bracket_draws_and_then_rests(cx: &mut TestAppContext) {
    if isolate("a_drop_on_the_middle_of_a_row_makes_a_split_whose_bracket_draws_and_then_rests") {
        return;
    }
    let window = open(cx, 4);
    let (grab, onto) = (middle(cx, &window, 4), middle(cx, &window, 1));
    drag(cx, &window, grab, &[grab + point(px(30.0), px(0.0)), onto]);
    release(cx, &window, onto);
    assert_eq!(ids(cx, &window), [1, 4, 2, 3], "right after the row it joined, each row once");
    assert_eq!(in_a_split(cx, &window), [true, false, false, true], "1 and 4 are a split");
    let (target, leaving, snapping, drawing) = join_state(cx, &window);
    assert_eq!(
        (target, leaving, snapping, drawing),
        (None, Some(1), false, true),
        "the bracket draws, the tint goes out"
    );
    assert_eq!(children(cx, &window), 5, "no row is drawn twice");
    settle(cx, &window);
    assert_eq!(join_state(cx, &window), (None, None, false, false), "everything is at rest");
    assert!(!sidebar_of(cx, &window, |s| s.motion_busy() || s.has_gap()));
}

#[gpui_kit::test]
fn with_reduce_motion_the_gap_and_the_split_are_cuts(cx: &mut TestAppContext) {
    if isolate("with_reduce_motion_the_gap_and_the_split_are_cuts") {
        return;
    }
    let window = open(cx, 4);
    cx.update(|cx| cx.set_reduce_motion(true));
    draw(cx, &window);
    let grab = middle(cx, &window, 4);
    let gap = above(cx, &window, 2);
    drag(cx, &window, grab, &[grab + point(px(30.0), px(0.0)), gap]);
    assert_eq!(pushed(cx, &window).len(), 2, "the gap is there, with no spring to open it");
    let onto = middle(cx, &window, 1);
    pointer(cx, &window).simulate_mouse_move(onto, MouseButton::Left, Modifiers::none());
    draw(cx, &window);
    release(cx, &window, onto);
    assert_eq!(ids(cx, &window), [1, 4, 2, 3]);
    assert!(!sidebar_of(cx, &window, |s| s.motion_busy()), "no row slides");
    assert!(!join_state(cx, &window).3, "no bracket draws");
}
