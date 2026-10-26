//! The right panel's tab switch in a real window (see `test_harness` for how each test runs in a child
//! process with scratch data): the new tab's content slides in from the side of the tab it came from,
//! the animation draws the panel's region and no other, and it is a cut with Reduce Motion, without the
//! Motion feature and when the panel was not on screen. GPUI's animations run on the wall clock, which a
//! test cannot advance, so these check what the window holds, its own clock (the executor's) and which
//! regions the frames the animation asks for build.
use std::time::Duration;

use gpui_kit::{TestAppContext, px, size};
use wormhole_core::config::TabId;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::motion::tabs::Side;

use crate::app_state::OpenWindow;
use crate::window::test_harness::{draw, draw_replaying, drive, isolate_in, read_cx, start, with_window};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window with its panel on screen and motion on (the harness starts with Reduce Motion), drawn at rest.
fn open(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 1);
    cx.update(|cx| cx.set_reduce_motion(false));
    cx.simulate_window_resize(window.handle, size(px(1400.0), px(900.0)));
    drive(cx, &window, |ws, window, cx| {
        if !ws.notes_visible() {
            ws.toggle_notes(window, cx);
        }
    });
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    draw(cx, &window);
    draw(cx, &window);
    window
}

/// The panel's tabs in the bar's order, with the tab shown first.
fn tabs(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<TabId> {
    read_cx(cx, window, |ws, cx| ws.panel_tab_list(cx))
}

fn playing(cx: &mut TestAppContext, window: &OpenWindow) -> Option<(Side, u64)> {
    read_cx(cx, window, |ws, cx| ws.regions.drawn.panel_switch.playing(cx.background_executor().now()))
}

fn show(cx: &mut TestAppContext, window: &OpenWindow, tab: TabId) {
    drive(cx, window, |ws, _, cx| ws.show_tab(tab, cx));
    draw_replaying(cx, window);
}

#[gpui_kit::test]
fn a_switch_comes_from_the_side_of_its_tab_plays_a_moment_and_leaves_nothing_after(cx: &mut TestAppContext) {
    if isolate("a_switch_comes_from_the_side_of_its_tab_plays_a_moment_and_leaves_nothing_after") {
        return;
    }
    let window = open(cx);
    let tabs = tabs(cx, &window);
    assert!(tabs.len() >= 3, "{tabs:?}");
    show(cx, &window, tabs[0]);
    assert_eq!(playing(cx, &window), None, "a tab chosen again is no switch");
    show(cx, &window, tabs[2]);
    assert_eq!(playing(cx, &window), Some((Side::Right, 1)), "to a tab after the one it left");
    show(cx, &window, tabs[1]);
    assert_eq!(playing(cx, &window), Some((Side::Left, 2)), "to a tab before it: from the left");
    // Its time goes by on the executor's clock; the next build of the panel drops it.
    cx.executor().advance_clock(Duration::from_millis(250));
    drive(cx, &window, |ws, _, cx| ws.mark_dirty(cx));
    draw(cx, &window);
    assert_eq!(playing(cx, &window), None, "it is over");
}

#[gpui_kit::test]
fn a_switch_asks_for_frames_that_build_the_panel_and_no_other_region(cx: &mut TestAppContext) {
    if isolate("a_switch_asks_for_frames_that_build_the_panel_and_no_other_region") {
        return;
    }
    let window = open(cx);
    let tabs = tabs(cx, &window);
    show(cx, &window, tabs[2]);
    // The frame the click made draws everything; the frames the animation asks for after it build the panel.
    draw_replaying(cx, &window);
    let builds = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| ws.regions.builds(cx));
    let before = builds(cx);
    let mut asked = 0;
    for _ in 0..4 {
        let from = builds(cx);
        // The frame the animation asked for arrives, and the window draws as the display would.
        with_window(cx, &window, |window, cx| window.simulate_next_frame(cx));
        draw_replaying(cx, &window);
        let now = builds(cx);
        assert_eq!(now[0], from[0], "the sidebar's header was not built");
        assert_eq!(now[2..], from[2..], "nor the title row, nor the status bar");
        assert!(now[1] - from[1] <= 1, "the panel is built once for a frame");
        asked += now[1] - from[1];
    }
    assert!(asked > 0, "the switch asked for frames of the panel");
    assert_eq!(builds(cx)[1], before[1] + asked);
}

#[gpui_kit::test]
fn nothing_slides_under_reduce_motion_without_the_feature_or_in_a_panel_that_is_not_there(
    cx: &mut TestAppContext,
) {
    if isolate("nothing_slides_under_reduce_motion_without_the_feature_or_in_a_panel_that_is_not_there") {
        return;
    }
    let window = open(cx);
    let tabs = tabs(cx, &window);
    cx.update(|cx| cx.set_reduce_motion(true));
    show(cx, &window, tabs[1]);
    assert_eq!(playing(cx, &window), None, "Reduce Motion cuts it");
    cx.update(|cx| {
        cx.set_reduce_motion(false);
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), false);
        });
    });
    show(cx, &window, tabs[2]);
    assert_eq!(playing(cx, &window), None, "without the Motion feature it is today's cut");
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), true);
        });
    });
    drive(cx, &window, |ws, window, cx| ws.toggle_notes(window, cx));
    draw(cx, &window);
    show(cx, &window, tabs[1]);
    assert_eq!(playing(cx, &window), None, "a panel that is not on screen has no content to slide");
}
