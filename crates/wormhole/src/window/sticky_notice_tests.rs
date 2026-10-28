//! A warning on the notice chip in a real window (GPUI's test platform; see `test_harness`): it is said once
//! about the terminal whose host couldn't start, stays until it is clicked, gives its words to a copy for a
//! moment and takes them back, and is not said again once it was dismissed. Where the chip is off the old
//! banner says it. The panel is native and not made in a test, so the click goes over the same channel the
//! panel uses (`notice::click`) and what is read is the chip's rules (`notice::notices`). A warning keeps to
//! its terminal: it waits out of sight while the terminal is off screen or an overlay covers the window,
//! follows the terminal when the layout moves it, and goes when the terminal is closed (without the chip
//! keeping the closed terminal alive).

use std::time::Duration;

use gpui_kit::TestAppContext;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::motion::notice::{LIFE, UP};
use wormhole_core::view::notice::{Buttons, Phase, Press};
use wormhole_core::view::panes::Rect;
use wormhole_text::{Language, with_language};

use super::reported_tests::terminal;
use super::test_harness::{draw, drive, isolate_in, start};
use crate::app_state::OpenWindow;
use crate::notice::FOLLOW_EVERY;
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
    press(cx, window, Press::Close)
}

fn press(cx: &mut TestAppContext, window: &OpenWindow, press: Press) -> bool {
    let id = window.handle.window_id();
    let sent = cx.update(|cx| crate::notice::click(id, press, cx));
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

/// Whether the warning waits out of sight, and the terminal's area it is placed for.
fn kept(cx: &mut TestAppContext) -> (bool, Option<Rect>) {
    cx.update(|cx| {
        let report = crate::notice::notices(cx).into_iter().next().expect("a chip");
        (report.hidden, report.spot.map(|spot| spot.anchor))
    })
}

/// Time for the warning to look again at where its terminal is.
fn look(cx: &mut TestAppContext) {
    wait(cx, FOLLOW_EVERY + Duration::from_millis(10));
}

#[gpui_kit::test]
fn closing_the_terminal_a_warning_is_about_takes_it_away_and_lets_the_terminal_go(cx: &mut TestAppContext) {
    if isolate("closing_the_terminal_a_warning_is_about_takes_it_away_and_lets_the_terminal_go") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 2);
        set_notices(cx, true);
        draw(cx, &window);
        draw(cx, &window);
        lost_host(cx, &window);
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Showing));
        let closed = terminal(cx, &window, 0).downgrade();

        drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
        cx.run_until_parked();
        look(cx);
        assert!(closed.upgrade().is_none(), "the chip does not keep a closed terminal (and its shell) alive");
        assert_eq!(chip(cx).map(|c| (c.0, c.2)), Some((Phase::Leaving, false)), "its warning goes");
        wait(cx, UP + Duration::from_millis(10));
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Hidden));
    });
}

#[gpui_kit::test]
fn a_warning_waits_out_of_sight_while_its_terminal_is_off_screen(cx: &mut TestAppContext) {
    if isolate("a_warning_waits_out_of_sight_while_its_terminal_is_off_screen") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 2);
        set_notices(cx, true);
        draw(cx, &window);
        draw(cx, &window);
        lost_host(cx, &window);
        look(cx);
        assert!(!kept(cx).0, "its terminal is the selected one");

        drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
        draw(cx, &window);
        look(cx);
        assert!(kept(cx).0, "another terminal is selected: the warning waits");
        assert_eq!(chip(cx).map(|c| (c.0, c.3)), Some((Phase::Showing, true)), "it is still held");

        drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
        draw(cx, &window);
        look(cx);
        assert!(!kept(cx).0, "and comes back with its terminal");
    });
}

