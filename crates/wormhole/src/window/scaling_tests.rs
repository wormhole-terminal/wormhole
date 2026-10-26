//! A window being scaled, in a real window (see `test_harness` for how each test runs in a child process with
//! scratch data): the text zoom eases while the terminal keeps its grid and tells the program once, at the
//! end; a live resize tells it at most every 100 ms and once more when it ends, the drawn grid being the
//! program's meanwhile; the chip says the size after a zoom and while the window is dragged and never when
//! it opens; and the Motion feature off and Reduce Motion are cuts. Time is the test clock's
//! (`BackgroundExecutor::now`), which the tests move.
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{Subscription, TestAppContext, VisualTestContext, px, size};
use wormhole_core::config::features::FeatureFlag;

use crate::app_state::OpenWindow;
use crate::platform::live_resize::test_live;
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

fn zoom(cx: &mut TestAppContext, window: &OpenWindow, step: Option<f32>) {
    drive(cx, window, |ws, _, cx| ws.zoom_font(step, cx));
}

fn resize_window(cx: &mut TestAppContext, window: &OpenWindow, width: f32) {
    cx.simulate_window_resize(window.handle, size(px(width), px(900.0)));
}

fn set_feature(cx: &mut TestAppContext, flag: FeatureFlag, on: bool) {
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(flag.key().to_string(), on);
        });
    });
}

#[gpui_kit::test]
fn a_zoom_keeps_the_grid_while_the_text_eases_and_tells_the_program_once_at_the_end(cx: &mut TestAppContext) {
    if isolate("a_zoom_keeps_the_grid_while_the_text_eases_and_tells_the_program_once_at_the_end") {
        return;
    }
    let window = open(cx);
    let (resizes, _count) = count_resizes(cx, &window);
    let before = told(cx, &window);
    assert_eq!(drawn(cx, &window), before);
    assert_eq!(chip(cx, &window), None, "nothing says the size when a window opens");

    zoom(cx, &window, Some(3.0));
    // Halfway through its 120 ms: the text moves, the grid is the program's own and nothing was told.
    frames_apart(cx, &window, 4, Duration::from_millis(15));
    assert_eq!(told(cx, &window), before, "the program is not told while the text moves");
    assert_eq!(drawn(cx, &window), before, "and the grid keeps its columns and rows");
    assert_eq!(resizes.get(), 0);
    assert_eq!(chip(cx, &window), None, "the chip waits for the new size");

    frames_apart(cx, &window, 8, Duration::from_millis(15));
    let after = told(cx, &window);
    assert!(after.0 < before.0, "bigger text, fewer columns: {before:?} then {after:?}");
    assert_eq!(drawn(cx, &window), after, "the grid is the new one");
    assert_eq!(resizes.get(), 1, "told once, at the end");
    assert_eq!(chip(cx, &window), Some(format!("{} × {}", after.0, after.1)));
    frames(cx, &window, 2);
    let shown = VisualTestContext::from_window(window.handle, cx).debug_bounds("size-readout");
    let shown = shown.expect("the chip is drawn");
    let pane = read(cx, &window, |ws| ws.pane_area);
    assert!(
        shown.size.height > px(100.0),
        "it covers the terminal, so its chip can sit in the middle: {shown:?}"
    );
    assert!(shown.size.width <= pane.size.width + px(1.0));
}

