//! A split opening (⌘D) in a real window (see `test_harness` for how each test runs in a child process with
//! scratch data): while the new pane's edge springs the window draws the layout it replaces with the new
//! pane's box over it, no terminal is resized, and when the spring is at rest the new layout is the picture
//! and the pane that was split is resized once. Motion off, Reduce Motion and a layout that changed meanwhile
//! are cuts. GPUI's animations run on the wall clock, which a test cannot advance, so these check what the
//! window holds, what it draws and its timers, which the test clock moves.
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{Subscription, TestAppContext, VisualTestContext, px, size};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::motion::split;
use wormhole_core::view::panes::Axis;

use crate::app_state::OpenWindow;
use crate::terminal_view::TerminalEvent;
use crate::window::test_harness::{draw, drive, isolate_in, read, start, with_window};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window with motion on (the harness starts with Reduce Motion), laid out so the panes' room is known.
fn open(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 1);
    cx.update(|cx| cx.set_reduce_motion(false));
    cx.simulate_window_resize(window.handle, size(px(1500.0), px(900.0)));
    settle_frames(cx, &window);
    window
}

fn settle_frames(cx: &mut TestAppContext, window: &OpenWindow) {
    for _ in 0..3 {
        draw(cx, window);
        cx.run_until_parked();
    }
}

/// Frames of the opening: each a few milliseconds on the clock the timers keep.
fn frames(cx: &mut TestAppContext, window: &OpenWindow, count: usize) {
    for _ in 0..count {
        with_window(cx, window, |window, cx| window.simulate_next_frame(cx));
        draw(cx, window);
        cx.executor().advance_clock(Duration::from_millis(5));
        cx.run_until_parked();
    }
}

fn finish(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.executor().advance_clock(split::duration() + Duration::from_millis(100));
    cx.run_until_parked();
    settle_frames(cx, window);
}

fn drawn(cx: &mut TestAppContext, window: &OpenWindow, selector: &'static str) -> bool {
    VisualTestContext::from_window(window.handle, cx).debug_bounds(selector).is_some()
}

/// Whether pane `id`'s header names it: only the panes of the layout that is drawn have one.
fn header(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> bool {
    // The selector is a `'static` name; a test leaks the few it asks for.
    drawn(cx, window, Box::leak(format!("pane-name-{id}").into_boxed_str()))
}

fn bounds_of(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    selector: &'static str,
) -> Option<gpui_kit::Bounds<gpui_kit::Pixels>> {
    VisualTestContext::from_window(window.handle, cx).debug_bounds(selector)
}

fn set_gaps(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| crate::settings::update_settings(cx, |settings| settings.split_gaps = on));
}

/// Which layers paint the terminal's colour in the frame drawn: the column, and each pane of the picture.
fn coats(cx: &mut TestAppContext, window: &OpenWindow, panes: &[i64]) -> (bool, Vec<bool>) {
    let column = drawn(cx, window, "column-coat");
    let own = panes
        .iter()
        .map(|id| drawn(cx, window, Box::leak(format!("pane-coat-{id}").into_boxed_str())))
        .collect();
    (column, own)
}

fn opening(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read(cx, window, |ws| ws.active_split().is_some_and(|split| split.opening.is_some()))
}

