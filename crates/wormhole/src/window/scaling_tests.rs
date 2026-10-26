//! A window being scaled, in a real window (see `test_harness` for how each test runs in a child process with
//! scratch data): a live resize tells it at most every 100 ms and once more when it ends, the drawn grid being the
//! program's meanwhile; the chip says the size after a zoom and while the window is dragged and never when
//! it opens; and the Motion feature off and Reduce Motion are cuts. Time is the test clock's
//! (`BackgroundExecutor::now`), which the tests move.
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{Subscription, TestAppContext, px, size};
use wormhole_core::config::features::FeatureFlag;

use crate::app_state::OpenWindow;
use crate::platform::window::test_live;
use crate::terminal_view::TerminalEvent;
use crate::window::test_harness::{draw, drive, isolate_in, read, read_cx, start, with_window};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window with motion on (the harness starts with Reduce Motion), laid out and at rest.
fn open(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 1);
    cx.update(|cx| cx.set_reduce_motion(false));
    cx.simulate_window_resize(window.handle, size(px(1500.0), px(900.0)));
    frames(cx, &window, 3);
    window
}

/// `count` frames, each `step` apart on the test clock.
fn frames_apart(cx: &mut TestAppContext, window: &OpenWindow, count: usize, step: Duration) {
    for _ in 0..count {
        with_window(cx, window, |window, cx| window.simulate_next_frame(cx));
        draw(cx, window);
        cx.executor().advance_clock(step);
        cx.run_until_parked();
    }
}

fn frames(cx: &mut TestAppContext, window: &OpenWindow, count: usize) {
    frames_apart(cx, window, count, Duration::from_millis(1));
}

/// What the PTY has been told, and what the grid draws.
fn told(cx: &mut TestAppContext, window: &OpenWindow) -> (usize, usize) {
    read_cx(cx, window, |ws, cx| ws.sessions[0].terminal().read(cx).told_cells().expect("a PTY"))
}

fn drawn(cx: &mut TestAppContext, window: &OpenWindow) -> (usize, usize) {
    read(cx, window, |ws| ws.sessions[0].pane.grid.size())
}

fn chip(cx: &mut TestAppContext, window: &OpenWindow) -> Option<String> {
    read_cx(cx, window, |ws, cx| ws.sessions[0].terminal().read(cx).size_readout_text())
}

/// How many times the terminal has told the window its grid has another size.
fn count_resizes(cx: &mut TestAppContext, window: &OpenWindow) -> (Rc<Cell<usize>>, Subscription) {
    let terminal = read(cx, window, |ws| ws.sessions[0].terminal().clone());
    let resizes = Rc::new(Cell::new(0));
    let counted = resizes.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(&terminal, move |_, event: &TerminalEvent, _| {
            if *event == TerminalEvent::Resized {
                counted.set(counted.get() + 1);
            }
        })
    });
    (resizes, subscription)
}

fn count_ends(cx: &mut TestAppContext, window: &OpenWindow) -> (Rc<Cell<usize>>, Subscription) {
    let terminal = read(cx, window, |ws| ws.sessions[0].terminal().clone());
    let ends = Rc::new(Cell::new(0));
    let counted = ends.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(&terminal, move |_, event: &TerminalEvent, _| {
            if *event == TerminalEvent::LiveResizeEnded {
                counted.set(counted.get() + 1);
            }
        })
    });
    (ends, subscription)
}

fn zoom(cx: &mut TestAppContext, window: &OpenWindow, step: Option<f32>) {
    drive(cx, window, |ws, _, cx| ws.zoom_font(step, cx));
}

fn resize_window(cx: &mut TestAppContext, window: &OpenWindow, width: f32) {
    cx.simulate_window_resize(window.handle, size(px(width), px(900.0)));
}

fn set_feature(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), on);
        });
    });
}

#[gpui_kit::test]
fn the_motion_feature_off_and_reduce_motion_zoom_at_once_and_the_first_says_the_size(
    cx: &mut TestAppContext,
) {
    if isolate("the_motion_feature_off_and_reduce_motion_zoom_at_once_and_the_first_says_the_size") {
        return;
    }
    let window = open(cx);
    let (resizes, _count) = count_resizes(cx, &window);
    let before = told(cx, &window);

    cx.update(|cx| cx.set_reduce_motion(true));
    zoom(cx, &window, Some(2.0));
    frames(cx, &window, 2);
    let after = told(cx, &window);
    assert!(after.0 < before.0, "Reduce Motion: a cut, the program is told in the first frame");
    assert_eq!(resizes.get(), 1);
    assert_eq!(chip(cx, &window), Some(format!("{} × {}", after.0, after.1)), "and the chip still says it");

    cx.update(|cx| cx.set_reduce_motion(false));
    set_feature(cx, false);
    zoom(cx, &window, None);
    frames(cx, &window, 2);
    assert_eq!(told(cx, &window), before, "without the feature the zoom is as it always was");
    assert_eq!(resizes.get(), 2);
    zoom(cx, &window, Some(1.0));
    frames(cx, &window, 2);
    assert_eq!(resizes.get(), 3);
}