#[gpui_kit::test]
fn a_key_pressed_again_retargets_and_the_program_is_still_told_once(cx: &mut TestAppContext) {
    if isolate("a_key_pressed_again_retargets_and_the_program_is_still_told_once") {
        return;
    }
    let window = open(cx);
    let (resizes, _count) = count_resizes(cx, &window);
    zoom(cx, &window, Some(1.0));
    frames_apart(cx, &window, 3, Duration::from_millis(15));
    zoom(cx, &window, Some(1.0));
    frames_apart(cx, &window, 3, Duration::from_millis(15));
    zoom(cx, &window, None);
    assert_eq!(resizes.get(), 0, "three presses in half a second, and the program has not been told");
    frames_apart(cx, &window, 12, Duration::from_millis(15));
    assert_eq!(resizes.get(), 0, "⌘0 went back to the size it began from: the grid is the one it had");
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
    set_feature(cx, FeatureFlag::Motion, false);
    set_feature(cx, FeatureFlag::LiveResize, false);
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
    frames(cx, &window, 2);
    assert_eq!(resizes.get(), 3, "and once only");
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
fn the_chip_holds_700_ms_then_fades_for_150_and_goes(cx: &mut TestAppContext) {
    if isolate("the_chip_holds_700_ms_then_fades_for_150_and_goes") {
        return;
    }
    let window = open(cx);
    zoom(cx, &window, Some(1.0));
    frames_apart(cx, &window, 10, Duration::from_millis(15));
    assert!(chip(cx, &window).is_some());
    // The last change was a little while ago; 600 ms on it is whole still.
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    frames(cx, &window, 1);
    assert!(chip(cx, &window).is_some(), "still there");
    cx.executor().advance_clock(Duration::from_millis(250));
    cx.run_until_parked();
    frames_apart(cx, &window, 1, Duration::from_millis(60));
    assert!(chip(cx, &window).is_some(), "fading");
    frames_apart(cx, &window, 3, Duration::from_millis(60));
    assert_eq!(chip(cx, &window), None, "gone");
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

#[gpui_kit::test]
fn each_feature_has_its_own_part_the_live_resize_and_the_zoom(cx: &mut TestAppContext) {
    if isolate("each_feature_has_its_own_part_the_live_resize_and_the_zoom") {
        return;
    }
    let window = open(cx);
    let (resizes, _count) = count_resizes(cx, &window);
    test_live::set(Some(true));

    // Live resize off: a drag tells the program every size at once, and says nothing.
    set_feature(cx, FeatureFlag::LiveResize, false);
    for width in [1400.0, 1380.0, 1360.0] {
        resize_window(cx, &window, width);
        frames_apart(cx, &window, 1, Duration::from_millis(5));
    }
    assert_eq!(resizes.get(), 3, "no throttle without the feature");
    assert_eq!(chip(cx, &window), None, "and no chip for a drag");
    // Its zoom still eases and says the size: that is the Motion feature's.
    let before = told(cx, &window);
    zoom(cx, &window, Some(2.0));
    frames_apart(cx, &window, 3, Duration::from_millis(15));
    assert_eq!(told(cx, &window), before, "the text is on its way");
    frames_apart(cx, &window, 8, Duration::from_millis(15));
    assert!(chip(cx, &window).is_some());
    assert_eq!(resizes.get(), 4);

    // Live resize on and Motion off: the drag is throttled, the zoom is a cut with no chip.
    set_feature(cx, FeatureFlag::LiveResize, true);
    set_feature(cx, FeatureFlag::Motion, false);
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    frames(cx, &window, 2);
    let held_from = resizes.get();
    resize_window(cx, &window, 1300.0);
    frames_apart(cx, &window, 1, Duration::from_millis(5));
    resize_window(cx, &window, 1280.0);
    frames_apart(cx, &window, 1, Duration::from_millis(5));
    assert_eq!(resizes.get(), held_from + 1, "the second size of the drag is held back");
    test_live::set(None);
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    frames(cx, &window, 3);
    assert_eq!(chip(cx, &window), None, "the drag's chip has gone");
    zoom(cx, &window, None);
    frames(cx, &window, 2);
    assert_eq!(chip(cx, &window), None, "a zoom says nothing without the Motion feature");
}

#[gpui_kit::test]
fn a_terminal_that_is_not_on_screen_says_nothing_when_the_text_is_zoomed_and_switched_to(
    cx: &mut TestAppContext,
) {
    if isolate("a_terminal_that_is_not_on_screen_says_nothing_when_the_text_is_zoomed_and_switched_to") {
        return;
    }
    let window = start(cx, 2);
    cx.update(|cx| cx.set_reduce_motion(true));
    cx.simulate_window_resize(window.handle, size(px(1500.0), px(900.0)));
    frames(cx, &window, 3);
    // A tick tells each terminal whether it is on screen.
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    frames(cx, &window, 2);
    zoom(cx, &window, Some(1.0));
    frames(cx, &window, 3);
    assert!(chip(cx, &window).is_some(), "the terminal on screen says its size");
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    frames(cx, &window, 3);
    let other = read_cx(cx, &window, |ws, cx| ws.sessions[1].terminal().read(cx).size_readout_text());
    assert_eq!(other, None, "the one that was behind says nothing when it is switched to");
}
