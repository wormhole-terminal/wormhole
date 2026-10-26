//! Tearing a terminal off its window in a real window (GPUI's test platform; see `test_harness` for how each
//! test runs in a child process with scratch data): the drag out of the window and what a release does
//! (core says where the pointer lies, `view::tear_off`; the move itself is `terminal_move`). The pointer's
//! place on the screen is said by the test (`platform::pointer::pretend`), never read from the person's mouse.
use gpui_kit::{EntityId, TestAppContext, point, px};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::panes::{Rect, Zone};
use wormhole_core::view::tear_off::{Occupant, Placed};

use super::drag_tests::{drag, open, release, row_bounds};
use super::tear_off::{land_over, pointer_at};
use super::test_harness::{draw, drive, isolate_in, read, start, with_window};
use crate::app_state::{OpenWindow, app};
use crate::platform::pointer;
use crate::settings::update_settings;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn set_tear_off(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        update_settings(cx, |s| _ = s.feature_flags.insert(FeatureFlag::TearOff.key().into(), on))
    });
}

fn ids(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<i64> {
    read(cx, window, |ws| ws.sessions.iter().map(|s| s.id).collect())
}

fn windows(cx: &mut TestAppContext) -> Vec<OpenWindow> {
    cx.update(|cx| app(cx).windows.clone())
}

/// The window that is not `first`.
fn other_than(cx: &mut TestAppContext, first: &OpenWindow) -> OpenWindow {
    windows(cx).into_iter().find(|w| w.id != first.id).expect("another window")
}

/// The entity of terminal `id`'s view in `window`: the program that runs in it.
fn view_of(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> EntityId {
    read(cx, window, |ws| {
        ws.sessions.iter().find(|s| s.id == id).expect("the terminal is here").terminal().entity_id()
    })
}

/// Let go of terminal `id` with the pointer at (`x`, `y`) of the screen, as the sidebar's release does.
fn let_go(cx: &mut TestAppContext, window: &OpenWindow, id: i64, x: f32, y: f32) {
    drive(cx, window, |ws, window, cx| ws.tear_off_at(id, x, y, window, cx));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_terminal_let_go_over_the_desktop_gets_a_window_of_its_own_and_keeps_running(cx: &mut TestAppContext) {
    if isolate("a_terminal_let_go_over_the_desktop_gets_a_window_of_its_own_and_keeps_running") {
        return;
    }
    let window = start(cx, 3);
    drive(cx, &window, |ws, _, _| ws.sessions[1].custom_name = Some("the build".into()));
    let view = view_of(cx, &window, 2);
    let_go(cx, &window, 2, 600.0, 300.0);

    assert_eq!(windows(cx).len(), 2, "a window of its own");
    assert_eq!(ids(cx, &window), [1, 3], "it left the first window");
    let torn = other_than(cx, &window);
    assert_eq!(ids(cx, &torn), [2], "and is the only terminal of the new one, with its own id");
    assert_eq!(view_of(cx, &torn, 2), view, "the very view, so the same program, not a restart");
    let name = read(cx, &torn, |ws| ws.sessions[0].name());
    assert_eq!(name.as_ref(), "the build", "with what the person had called it");
}

#[gpui_kit::test]
fn the_new_window_opens_under_the_pointer_on_the_display_it_was_let_go_on(cx: &mut TestAppContext) {
    if isolate("the_new_window_opens_under_the_pointer_on_the_display_it_was_let_go_on") {
        return;
    }
    let window = start(cx, 2);
    let_go(cx, &window, 2, 600.0, 300.0);
    let torn = other_than(cx, &window);
    let bounds = with_window(cx, &torn, |window, _| window.bounds());
    // The pointer lies at the anchor of the window, which is kept inside the screen.
    assert_eq!((f32::from(bounds.origin.x), f32::from(bounds.origin.y)), (480.0, 210.0));
    let on = with_window(cx, &torn, |window, cx| window.display(cx).map(|d| u64::from(d.id())));
    let primary = cx.update(|cx| cx.primary_display().map(|d| u64::from(d.id())));
    assert_eq!(on, primary);
}

#[gpui_kit::test]
fn with_the_feature_off_a_release_outside_does_nothing(cx: &mut TestAppContext) {
    if isolate("with_the_feature_off_a_release_outside_does_nothing") {
        return;
    }
    let window = start(cx, 3);
    set_tear_off(cx, false);
    let_go(cx, &window, 2, 600.0, 300.0);
    assert_eq!(windows(cx).len(), 1);
    assert_eq!(ids(cx, &window), [1, 2, 3]);
}

#[gpui_kit::test]
fn the_last_terminal_of_a_window_is_not_torn_off_into_a_window_of_its_own(cx: &mut TestAppContext) {
    if isolate("the_last_terminal_of_a_window_is_not_torn_off_into_a_window_of_its_own") {
        return;
    }
    let window = start(cx, 1);
    let_go(cx, &window, 1, 600.0, 300.0);
    assert_eq!(windows(cx).len(), 1, "that would only move the window");
    assert_eq!(ids(cx, &window), [1]);
}

#[gpui_kit::test]
fn a_terminal_torn_off_a_split_leaves_it_and_its_mate_stays(cx: &mut TestAppContext) {
    if isolate("a_terminal_torn_off_a_split_leaves_it_and_its_mate_stays") {
        return;
    }
    let window = start(cx, 3);
    drive(cx, &window, |ws, window, cx| {
        ws.join_split(1, 1, Zone::Right, window, cx).expect("room for two");
    });
    assert_eq!(read(cx, &window, |ws| ws.sessions.iter().filter(|s| s.split.is_some()).count()), 2);
    let_go(cx, &window, 2, 600.0, 300.0);

    assert_eq!(ids(cx, &window), [1, 3]);
    assert!(read(cx, &window, |ws| ws.sessions.iter().all(|s| s.split.is_none())), "its mate stands alone");
    assert!(read(cx, &window, |ws| ws.splits.is_empty()), "the split dissolved");
    let torn = other_than(cx, &window);
    assert!(read(cx, &torn, |ws| ws.sessions[0].split.is_none() && ws.splits.is_empty()), "it is no split");
}

/// Two windows, the second (made by tearing terminal 3 off the first) lying over the first, which keeps 1 and
/// 2: where each is on the screen, front to back.
fn two_windows(cx: &mut TestAppContext) -> (OpenWindow, OpenWindow, Vec<Placed<i64>>) {
    let window = start(cx, 3);
    let_go(cx, &window, 3, 600.0, 300.0);
    let target = other_than(cx, &window);
    let display = cx.update(|cx| cx.primary_display().map(|d| u64::from(d.id()))).expect("a display");
    let stack = vec![
        Placed {
            occupant: Occupant::Terminals(target.id),
            display,
            bounds: Rect::new(0.0, 0.0, 800.0, 600.0),
        },
        Placed {
            occupant: Occupant::Terminals(window.id),
            display,
            bounds: Rect::new(0.0, 0.0, 1400.0, 900.0),
        },
    ];
    (window, target, stack)
}

fn land(cx: &mut TestAppContext, stack: &[Placed<i64>], source: &OpenWindow, id: i64, at: (f32, f32)) {
    let pointer = cx.update(|cx| pointer_at(at.0, at.1, cx)).expect("a pointer");
    cx.update(|cx| land_over(stack, source.id, id, pointer, cx));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_terminal_let_go_over_another_window_goes_into_it_last_and_selected(cx: &mut TestAppContext) {
    if isolate("a_terminal_let_go_over_another_window_goes_into_it_last_and_selected") {
        return;
    }
    let (window, target, stack) = two_windows(cx);
    let view = view_of(cx, &target, 3);
    let moved = view_of(cx, &window, 2);
    land(cx, &stack, &window, 2, (400.0, 300.0));

    assert_eq!(ids(cx, &window), [1]);
    assert_eq!(ids(cx, &target), [3, 2], "last in its list");
    assert_eq!(read(cx, &target, |ws| ws.active_session().id), 2, "and selected");
    assert_eq!(view_of(cx, &target, 2), moved, "the same view, running on");
    assert_eq!(view_of(cx, &target, 3), view, "and the others undisturbed");
    assert_eq!(windows(cx).len(), 2, "no third window");
}

#[gpui_kit::test]
fn a_release_over_the_window_it_came_from_or_one_that_takes_nothing_changes_nothing(cx: &mut TestAppContext) {
    if isolate("a_release_over_the_window_it_came_from_or_one_that_takes_nothing_changes_nothing") {
        return;
    }
    let (window, target, mut stack) = two_windows(cx);
    // Over its own window (outside the second one).
    land(cx, &stack, &window, 2, (1000.0, 700.0));
    assert_eq!((ids(cx, &window), ids(cx, &target)), (vec![1, 2], vec![3]));
    // Over a window that takes no terminal (Settings, lying over the second window).
    stack.insert(0, Placed { occupant: Occupant::Other, ..stack[0] });
    land(cx, &stack, &window, 2, (400.0, 300.0));
    assert_eq!((ids(cx, &window), ids(cx, &target)), (vec![1, 2], vec![3]));
    assert_eq!(windows(cx).len(), 2);
}

#[gpui_kit::test]
fn the_last_terminal_of_a_window_can_go_into_another_and_the_window_starts_a_new_one(
    cx: &mut TestAppContext,
) {
    if isolate("the_last_terminal_of_a_window_can_go_into_another_and_the_window_starts_a_new_one") {
        return;
    }
    let (window, target, stack) = two_windows(cx);
    // The second window's only terminal goes back into the first, which lies behind it.
    let (first, second) = (stack[1], stack[0]);
    let back = [Placed { bounds: Rect::new(0.0, 0.0, 100.0, 100.0), ..second }, first];
    land(cx, &back, &target, 3, (500.0, 500.0));
    assert_eq!(ids(cx, &window), [1, 2, 3]);
    let fresh = ids(cx, &target);
    assert_eq!(fresh.len(), 1, "a window is never left without a terminal, as when its last one is closed");
    assert!(!fresh.contains(&3));
}

#[gpui_kit::test]
fn a_row_dragged_out_of_the_window_is_followed_by_a_chip_and_let_go_there_it_is_torn_off(
    cx: &mut TestAppContext,
) {
    if isolate("a_row_dragged_out_of_the_window_is_followed_by_a_chip_and_let_go_there_it_is_torn_off") {
        return;
    }
    let window = open(cx, 3, 900.0, 600.0);
    let from = row_bounds(cx, &window, 2).center();
    let outside = point(px(1000.0), px(300.0));
    cx.update(|cx| pointer::pretend(pointer_at(1000.0, 300.0, cx)));
    let all = |cx: &mut TestAppContext| cx.update(|cx| cx.windows().len());

    drag(cx, &window, from, &[from + point(px(30.0), px(0.0)), point(px(500.0), px(300.0))]);
    assert_eq!(all(cx), 1, "inside the window the drag draws its own picture");
    drag_on(cx, &window, outside);
    assert_eq!(all(cx), 2, "outside it the picture is a window of its own");
    drag_on(cx, &window, point(px(500.0), px(300.0)));
    // (The chip is put away, not closed, while the drag is in the window: it would only open again.)
    release(cx, &window, outside);
    assert_eq!(windows(cx).len(), 2, "a window of its own for the terminal");
    assert_eq!(all(cx), 2, "and no chip left");
    assert_eq!(ids(cx, &window), [1, 3]);
    pointer::pretend(None);
}

/// Move on with the button held, a frame after.
fn drag_on(cx: &mut TestAppContext, window: &OpenWindow, to: gpui_kit::Point<gpui_kit::Pixels>) {
    super::drag_tests::pointer(cx, window).simulate_mouse_move(
        to,
        gpui_kit::MouseButton::Left,
        gpui_kit::Modifiers::none(),
    );
    super::test_harness::draw(cx, window);
}

#[gpui_kit::test]
fn a_window_that_stops_being_active_cancels_the_drag_and_puts_the_chip_away(cx: &mut TestAppContext) {
    if isolate("a_window_that_stops_being_active_cancels_the_drag_and_puts_the_chip_away") {
        return;
    }
    let window = open(cx, 3, 900.0, 600.0);
    let from = row_bounds(cx, &window, 2).center();
    cx.update(|cx| pointer::pretend(pointer_at(1000.0, 300.0, cx)));
    drag(cx, &window, from, &[from + point(px(30.0), px(0.0))]);
    drag_on(cx, &window, point(px(1000.0), px(300.0)));
    assert!(cx.update(|cx| cx.has_active_drag()));
    assert_eq!(cx.update(|cx| cx.windows().len()), 2);

    with_window(cx, &window, |window, cx| crate::sidebar::tear_off::cancel(window, cx));
    cx.run_until_parked();
    assert!(!cx.update(|cx| cx.has_active_drag()), "GPUI's drag is dropped");
    assert_eq!(cx.update(|cx| cx.windows().len()), 1, "and the chip is closed");
    pointer::pretend(None);
}

#[gpui_kit::test]
fn the_new_window_grows_from_the_chip_only_where_motion_plays(cx: &mut TestAppContext) {
    if isolate("the_new_window_grows_from_the_chip_only_where_motion_plays") {
        return;
    }
    let window = start(cx, 4);
    // Reduce Motion (the tests' default): the window just appears.
    let_go(cx, &window, 2, 600.0, 300.0);
    let plain = other_than(cx, &window);
    assert!(!cx.update(|cx| super::grow::active(plain.id, cx)), "cut under Reduce Motion");

    cx.update(|cx| cx.set_reduce_motion(false));
    let_go(cx, &window, 3, 600.0, 300.0);
    let grown =
        windows(cx).into_iter().find(|w| w.id != window.id && w.id != plain.id).expect("a third window");
    assert!(cx.update(|cx| super::grow::active(grown.id, cx)), "growing");
    // Drawn while it grows (the oracle of the chrome checks the frame), then at rest again.
    draw(cx, &grown);
    cx.executor().advance_clock(std::time::Duration::from_millis(500));
    cx.run_until_parked();
    assert!(!cx.update(|cx| super::grow::active(grown.id, cx)), "over");
    draw(cx, &grown);
    // Moving it did not start it again, nor resize anything of the other windows.
    assert_eq!(ids(cx, &grown), [3]);
}
