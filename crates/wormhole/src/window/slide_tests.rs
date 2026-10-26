//! A side column hiding and showing in a real window, with each style of `ColumnMotion` (see
//! `test_harness` for how each test runs in a child process with scratch data): the slide lasts as long as
//! the style says, the terminal resizes once for each way, at the end of a column coming in and at the start
//! of one going out and never a little at a time, the layout is today's when it is over, a toggle made
//! while one is going turns it round, and a build without the Motion feature and Reduce Motion move as they
//! always did. GPUI's animations run on the wall clock, which a test cannot advance, so these check what the
//! window holds and its timers, which the test clock moves.
use std::time::Duration;

use gpui_kit::{TestAppContext, px, size};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::layout::Edge;
use wormhole_core::view::motion::pose::column;
use wormhole_core::view::motion::{ColumnMotion, MotionChoice, MotionChoices};

use crate::app_state::OpenWindow;
use crate::window::test_harness::{draw, drive, isolate_in, read, start, with_window};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window wide enough for both columns, with motion on (the harness starts with Reduce Motion), at rest.
fn open(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 1);
    cx.update(|cx| cx.set_reduce_motion(false));
    cx.simulate_window_resize(window.handle, size(px(1800.0), px(900.0)));
    settle_frames(cx, &window);
    window
}

fn choose(cx: &mut TestAppContext, column: ColumnMotion) {
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.motion = MotionChoices { column, ..MotionChoices::CLASSIC }
        });
    });
}

fn settle_frames(cx: &mut TestAppContext, window: &OpenWindow) {
    for _ in 0..3 {
        draw(cx, window);
        cx.run_until_parked();
    }
}

/// The terminal's grid, as its last frame laid it out.
fn grid(cx: &mut TestAppContext, window: &OpenWindow) -> (usize, usize) {
    read(cx, window, |ws| ws.sessions[0].pane.grid.size())
}

/// Toggle the sidebar and draw `frames` frames of the slide, each a few milliseconds apart; the grid after
/// each of them, which is how often the terminal has another size.
fn toggle_and_watch(cx: &mut TestAppContext, window: &OpenWindow, frames: usize) -> Vec<(usize, usize)> {
    drive(cx, window, |ws, _, cx| ws.toggle_sidebar(cx));
    let mut seen = Vec::new();
    for _ in 0..frames {
        with_window(cx, window, |window, cx| window.simulate_next_frame(cx));
        draw(cx, window);
        cx.executor().advance_clock(Duration::from_millis(5));
        cx.run_until_parked();
        seen.push(grid(cx, window));
    }
    seen
}

fn distinct(sizes: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for &size in sizes {
        if out.last() != Some(&size) {
            out.push(size);
        }
    }
    out
}

fn sliding(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read(cx, window, |ws| ws.slide_motion().is_some())
}

fn finish(cx: &mut TestAppContext, window: &OpenWindow, duration: Duration) {
    cx.executor().advance_clock(duration + Duration::from_millis(60));
    cx.run_until_parked();
    settle_frames(cx, window);
}

#[gpui_kit::test]
fn each_style_slides_for_its_own_time_and_leaves_todays_layout(cx: &mut TestAppContext) {
    if isolate("each_style_slides_for_its_own_time_and_leaves_todays_layout") {
        return;
    }
    let window = open(cx);
    let rest = read(cx, &window, |ws| (ws.columns().sidebar, ws.in_flow(Edge::Sidebar)));
    assert!(rest.0 > 0.0 && rest.1, "the sidebar is in at rest");
    for &variant in ColumnMotion::ALL {
        choose(cx, variant);
        for opening in [false, true] {
            drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
            assert!(sliding(cx, &window), "{variant:?} opening {opening}: it slides");
            let duration = column::duration(variant, opening);
            cx.executor().advance_clock(duration - Duration::from_millis(30));
            cx.run_until_parked();
            assert!(
                sliding(cx, &window),
                "{variant:?} opening {opening}: it is still going before its time is up"
            );
            draw(cx, &window);
            cx.executor().advance_clock(Duration::from_millis(60));
            cx.run_until_parked();
            assert!(!sliding(cx, &window), "{variant:?} opening {opening}: and over after it");
            settle_frames(cx, &window);
        }
        let end = read(cx, &window, |ws| (ws.columns().sidebar, ws.in_flow(Edge::Sidebar)));
        assert_eq!(end, rest, "{variant:?}: it ends where it began, in the layout");
    }
}