#[gpui_kit::test]
fn a_warning_waits_out_of_sight_under_an_overlay_and_a_copy_still_shows(cx: &mut TestAppContext) {
    if isolate("a_warning_waits_out_of_sight_under_an_overlay_and_a_copy_still_shows") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        cx.update(|cx| crate::command_palette::open_in(window, "", cx));
        cx.run_until_parked();
        draw(cx, window);
        look(cx);
        assert!(kept(cx).0, "its panel would take clicks from the palette");
        drive(cx, window, |ws, window, cx| ws.close_overlay(window, cx));
        draw(cx, window);
        wait(cx, Duration::from_secs(1));
        look(cx);
        assert!(!kept(cx).0);
    });
}

#[gpui_kit::test]
fn a_warning_follows_its_terminal_when_the_layout_moves_it(cx: &mut TestAppContext) {
    if isolate("a_warning_follows_its_terminal_when_the_layout_moves_it") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        let before = kept(cx).1.expect("placed");
        drive(cx, window, |ws, _, cx| ws.toggle_sidebar(cx));
        draw(cx, window);
        draw(cx, window);
        look(cx);
        let after = kept(cx).1.expect("placed");
        assert_ne!(before, after, "the sidebar left, the terminal moved");
        assert_eq!(after, pane(cx, window), "and the warning is placed for where it is now");
    });
}

/// The first terminal as the last frame drew it.
fn pane(cx: &mut TestAppContext, window: &OpenWindow) -> Rect {
    let bounds = terminal(cx, window, 0).read_with(cx, |t, _| t.pane_bounds());
    Rect::new(
        f32::from(bounds.origin.x),
        f32::from(bounds.origin.y),
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    )
}

#[gpui_kit::test]
fn a_held_warning_takes_the_look_the_system_changes_to(cx: &mut TestAppContext) {
    if isolate("a_held_warning_takes_the_look_the_system_changes_to") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        let looks = |cx: &mut TestAppContext| {
            cx.update(|cx| crate::notice::notices(cx).into_iter().next().and_then(|r| r.look))
        };
        let (theme, opaque) = looks(cx).expect("a look");
        assert!(!opaque);
        let system = cx.update(|cx| cx.global::<crate::settings::SettingsStore>().system());
        let flipped = crate::settings::SystemLook { reduce_transparency: true, ..system };
        cx.update(|cx| crate::settings::SettingsStore::set_system(cx, flipped));
        cx.run_until_parked();
        look(cx);
        assert_eq!(looks(cx), Some((theme, true)), "Reduce Transparency: the chip is made again, plain");
    });
}

/// The buttons the chip shows and whether the explanation is open.
fn buttons(cx: &mut TestAppContext) -> (Buttons, bool) {
    cx.update(|cx| {
        let report = crate::notice::notices(cx).into_iter().next().expect("a chip");
        (report.buttons, report.info_open)
    })
}

/// The explanation as the platform was asked to show it, as lines.
fn explanation() -> Option<Vec<String>> {
    crate::platform::record::open_popover().map(|popover| popover.outline())
}

#[gpui_kit::test]
fn a_warning_has_a_cross_and_an_info_and_a_copy_has_neither(cx: &mut TestAppContext) {
    if isolate("a_warning_has_a_cross_and_an_info_and_a_copy_has_neither") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        assert_eq!(buttons(cx).0, Buttons { info: true, close: true });
        copy(cx, window, "twelve chars");
        assert_eq!(buttons(cx).0, Buttons::NONE, "a copy's words over the warning have none");
        wait(cx, LIFE + Duration::from_millis(10));
        assert_eq!(buttons(cx).0, Buttons { info: true, close: true }, "and they come back with the warning");
    });
}

