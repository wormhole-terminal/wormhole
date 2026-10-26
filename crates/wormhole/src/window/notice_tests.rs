//! The notice chip in a real window (GPUI's test platform; see `test_harness`): a copy says so with the
//! Notices flag on and says nothing with it off, and a program's clipboard write is said by the chip or by
//! the pane bar, never both. The panel itself is native and not made in a test, so what is read is the
//! chip's rules (`notice::notices`), which run all the same.

use gpui_kit::TestAppContext;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::notice::Phase;
use wormhole_text::{Language, with_language};

use super::reported_tests::terminal;
use super::test_harness::{isolate_in, start};
use crate::app_state::OpenWindow;
use crate::settings::update_settings;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn set_notices(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        update_settings(cx, |s| _ = s.feature_flags.insert(FeatureFlag::Notices.key().into(), on))
    });
}

/// What every window's chip says, and how it stands.
fn chips(cx: &mut TestAppContext) -> Vec<(Phase, String)> {
    cx.update(|cx| crate::notice::notices(cx).into_iter().map(|r| (r.phase, r.words)).collect())
}

fn store(cx: &mut TestAppContext, window: &OpenWindow, text: &str, in_front: bool) {
    let terminal = terminal(cx, window, 0);
    cx.update(|cx| terminal.update(cx, |t, cx| t.store_clipboard_for_test(text, in_front, cx)));
    cx.run_until_parked();
}

fn bar(cx: &mut TestAppContext, window: &OpenWindow) -> Option<String> {
    let terminal = terminal(cx, window, 0);
    cx.update(|cx| terminal.read(cx).bar_notice_for_test())
}

#[gpui_kit::test]
fn a_copy_says_so_with_the_flag_on_and_nothing_with_it_off(cx: &mut TestAppContext) {
    if isolate("a_copy_says_so_with_the_flag_on_and_nothing_with_it_off") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        let terminal = terminal(cx, &window, 0);
        // Something on the screen to select: what the shell (`cat`) echoes.
        cx.update(|cx| terminal.update(cx, |t, _| t.write_input(b"secret-words\r".to_vec())));
        let limit = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !cx.update(|cx| terminal.read(cx).screen_text().contains("secret-words")) {
            assert!(std::time::Instant::now() < limit, "the echo");
            std::thread::sleep(std::time::Duration::from_millis(10));
            cx.run_until_parked();
        }
        let copy = |cx: &mut TestAppContext| {
            cx.update(|cx| terminal.update(cx, |t, cx| t.copy_all_for_test(cx)));
            cx.run_until_parked();
        };

        set_notices(cx, false);
        copy(cx);
        assert!(chips(cx).is_empty(), "off: no chip");

        set_notices(cx, true);
        copy(cx);
        let chips = chips(cx);
        assert_eq!(chips.len(), 1, "on: the window's chip");
        assert_eq!(chips[0].0, Phase::Showing);
        assert!(chips[0].1.starts_with("Copied "), "{:?}", chips[0].1);
        assert!(!chips[0].1.contains("secret"), "it says how much, never what");
    });
}

#[gpui_kit::test]
fn a_program_that_writes_the_clipboard_is_said_by_the_chip_and_not_the_bar(cx: &mut TestAppContext) {
    if isolate("a_program_that_writes_the_clipboard_is_said_by_the_chip_and_not_the_bar") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, true);
        store(cx, &window, "twelve chars", true);
        assert_eq!(chips(cx), [(Phase::Showing, "Copied 12 characters".to_string())]);
        assert_eq!(bar(cx, &window), None, "never both");
    });
}

#[gpui_kit::test]
fn a_terminal_nobody_is_looking_at_keeps_its_bar_notice_and_gets_no_chip(cx: &mut TestAppContext) {
    if isolate("a_terminal_nobody_is_looking_at_keeps_its_bar_notice_and_gets_no_chip") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, true);
        store(cx, &window, "twelve chars", false);
        assert!(chips(cx).is_empty());
        assert!(bar(cx, &window).is_some_and(|text| text.contains("12 characters")));
    });
}

#[gpui_kit::test]
fn with_the_flag_off_the_bar_says_what_it_always_did(cx: &mut TestAppContext) {
    if isolate("with_the_flag_off_the_bar_says_what_it_always_did") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, false);
        store(cx, &window, "x", true);
        assert!(chips(cx).is_empty());
        assert!(bar(cx, &window).is_some_and(|text| text.contains("1 character")));
    });
}

#[gpui_kit::test]
fn the_same_copy_again_at_once_is_not_said_twice_and_a_different_one_replaces_it(cx: &mut TestAppContext) {
    if isolate("the_same_copy_again_at_once_is_not_said_twice_and_a_different_one_replaces_it") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, true);
        store(cx, &window, "abc", true);
        store(cx, &window, "abc", true);
        assert_eq!(chips(cx), [(Phase::Showing, "Copied 3 characters".to_string())]);
        store(cx, &window, "abcd\nef", true);
        assert_eq!(chips(cx), [(Phase::Showing, "Copied 2 lines".to_string())], "one chip, new words");
    });
}
