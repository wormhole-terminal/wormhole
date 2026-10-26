//! The command palette's exit in a real window (see `test_harness` for how each test runs in a child
//! process with scratch data): with motion on it is closed at once as far as the window goes (the keyboard
//! is back, it counts no longer open) while its last picture fades under a shield, and a timer takes the
//! picture down; Reduce Motion, a build without the Motion feature and the shortcut sheet cut as they
//! always did. GPUI's animations run on the wall clock, which a test cannot advance, so these check what
//! the window holds and its timers, which the test clock moves.
use std::time::Duration;

use gpui_kit::{Focusable as _, TestAppContext, VisualTestContext, px, size};
use wormhole_core::config::features::FeatureFlag;

use crate::app_state::OpenWindow;
use crate::window::test_harness::{draw, drive, isolate_in, read, start, with_window};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window with motion on (the harness starts with Reduce Motion), drawn at rest.
fn open(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 1);
    cx.update(|cx| cx.set_reduce_motion(false));
    cx.simulate_window_resize(window.handle, size(px(1200.0), px(900.0)));
    draw(cx, &window);
    window
}

fn open_palette(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.update(|cx| crate::command_palette::open_in(window, "", cx));
    cx.run_until_parked();
    draw(cx, window);
}

fn close_overlay(cx: &mut TestAppContext, window: &OpenWindow) {
    drive(cx, window, |ws, window, cx| ws.close_overlay(window, cx));
    draw(cx, window);
}

fn drawn(cx: &mut TestAppContext, window: &OpenWindow, selector: &'static str) -> bool {
    VisualTestContext::from_window(window.handle, cx).debug_bounds(selector).is_some()
}

fn terminal_focused(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    with_window(cx, window, |win, cx| {
        let workspace = window.workspace.upgrade().expect("the window is open");
        workspace.read(cx).active_terminal().focus_handle(cx).is_focused(win)
    })
}

fn leaving(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read(cx, window, |ws| ws.overlay.is_leaving())
}

#[gpui_kit::test]
fn a_palette_closes_at_once_and_its_picture_fades_for_a_moment_and_is_gone_after(cx: &mut TestAppContext) {
    if isolate("a_palette_closes_at_once_and_its_picture_fades_for_a_moment_and_is_gone_after") {
        return;
    }
    let window = open(cx);
    open_palette(cx, &window);
    assert!(read(cx, &window, |ws| ws.palette_open()));
    assert!(drawn(cx, &window, "overlay"), "it is drawn on the first frame");
    assert!(!drawn(cx, &window, "overlay-shield"), "and live");
    assert!(!terminal_focused(cx, &window), "its search has the keyboard");

    close_overlay(cx, &window);
    assert!(!read(cx, &window, |ws| ws.palette_open()), "closed: it takes no key");
    assert!(terminal_focused(cx, &window), "the keyboard is back before it has faded");
    assert!(leaving(cx, &window), "its picture is kept");
    assert!(drawn(cx, &window, "overlay-shield"), "drawn under a shield that takes the pointer");
    assert!(!drawn(cx, &window, "overlay"), "and no longer the open overlay");

    // Its time goes by on the executor's clock.
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();
    draw(cx, &window);
    assert!(!leaving(cx, &window), "the timer took it down");
    assert!(!drawn(cx, &window, "overlay-shield"), "and nothing of it is drawn");
}

#[gpui_kit::test]
fn a_palette_opened_again_while_one_fades_replaces_it(cx: &mut TestAppContext) {
    if isolate("a_palette_opened_again_while_one_fades_replaces_it") {
        return;
    }
    let window = open(cx);
    open_palette(cx, &window);
    close_overlay(cx, &window);
    assert!(leaving(cx, &window));
    open_palette(cx, &window);
    assert!(read(cx, &window, |ws| ws.palette_open()), "it opens");
    assert!(!leaving(cx, &window), "and the fading one is gone");
    assert!(drawn(cx, &window, "overlay") && !drawn(cx, &window, "overlay-shield"));
    // The first one's timer must not take anything of the second down.
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();
    assert!(read(cx, &window, |ws| ws.palette_open()));
}

#[gpui_kit::test]
fn the_palette_closes_by_a_cut_under_reduce_motion_and_without_the_feature(cx: &mut TestAppContext) {
    if isolate("the_palette_closes_by_a_cut_under_reduce_motion_and_without_the_feature") {
        return;
    }
    let window = open(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    open_palette(cx, &window);
    close_overlay(cx, &window);
    assert!(!leaving(cx, &window), "Reduce Motion cuts it");
    assert!(!drawn(cx, &window, "overlay-shield"));

    cx.update(|cx| {
        cx.set_reduce_motion(false);
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert(FeatureFlag::Motion.key().to_string(), false);
        });
    });
    open_palette(cx, &window);
    close_overlay(cx, &window);
    assert!(!leaving(cx, &window), "without the Motion feature it is today's cut");
    assert!(terminal_focused(cx, &window));
}

#[gpui_kit::test]
fn the_shortcut_sheet_closes_by_a_cut(cx: &mut TestAppContext) {
    if isolate("the_shortcut_sheet_closes_by_a_cut") {
        return;
    }
    let window = open(cx);
    cx.update(crate::shortcut_sheet::toggle);
    cx.run_until_parked();
    draw(cx, &window);
    assert!(read(cx, &window, |ws| ws.sheet_open()));
    close_overlay(cx, &window);
    assert!(!leaving(cx, &window), "only the palette fades");
}