#[gpui_kit::test]
fn the_cross_closes_the_warning_and_the_info_and_the_body_open_the_explanation(cx: &mut TestAppContext) {
    if isolate("the_cross_closes_the_warning_and_the_info_and_the_body_open_the_explanation") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        assert!(press(cx, window, Press::Info));
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Showing), "the ⓘ does not close it");
        assert!(buttons(cx).1, "the explanation is open");
        let lines = explanation().expect("shown");
        assert_eq!(lines[1], "## This terminal can't outlive Wormhole");
        assert!(
            lines.iter().any(|line| line == "code: couldn't start the helper: spawn failed"),
            "the error's own words: {lines:?}"
        );

        // The ⓘ again closes it.
        wait(cx, Duration::from_secs(1));
        assert!(press(cx, window, Press::Info));
        assert!(!buttons(cx).1, "closed again");
        assert_eq!(explanation(), None);

        // A click on the capsule opens it, and does not close the warning either.
        wait(cx, Duration::from_secs(1));
        assert!(press(cx, window, Press::Body));
        assert!(buttons(cx).1);
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Showing), "the body no longer closes it");

        // The × closes the warning, and the explanation with it.
        assert!(press(cx, window, Press::Close));
        assert_eq!(chip(cx).map(|c| c.0), Some(Phase::Leaving));
        assert_eq!(explanation(), None, "nothing is left open under a chip that is gone");
    });
}

#[gpui_kit::test]
fn the_explanation_goes_when_its_terminal_does(cx: &mut TestAppContext) {
    if isolate("the_explanation_goes_when_its_terminal_does") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 2);
        set_notices(cx, true);
        draw(cx, &window);
        draw(cx, &window);
        lost_host(cx, &window);
        assert!(press(cx, &window, Press::Info));
        assert!(explanation().is_some());
        drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
        cx.run_until_parked();
        look(cx);
        assert_eq!(explanation(), None);
    });
}

#[gpui_kit::test]
fn the_explanation_closes_when_the_chip_is_not_where_it_pointed(cx: &mut TestAppContext) {
    if isolate("the_explanation_closes_when_the_chip_is_not_where_it_pointed") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 2);
        set_notices(cx, true);
        draw(cx, &window);
        draw(cx, &window);
        lost_host(cx, &window);

        // Another terminal is selected: the chip waits out of sight, and the explanation with it.
        assert!(press(cx, &window, Press::Info));
        assert!(explanation().is_some());
        drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
        draw(cx, &window);
        look(cx);
        assert_eq!(explanation(), None, "its terminal is off screen");
        assert!(!buttons(cx).1);

        // Back, and a copy's words take the chip over: the arrow would point at words that are not the warning.
        drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
        draw(cx, &window);
        look(cx);
        assert!(press(cx, &window, Press::Info));
        assert!(explanation().is_some());
        copy(cx, &window, "twelve chars");
        assert_eq!(explanation(), None, "a copy has no ⓘ");
    });
}

#[gpui_kit::test]
fn a_click_right_after_the_explanation_closed_by_itself_does_not_open_it_again(cx: &mut TestAppContext) {
    if isolate("a_click_right_after_the_explanation_closed_by_itself_does_not_open_it_again") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        assert!(press(cx, window, Press::Info));
        assert!(buttons(cx).1);

        // A click outside the popover (on the ⓘ, say) closes it, as AppKit does; the press that did it
        // reaches the button a moment after.
        let callbacks = crate::platform::record::popover_close(true).expect("it is open");
        cx.update(|cx| {
            window.handle.update(cx, |_, window, cx| callbacks.iter().for_each(|run| run(window, cx))).ok();
        });
        cx.run_until_parked();
        assert!(!buttons(cx).1, "closed by itself");
        assert!(press(cx, window, Press::Info));
        assert!(!buttons(cx).1, "the press that closed it does not open it again");

        // A press of its own, later, does.
        wait(cx, Duration::from_millis(400));
        assert!(press(cx, window, Press::Info));
        assert!(buttons(cx).1);
    });
}

#[gpui_kit::test]
fn closing_the_window_closes_the_explanation(cx: &mut TestAppContext) {
    if isolate("closing_the_window_closes_the_explanation") {
        return;
    }
    with_chip(cx, |cx, window| {
        lost_host(cx, window);
        assert!(press(cx, window, Press::Info));
        assert!(explanation().is_some());
        cx.update(|cx| window.handle.update(cx, |_, window, _| window.remove_window()).ok());
        cx.run_until_parked();
        assert_eq!(explanation(), None);
        assert!(cx.update(|cx| crate::notice::notices(cx).is_empty()), "and the chip with it");
    });
}
