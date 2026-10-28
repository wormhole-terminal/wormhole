//! A warning on the notice chip in a real window (GPUI's test platform; see `test_harness`): it is said once
//! about the terminal whose host couldn't start, stays until it is clicked, gives its words to a copy for a
//! moment and takes them back, and is not said again once it was dismissed. Where the chip is off the old
//! banner says it. The panel is native and not made in a test, so the click goes over the same channel the
//! panel uses (`notice::click`) and what is read is the chip's rules (`notice::notices`).

use std::time::Duration;

use gpui_kit::TestAppContext;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::motion::notice::{LIFE, UP};
use wormhole_core::view::notice::Phase;
use wormhole_text::{Language, with_language};

use super::reported_tests::terminal;
use super::test_harness::{draw, isolate_in, start};
use crate::app_state::OpenWindow;
use crate::settings::update_settings;

const SHORT: &str = "This terminal ends when Wormhole quits";

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn set_notices(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        update_settings(cx, |s| _ = s.feature_flags.insert(FeatureFlag::Notices.key().into(), on))
    });
}

/// The window's chip: phase, words, whether a warning is up and whether it takes a click.
fn chip(cx: &mut TestAppContext) -> Option<(Phase, String, bool, bool)> {
    cx.update(|cx| {
        crate::notice::notices(cx).into_iter().next().map(|r| (r.phase, r.words, r.sticky, r.clickable))
    })
}

fn lost_host(cx: &mut TestAppContext, window: &OpenWindow) {
    let terminal = terminal(cx, window, 0);
    cx.update(|cx| terminal.update(cx, |t, cx| t.tell_fallback_for_test(cx)));
    cx.run_until_parked();
}

fn click(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    let id = window.handle.window_id();
    let sent = cx.update(|cx| crate::notice::click(id, cx));
    cx.run_until_parked();
    sent
}

fn copy(cx: &mut TestAppContext, window: &OpenWindow, text: &str) {
    let terminal = terminal(cx, window, 0);
    cx.update(|cx| terminal.update(cx, |t, cx| t.store_clipboard_for_test(text, true, cx)));
    cx.run_until_parked();
}

fn wait(cx: &mut TestAppContext, by: Duration) {
    cx.executor().advance_clock(by);
    cx.run_until_parked();
}

/// A window with the chip on, drawn.
fn with_chip(cx: &mut TestAppContext, body: impl FnOnce(&mut TestAppContext, &OpenWindow)) {
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, true);
        draw(cx, &window);
        draw(cx, &window);
        body(cx, &window);
    });
}

#[gpui_kit::test]
fn a_host_that_could_not_start_is_said_in_one_short_line_that_stays(cx: &mut TestAppContext) {
    if isolate("a_host_that_could_not_start_is_said_in_one_short_line_that_stays") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        assert_eq!(chip(cx), Some((Phase::Showing, SHORT.to_string(), true, true)));
        wait(cx, Duration::from_secs(600));
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Showing), "ten minutes on, it is still there");
        let banner =
            terminal(cx, window, 0).read_with(cx, |t, _| t.host_banner_for_test().map(str::to_string));
        assert_eq!(banner, None, "the chip says it, not the banner too");
    });
}

#[gpui_kit::test]
fn a_click_takes_the_warning_away_for_good(cx: &mut TestAppContext) {
    if isolate("a_click_takes_the_warning_away_for_good") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        assert!(click(cx, window), "the chip hears the click");
        assert_eq!(chip(cx).map(|c| (c.0, c.3)), Some((Phase::Leaving, false)));
        wait(cx, UP + Duration::from_millis(10));
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Hidden));
        lost_host(cx, window);
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Hidden), "said once, not again");
    });
}

#[gpui_kit::test]
fn a_copy_takes_the_words_for_its_life_and_the_warning_comes_back(cx: &mut TestAppContext) {
    if isolate("a_copy_takes_the_words_for_its_life_and_the_warning_comes_back") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        copy(cx, window, "twelve chars");
        assert_eq!(chip(cx), Some((Phase::Showing, "Copied 12 characters".to_string(), true, false)));
        assert!(click(cx, window));
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Showing), "a click on a copy's words does nothing");
        wait(cx, LIFE + Duration::from_millis(10));
        assert_eq!(chip(cx), Some((Phase::Showing, SHORT.to_string(), true, true)));
        assert!(click(cx, window));
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Leaving));
    });
}

#[gpui_kit::test]
fn with_the_chip_off_the_banner_says_it(cx: &mut TestAppContext) {
    if isolate("with_the_chip_off_the_banner_says_it") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, false);
        lost_host(cx, &window);
        assert_eq!(chip(cx), None);
        let banner =
            terminal(cx, &window, 0).read_with(cx, |t, _| t.host_banner_for_test().map(str::to_string));
        assert_eq!(
            banner.as_deref(),
            Some(
                "This terminal runs inside Wormhole and ends when it quits: the helper that keeps terminals running couldn't start."
            )
        );
    });
}