#[gpui_kit::test]
fn a_live_resize_tells_the_program_at_most_every_100_ms_and_once_more_when_it_ends(cx: &mut TestAppContext) {
    if isolate("a_live_resize_tells_the_program_at_most_every_100_ms_and_once_more_when_it_ends") {
        return;
    }
    let window = open(cx);
    let (resizes, _count) = count_resizes(cx, &window);
    let (ends, _ends) = count_ends(cx, &window);
    let start_size = told(cx, &window);

    test_live::set(Some(true));
    resize_window(cx, &window, 1400.0);
    frames(cx, &window, 2);
    let first = told(cx, &window);
    assert!(first.0 < start_size.0, "the first size of a drag is told at once");
    assert_eq!(resizes.get(), 1);

    // A drag: a new width every 16 ms. Held back; the grid drawn is the program's.
    let mut widths = 1400.0;
    for _ in 0..4 {
        widths -= 40.0;
        resize_window(cx, &window, widths);
        frames_apart(cx, &window, 1, Duration::from_millis(16));
        assert_eq!(told(cx, &window), first, "held back at {widths}");
        assert_eq!(drawn(cx, &window), first, "and the grid drawn is the program's");
    }
    assert_eq!(resizes.get(), 1);
    assert_eq!(ends.get(), 0);

    // The pointer rests: the wake tells the size that is there, without another drag.
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.run_until_parked();
    frames(cx, &window, 1);
    let trailing = told(cx, &window);
    assert!(trailing.0 < first.0, "the last size got through: {first:?} then {trailing:?}");
    assert_eq!(resizes.get(), 2);
    assert_eq!(drawn(cx, &window), trailing);

    // More of the drag, then the button is let go before the interval is up: told at once.
    resize_window(cx, &window, 1000.0);
    frames_apart(cx, &window, 1, Duration::from_millis(16));
    assert_eq!(told(cx, &window), trailing, "held back again");
    test_live::set(Some(false));
    frames(cx, &window, 1);
    let last = told(cx, &window);
    assert!(last.0 < trailing.0, "told when the live resize ended: {trailing:?} then {last:?}");
    assert_eq!(resizes.get(), 3);
    assert_eq!(ends.get(), 1, "and the window heard that it ended, once");
    frames(cx, &window, 2);
    assert_eq!(ends.get(), 1);
    test_live::set(None);
}

#[gpui_kit::test]
fn the_chip_says_the_size_while_the_window_is_dragged_never_when_it_resizes_otherwise(
    cx: &mut TestAppContext,
) {
    if isolate("the_chip_says_the_size_while_the_window_is_dragged_never_when_it_resizes_otherwise") {
        return;
    }
    let window = open(cx);
    resize_window(cx, &window, 1300.0);
    frames(cx, &window, 3);
    assert_eq!(chip(cx, &window), None, "a window resized in any other way says nothing");

    test_live::set(Some(true));
    resize_window(cx, &window, 1200.0);
    frames(cx, &window, 3);
    let wanted = chip(cx, &window).expect("a drag says the size");
    let now = told(cx, &window);
    assert_eq!(wanted, format!("{} × {}", now.0, now.1));
    // While the program is held back the chip already says the size the window gives it.
    resize_window(cx, &window, 1100.0);
    frames_apart(cx, &window, 2, Duration::from_millis(16));
    assert_ne!(chip(cx, &window), Some(wanted), "it follows the drag");
    assert_eq!(told(cx, &window), now, "though the program has not been told");
    test_live::set(None);
}

#[gpui_kit::test]
fn under_reduce_motion_the_chip_is_cut_and_a_new_terminal_in_a_zoomed_window_says_nothing(
    cx: &mut TestAppContext,
) {
    if isolate("under_reduce_motion_the_chip_is_cut_and_a_new_terminal_in_a_zoomed_window_says_nothing") {
        return;
    }
    let window = open(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    zoom(cx, &window, Some(1.0));
    frames(cx, &window, 3);
    assert!(chip(cx, &window).is_some());
    cx.executor().advance_clock(Duration::from_millis(701));
    cx.run_until_parked();
    frames(cx, &window, 2);
    assert_eq!(chip(cx, &window), None, "cut at 700 ms, with no fade");

    drive(cx, &window, |ws, window, cx| ws.new_session(window, cx));
    frames(cx, &window, 3);
    let newest = read_cx(cx, &window, |ws, cx| {
        ws.sessions.last().expect("a session").terminal().read(cx).size_readout_text()
    });
    assert_eq!(newest, None, "a terminal that is new to a zoomed window just has the size");
}