/// How many times terminal `id` has been told it has another size.
fn count_resizes(cx: &mut TestAppContext, window: &OpenWindow, id: i64) -> (Rc<Cell<usize>>, Subscription) {
    let terminal = read(cx, window, |ws| ws.pane_terminal(id).expect("a terminal"));
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

fn split(cx: &mut TestAppContext, window: &OpenWindow, axis: Axis) {
    drive(cx, window, |ws, window, cx| ws.split_pane(axis, window, cx));
}

#[gpui_kit::test]
fn a_lone_terminal_is_split_under_a_picture_of_itself_and_resized_once_at_the_end(cx: &mut TestAppContext) {
    if isolate("a_lone_terminal_is_split_under_a_picture_of_itself_and_resized_once_at_the_end") {
        return;
    }
    let window = open(cx);
    let (old, _old) = count_resizes(cx, &window, 1);
    let before = read(cx, &window, |ws| ws.sessions[0].pane.grid.size());
    split(cx, &window, Axis::Row);
    let new_id = read(cx, &window, |ws| ws.sessions[ws.active].id);
    assert_ne!(new_id, 1, "the new pane has the keyboard");
    let (new, _new) = count_resizes(cx, &window, new_id);
    assert!(opening(cx, &window), "the split opens");

    // Frames of the spring: the terminal is as it was, alone in its room, and the new pane's box slides in.
    frames(cx, &window, 12);
    assert!(opening(cx, &window), "the spring has not ended");
    assert!(drawn(cx, &window, "split-opening-new"), "the new pane's box is drawn");
    assert!(header(cx, &window, 1), "the pane that was split has its header from the start");
    assert!(header(cx, &window, new_id), "and the new one has its own");
    assert!(drawn(cx, &window, "opening-shield"), "the picture of the layout before is under a shield");
    let kept = bounds_of(cx, &window, "split-opening-kept").expect("the pane that was split is drawn");
    let whole = read(cx, &window, |ws| f32::from(ws.pane_area.size.width));
    assert!(f32::from(kept.size.width) < whole, "its box has given way to the edge: {kept:?} in {whole}");
    assert_eq!(old.get(), 0, "its terminal was not resized while the edge moved");
    assert_eq!(read(cx, &window, |ws| ws.sessions[0].pane.grid.size()), before);
    let new_during = new.get();
    assert!(new_during <= 1, "the new terminal is made its size once: {new_during}");

    // The spring is at rest: the layout is the picture and the pane that was split has its new size, once.
    finish(cx, &window);
    assert!(!opening(cx, &window), "it ended");
    assert!(!drawn(cx, &window, "split-opening-new"), "the picture is the layout");
    assert!(!drawn(cx, &window, "opening-shield"), "and live");
    assert!(header(cx, &window, 1), "the pane that was split has its header");
    assert_eq!(old.get(), 1, "the pane that was split was resized exactly once, at the end");
    assert_eq!(new.get(), new_during, "and the new one not again");
    let after = read(cx, &window, |ws| ws.sessions[0].pane.grid.size());
    assert!(after.0 < before.0, "{after:?} is narrower than {before:?}");
}

#[gpui_kit::test]
fn a_pane_of_a_split_opens_a_new_one_over_its_own_room_and_the_others_are_not_resized(
    cx: &mut TestAppContext,
) {
    if isolate("a_pane_of_a_split_opens_a_new_one_over_its_own_room_and_the_others_are_not_resized") {
        return;
    }
    let window = open(cx);
    split(cx, &window, Axis::Row);
    finish(cx, &window);
    let second = read(cx, &window, |ws| ws.sessions[ws.active].id);
    let (first, _first) = count_resizes(cx, &window, 1);
    let (split_one, _split_one) = count_resizes(cx, &window, second);
    // The focused (second) pane is split down.
    split(cx, &window, Axis::Column);
    assert!(opening(cx, &window));
    frames(cx, &window, 12);
    assert!(drawn(cx, &window, "split-opening-new"));
    assert!(header(cx, &window, 1) && header(cx, &window, second), "both panes of the split as they were");
    assert_eq!((first.get(), split_one.get()), (0, 0), "nothing was resized while it moved");
    finish(cx, &window);
    assert!(!opening(cx, &window));
    assert_eq!(first.get(), 0, "the pane beside it never was");
    assert_eq!(split_one.get(), 1, "the one that was split, once, at the end");
}

#[gpui_kit::test]
fn a_split_down_opens_the_same_way(cx: &mut TestAppContext) {
    if isolate("a_split_down_opens_the_same_way") {
        return;
    }
    let window = open(cx);
    let (old, _old) = count_resizes(cx, &window, 1);
    split(cx, &window, Axis::Column);
    frames(cx, &window, 8);
    assert!(opening(cx, &window) && drawn(cx, &window, "split-opening-new"));
    assert_eq!(old.get(), 0);
    finish(cx, &window);
    assert_eq!(old.get(), 1);
    assert!(read(cx, &window, |ws| ws.active_split().is_some_and(|split| split.layout.panes().len() == 2)));
}

#[gpui_kit::test]
fn motion_off_and_reduce_motion_split_by_a_cut(cx: &mut TestAppContext) {
    if isolate("motion_off_and_reduce_motion_split_by_a_cut") {
        return;
    }
    let window = open(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    split(cx, &window, Axis::Row);
    assert!(!opening(cx, &window), "Reduce Motion cuts it");
    draw(cx, &window);
    assert!(header(cx, &window, 1) && !drawn(cx, &window, "split-opening-new"));

    cx.update(|cx| {
        cx.set_reduce_motion(false);
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), false);
        });
    });
    split(cx, &window, Axis::Row);
    assert!(!opening(cx, &window), "without the Motion feature it is today's cut");
}