#[gpui_kit::test]
fn the_terminal_resizes_once_for_each_way_and_never_a_frame_at_a_time(cx: &mut TestAppContext) {
    if isolate("the_terminal_resizes_once_for_each_way_and_never_a_frame_at_a_time") {
        return;
    }
    let window = open(cx);
    let with_column = grid(cx, &window);
    for &variant in ColumnMotion::ALL {
        choose(cx, variant);
        // Going out: the column leaves the layout at once, so the terminal takes its room at the first frame
        // and keeps that size all the way.
        let hiding = toggle_and_watch(cx, &window, 12);
        finish(cx, &window, column::duration(variant, false));
        let without = grid(cx, &window);
        assert!(without.0 > with_column.0, "{variant:?}: the terminal has the column's room");
        let sizes = distinct(&hiding);
        assert!(sizes.len() <= 2 && sizes.last() == Some(&without), "{variant:?} hiding: {sizes:?}");
        // Coming in: the column lies over the terminal at its width, which does not change until the column
        // is in, and then once.
        let showing = toggle_and_watch(cx, &window, 12);
        assert_eq!(
            distinct(&showing),
            vec![without],
            "{variant:?}: the terminal holds its size while it slides"
        );
        finish(cx, &window, column::duration(variant, true));
        assert_eq!(
            grid(cx, &window),
            with_column,
            "{variant:?}: it lands in the layout and the terminal gives the room back"
        );
    }
}

#[gpui_kit::test]
fn the_spring_and_content_after_turn_round_from_where_the_column_is_and_classic_starts_from_rest(
    cx: &mut TestAppContext,
) {
    if isolate(
        "the_spring_and_content_after_turn_round_from_where_the_column_is_and_classic_starts_from_rest",
    ) {
        return;
    }
    let window = open(cx);
    let shown_at_start = |cx: &mut TestAppContext| {
        read(cx, &window, |ws| ws.slide_motion().map(|slide| slide.pose(0.0).shown))
    };
    for (variant, turns) in
        [(ColumnMotion::Classic, false), (ColumnMotion::SoftSpring, true), (ColumnMotion::ContentAfter, true)]
    {
        choose(cx, variant);
        drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
        assert_eq!(shown_at_start(cx), Some(1.0), "{variant:?}: going out it starts all the way in");
        cx.executor().advance_clock(Duration::from_millis(120));
        cx.run_until_parked();
        drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
        let from = shown_at_start(cx).expect("it slides again");
        if turns {
            assert!(from > 0.05 && from < 0.99, "{variant:?}: it turns round from where it was: {from}");
        } else {
            assert_eq!(from, 0.0, "{variant:?}: Classic starts from rest, as it always did");
        }
        finish(cx, &window, Duration::from_secs(1));
        assert!(!sliding(cx, &window));
    }
}

#[gpui_kit::test]
fn without_the_feature_and_under_reduce_motion_a_column_moves_as_it_always_did(cx: &mut TestAppContext) {
    if isolate("without_the_feature_and_under_reduce_motion_a_column_moves_as_it_always_did") {
        return;
    }
    let window = open(cx);
    choose(cx, ColumnMotion::SoftSpring);
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), false);
        });
    });
    drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
    assert!(sliding(cx, &window));
    // Today's 200 ms, not the spring's.
    cx.executor().advance_clock(column::CLASSIC_OUT + Duration::from_millis(10));
    cx.run_until_parked();
    assert!(!sliding(cx, &window), "the choice waits for the feature");
    settle_frames(cx, &window);

    cx.update(|cx| {
        cx.set_reduce_motion(true);
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), true);
        });
    });
    drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
    assert!(!sliding(cx, &window), "Reduce Motion cuts it");
}

#[gpui_kit::test]
fn the_right_panel_slides_in_each_style_too(cx: &mut TestAppContext) {
    if isolate("the_right_panel_slides_in_each_style_too") {
        return;
    }
    let window = open(cx);
    for &variant in ColumnMotion::ALL {
        choose(cx, variant);
        for opening in [false, true] {
            drive(cx, &window, |ws, window, cx| ws.toggle_notes(window, cx));
            assert!(read(cx, &window, |ws| ws.slide_motion().is_some_and(|slide| slide.edge == Edge::Panel)));
            // A frame in the middle of each draws the column over a surface of its own where the style fades.
            cx.executor().advance_clock(column::duration(variant, opening) / 2);
            with_window(cx, &window, |window, cx| window.simulate_next_frame(cx));
            draw(cx, &window);
            finish(cx, &window, column::duration(variant, opening));
        }
        assert!(read(cx, &window, |ws| ws.columns().panel > 0.0 && ws.in_flow(Edge::Panel)));
    }
}

#[gpui_kit::test]
fn content_after_turned_round_carries_what_shows_of_the_content_over(cx: &mut TestAppContext) {
    if isolate("content_after_turned_round_carries_what_shows_of_the_content_over") {
        return;
    }
    let window = open(cx);
    choose(cx, ColumnMotion::ContentAfter);
    drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
    let began = read(cx, &window, |ws| ws.slide_motion().map(|slide| slide.pose(0.0).content));
    assert_eq!(began, Some(1.0), "hiding starts with all of the content");
    // Forty milliseconds in, the content is half way out; showing the column again goes on from there and
    // does not flash it back to nothing or to all of it.
    cx.executor().advance_clock(Duration::from_millis(40));
    cx.run_until_parked();
    drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
    let turned =
        read(cx, &window, |ws| ws.slide_motion().map(|slide| slide.pose(0.0))).expect("it slides again");
    assert!(turned.content > 0.1 && turned.content < 0.9, "{turned:?}");
    assert!(turned.shown > 0.99, "the clip had not moved yet: {turned:?}");
}