#[gpui_kit::test]
fn a_layout_that_changes_or_a_window_resized_while_it_opens_is_drawn_as_it_is(cx: &mut TestAppContext) {
    if isolate("a_layout_that_changes_or_a_window_resized_while_it_opens_is_drawn_as_it_is") {
        return;
    }
    let window = open(cx);
    split(cx, &window, Axis::Row);
    frames(cx, &window, 4);
    assert!(drawn(cx, &window, "split-opening-new"));
    // Zoom the pane: the picture is the layout at once.
    drive(cx, &window, |ws, _, cx| ws.toggle_zoom(cx));
    draw(cx, &window);
    assert!(!drawn(cx, &window, "split-opening-new"), "a zoomed pane is no opening");
    finish(cx, &window);

    drive(cx, &window, |ws, _, cx| ws.toggle_zoom(cx));
    finish(cx, &window);
    split(cx, &window, Axis::Row);
    frames(cx, &window, 4);
    assert!(drawn(cx, &window, "split-opening-new"));
    cx.simulate_window_resize(window.handle, size(px(1300.0), px(800.0)));
    draw(cx, &window);
    draw(cx, &window);
    assert!(!drawn(cx, &window, "split-opening-new"), "another room: the layout as it is");
}

#[gpui_kit::test]
fn the_new_pane_and_the_one_that_was_split_sit_on_one_coat_of_the_terminals_colour(cx: &mut TestAppContext) {
    if isolate("the_new_pane_and_the_one_that_was_split_sit_on_one_coat_of_the_terminals_colour") {
        return;
    }
    let window = open(cx);
    for gaps in [true, false] {
        set_gaps(cx, gaps);
        // A lone terminal, then a pane of a split: both are drawn on one coat while the new pane opens.
        for nested in [false, true] {
            if nested {
                finish(cx, &window);
            }
            split(cx, &window, Axis::Row);
            let new_id = read(cx, &window, |ws| ws.sessions[ws.active].id);
            frames(cx, &window, 6);
            assert!(opening(cx, &window), "gaps {gaps} nested {nested}: it is opening");
            let split_id = read(cx, &window, |ws| {
                ws.active_split().expect("a split").opening.as_ref().map(|o| o.target_for_test())
            });
            let target = split_id.expect("an opening");
            let (column, own) = coats(cx, &window, &[target, new_id]);
            if gaps {
                assert!(
                    !column,
                    "gaps {gaps} nested {nested}: each pane has its own surface, so the column has none"
                );
                assert_eq!(
                    own,
                    [true, true],
                    "gaps {gaps} nested {nested}: the pane that was split and the new one each paint theirs"
                );
            } else {
                assert!(column, "gaps {gaps} nested {nested}: the column paints it once");
                assert_eq!(own, [false, false], "gaps {gaps} nested {nested}: and the panes paint none");
            }
            finish(cx, &window);
        }
        // Back to one terminal for the next setting: a window of a lone terminal again.
        drive(cx, &window, |ws, window, cx| {
            while ws.active_split().is_some() {
                let id = ws.sessions[ws.active].id;
                ws.close_pane(id, window, cx);
                if ws.session_count() <= 1 {
                    break;
                }
            }
        });
        finish(cx, &window);
    }
}

#[gpui_kit::test]
fn a_pane_of_a_split_gives_way_to_the_new_pane_and_keeps_its_grid(cx: &mut TestAppContext) {
    if isolate("a_pane_of_a_split_gives_way_to_the_new_pane_and_keeps_its_grid") {
        return;
    }
    let window = open(cx);
    split(cx, &window, Axis::Row);
    finish(cx, &window);
    let second = read(cx, &window, |ws| ws.sessions[ws.active].id);
    let name: &'static str = Box::leak(format!("pane-coat-{second}").into_boxed_str());
    let whole = bounds_of(cx, &window, name).expect("the second pane");
    let grid =
        read(cx, &window, |ws| ws.sessions[ws.find_session(second).expect("a session")].pane.grid.size());
    split(cx, &window, Axis::Column);
    frames(cx, &window, 6);
    let mid = bounds_of(cx, &window, name).expect("the pane that is being split is still drawn");
    // The layout shares its width after the gaps and core's rectangles before them: a few points of difference.
    let narrower = f32::from(whole.size.width) - f32::from(mid.size.width);
    assert!(narrower.abs() < 6.0, "a split down gives up height and keeps its width: {mid:?} from {whole:?}");
    assert!(mid.size.height < whole.size.height, "its box gave way to the edge: {mid:?} from {whole:?}");
    assert_eq!(
        read(cx, &window, |ws| ws.sessions[ws.find_session(second).expect("a session")].pane.grid.size()),
        grid,
        "its terminal was held at the grid it had"
    );
    let new = bounds_of(cx, &window, "split-opening-new").expect("the new pane");
    assert!(
        f32::from(new.origin.y) >= f32::from(mid.origin.y + mid.size.height) - 0.5,
        "the two meet at the edge and do not overlap: {mid:?} {new:?}"
    );
    finish(cx, &window);
}
