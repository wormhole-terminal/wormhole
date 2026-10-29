//! The notice chip in a real window (GPUI's test platform; see `test_harness`): a copy says so with the
//! Notices flag on and says nothing with it off, a program's clipboard write is said by the chip (or, with the
//! flag off, by the pane bar), never both, the folder card's Copy path and a refused Put at Prompt say so on
//! the chip, each window has its chip, which comes out of the terminal it is about and goes with
//! its window. The panel itself is native and not made in a test, so what is read is the chip's rules and
//! where it was put (`notice::notices`), which run all the same, on GPUI's clock.

use std::time::Duration;

use gpui_kit::TestAppContext;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::motion::notice::{LIFE, UP};
use wormhole_core::view::notice::{Copied, Phase};
use wormhole_core::view::panes::Rect;
use wormhole_text::{Language, with_language};

use super::Workspace;
use super::reported_tests::terminal;
use super::test_harness::{draw, drive, isolate_in, read, start, with_window};
use crate::app_state::{OpenWindow, app};
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

fn store(cx: &mut TestAppContext, window: &OpenWindow, text: &str, focused: bool) {
    store_in_terminal(cx, window, 0, text, focused);
}

fn store_in_terminal(cx: &mut TestAppContext, window: &OpenWindow, ix: usize, text: &str, focused: bool) {
    let terminal = terminal(cx, window, ix);
    cx.update(|cx| terminal.update(cx, |t, cx| t.store_clipboard_for_test(text, focused, cx)));
    cx.run_until_parked();
}

fn bar(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) -> Option<String> {
    let terminal = terminal(cx, window, ix);
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
        assert_eq!(bar(cx, &window, 0), None, "never both");
    });
}

#[gpui_kit::test]
fn a_terminal_the_person_is_not_typing_in_says_which_program_copied_on_its_chip(cx: &mut TestAppContext) {
    if isolate("a_terminal_the_person_is_not_typing_in_says_which_program_copied_on_its_chip") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, true);
        // No program is known in front: the words say "a program".
        store(cx, &window, "twelve chars", false);
        assert_eq!(chips(cx), [(Phase::Showing, "A program copied 12 characters".to_string())]);
        assert_eq!(bar(cx, &window, 0), None, "the chip, not the bar");
        // One is: its name, as the sidebar shows it, and the lines when there are several.
        cx.executor().advance_clock(LIFE + UP + Duration::from_millis(50));
        cx.run_until_parked();
        let terminal = terminal(cx, &window, 0);
        cx.update(|cx| terminal.update(cx, |t, _| t.set_foreground_for_test("ssh -p 22 host")));
        store(cx, &window, "a\nb\nc", false);
        assert_eq!(chips(cx), [(Phase::Showing, "ssh copied 3 lines".to_string())]);
        // In the terminal the person is typing in it is a copy like any other.
        store(cx, &window, "twelve chars", true);
        assert_eq!(chips(cx), [(Phase::Showing, "Copied 12 characters".to_string())]);
    });
}

#[gpui_kit::test]
fn a_program_name_with_control_characters_is_cleaned_before_it_is_said(cx: &mut TestAppContext) {
    if isolate("a_program_name_with_control_characters_is_cleaned_before_it_is_said") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, true);
        let terminal = terminal(cx, &window, 0);
        let long = "x".repeat(200);
        cx.update(|cx| {
            terminal.update(cx, |t, _| t.set_foreground_for_test(&format!("a\u{7}\u{202E}b{long}")))
        });
        store(cx, &window, "abc", false);
        let said = chips(cx).remove(0).1;
        assert!(said.starts_with("ab"), "{said}");
        assert!(said.chars().all(|c| !c.is_control() && c != '\u{202E}'), "{said:?}");
        assert!(said.chars().count() < 80, "cut: {said}");
    });
}

#[gpui_kit::test]
fn a_terminal_that_is_not_on_screen_says_nothing_instead_of_a_chip_on_another_terminal(
    cx: &mut TestAppContext,
) {
    if isolate("a_terminal_that_is_not_on_screen_says_nothing_instead_of_a_chip_on_another_terminal") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 2);
        set_notices(cx, true);
        draw(cx, &window);
        draw(cx, &window);
        // The second terminal is not selected, so nothing shows it.
        store_in_terminal(cx, &window, 1, "twelve chars", false);
        assert!(chips(cx).is_empty(), "no chip, in no window");
        assert_eq!(bar(cx, &window, 1), None, "and no bar in the terminal that wrote");
        // Once it is the one on screen, a write is said on the chip.
        drive(cx, &window, |ws, _, cx| ws.select(1, cx));
        store_in_terminal(cx, &window, 1, "twelve chars", false);
        assert_eq!(chips(cx), [(Phase::Showing, "A program copied 12 characters".to_string())]);
    });
}

#[gpui_kit::test]
fn a_program_driving_a_terminal_that_is_not_on_screen_still_shows_for_the_window(cx: &mut TestAppContext) {
    use wormhole_core::view::notice::{Driven, Driver, Notice};
    if isolate("a_program_driving_a_terminal_that_is_not_on_screen_still_shows_for_the_window") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 2);
        set_notices(cx, true);
        draw(cx, &window);
        draw(cx, &window);
        let hidden = pane(cx, &window, 1);
        let selected = pane(cx, &window, 0);
        assert!(hidden.w == 0.0 && selected.w > 0.0, "only the selected terminal was drawn");
        let driven = Notice::Driven(Driven::Typing(Driver::Scripts));
        drive(cx, &window, move |ws, _, cx| {
            let tab = ws.sessions[1].id;
            ws.control_driven(tab, driven, cx);
        });
        assert_eq!(chips(cx), [(Phase::Showing, "A script is typing".to_string())], "the marker is not lost");
        assert_eq!(spot_of(cx), selected, "at the window's own place, not at the hidden terminal's");
    });
}

#[gpui_kit::test]
fn a_notice_about_a_closed_terminal_is_dropped_unless_a_program_was_driving_it(cx: &mut TestAppContext) {
    use wormhole_core::view::notice::{Driven, Driver, Notice};

    use crate::notice::{Anchor, Where};
    if isolate("a_notice_about_a_closed_terminal_is_dropped_unless_a_program_was_driving_it") {
        return;
    }
    with_window_size(cx, |cx, window| {
        let gone = || -> Option<Anchor> { Some(std::rc::Rc::new(|_: &gpui_kit::App| Where::Gone)) };
        with_window(cx, window, |window, cx| crate::notice::post(window, Copied::Command, gone(), cx));
        cx.run_until_parked();
        assert!(chips(cx).is_empty(), "a copy about a terminal that is gone says nothing");
        let driven = Notice::Driven(Driven::Reading(Driver::Scripts));
        with_window(cx, window, |window, cx| crate::notice::post(window, driven, gone(), cx));
        cx.run_until_parked();
        assert_eq!(chips(cx), [(Phase::Showing, "A script is reading".to_string())]);
        assert_eq!(spot_of(cx), pane(cx, window, 0));
    });
}

#[gpui_kit::test]
fn with_the_flag_off_the_bar_names_the_program_in_the_catalogs_words(cx: &mut TestAppContext) {
    if isolate("with_the_flag_off_the_bar_names_the_program_in_the_catalogs_words") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, false);
        store(cx, &window, "x", true);
        assert!(chips(cx).is_empty());
        assert_eq!(bar(cx, &window, 0).as_deref(), Some("A program copied 1 character"));
        let terminal = terminal(cx, &window, 0);
        cx.update(|cx| terminal.update(cx, |t, _| t.set_foreground_for_test("ssh -p 22 host")));
        store(cx, &window, "a\nb\nc", false);
        assert_eq!(bar(cx, &window, 0).as_deref(), Some("ssh copied 3 lines"));
    });
}

#[gpui_kit::test]
fn the_cards_copy_path_keeps_its_own_feedback_and_the_menus_uses_the_chip(cx: &mut TestAppContext) {
    if isolate("the_cards_copy_path_keeps_its_own_feedback_and_the_menus_uses_the_chip") {
        return;
    }
    with_window_size(cx, |cx, window| {
        let clipboard =
            |cx: &mut TestAppContext| cx.update(|cx| cx.read_from_clipboard().and_then(|item| item.text()));
        cx.update(|cx| Workspace::copy_path_from_card(std::path::Path::new("/tmp/card"), cx));
        cx.run_until_parked();
        assert_eq!(clipboard(cx).as_deref(), Some("/tmp/card"));
        assert!(chips(cx).is_empty(), "the button says Copied where it was clicked: no chip over the card");
        #[cfg(feature = "devctl")]
        assert_eq!(said_aloud(), ["Path copied"]);

        drive(cx, window, |ws, _, cx| ws.copy_path_from_menu(std::path::Path::new("/tmp/tube"), cx));
        assert_eq!(chips(cx), [(Phase::Showing, "Copied path".to_string())]);
        assert_eq!(spot_of(cx), pane(cx, window, 0), "on the terminal the menu is about");
        assert_eq!(clipboard(cx).as_deref(), Some("/tmp/tube"));
    });
}

/// What was announced to VoiceOver since the last call (devctl keeps the last few).
#[cfg(feature = "devctl")]
fn said_aloud() -> Vec<String> {
    let said = crate::platform::a11y::recent().into_iter().map(|(text, _)| text).collect();
    crate::platform::a11y::forget_recent();
    said
}

/// A finished command whose text was drawn concealed (SGR 8), then the next prompt, and Put Command at Prompt
/// on it: refused.
fn refuse_at_prompt(cx: &mut TestAppContext, window: &OpenWindow) {
    let terminal = terminal(cx, window, 0);
    let marks = b"\x1b]133;A\x07$ \x1b]133;B\x07\x1b[8mrm\x1b[0m\r\x1b]133;C\x07x\r\x1b]133;D;0\x07\x1b]133;A\x07$ \r";
    cx.update(|cx| terminal.update(cx, |t, _| t.write_input(marks.to_vec())));
    let until = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        if cx.update(|cx| terminal.read(cx).ready_to_put_for_test()) {
            break;
        }
        assert!(std::time::Instant::now() < until, "the finished block and the prompt");
        std::thread::sleep(Duration::from_millis(10));
    }
    let found = cx.update(|cx| terminal.update(cx, |t, cx| t.put_first_block_at_prompt_for_test(cx)));
    assert!(found, "the block");
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_command_that_is_refused_at_the_prompt_says_why_on_the_chip(cx: &mut TestAppContext) {
    if isolate("a_command_that_is_refused_at_the_prompt_says_why_on_the_chip") {
        return;
    }
    with_window_size(cx, |cx, window| {
        refuse_at_prompt(cx, window);
        assert_eq!(
            chips(cx),
            [(Phase::Showing, "Not put at the prompt: part of it is concealed".to_string())]
        );
        assert_eq!(bar(cx, window, 0), None, "the chip, not the bar");
    });
}

#[gpui_kit::test]
fn with_the_chip_off_a_refused_put_at_the_prompt_is_in_the_bar_and_said_aloud(cx: &mut TestAppContext) {
    if isolate("with_the_chip_off_a_refused_put_at_the_prompt_is_in_the_bar_and_said_aloud") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, false);
        draw(cx, &window);
        draw(cx, &window);
        #[cfg(feature = "devctl")]
        said_aloud();
        refuse_at_prompt(cx, &window);
        let words = "Not put at the prompt: part of it is concealed";
        assert!(chips(cx).is_empty());
        assert_eq!(bar(cx, &window, 0).as_deref(), Some(words));
        #[cfg(feature = "devctl")]
        assert_eq!(said_aloud(), [words]);
    });
}

#[gpui_kit::test]
fn with_the_chip_off_the_menus_copy_path_is_said_aloud(cx: &mut TestAppContext) {
    if isolate("with_the_chip_off_the_menus_copy_path_is_said_aloud") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, false);
        #[cfg(feature = "devctl")]
        said_aloud();
        drive(cx, &window, |ws, _, cx| ws.copy_path_from_menu(std::path::Path::new("/tmp/tube"), cx));
        assert!(chips(cx).is_empty());
        #[cfg(feature = "devctl")]
        assert_eq!(said_aloud(), ["Copied path"]);
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

/// The terminal `ix` of `window`, as the last frame drew it.
fn pane(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) -> Rect {
    let terminal = terminal(cx, window, ix);
    let bounds = cx.update(|cx| terminal.read(cx).pane_bounds());
    Rect::new(
        f32::from(bounds.origin.x),
        f32::from(bounds.origin.y),
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    )
}

fn spot_of(cx: &mut TestAppContext) -> Rect {
    cx.update(|cx| {
        crate::notice::notices(cx).first().and_then(|r| r.spot).expect("a chip with a place").anchor
    })
}

#[gpui_kit::test]
fn the_chip_is_about_the_terminal_that_copied_and_about_the_selected_one_otherwise(cx: &mut TestAppContext) {
    if isolate("the_chip_is_about_the_terminal_that_copied_and_about_the_selected_one_otherwise") {
        return;
    }
    with_window_size(cx, |cx, window| {
        let expected = pane(cx, window, 0);
        assert!(expected.w > 0.0 && expected.h > 0.0, "the first terminal was drawn");
        store(cx, window, "from the terminal", true);
        assert_eq!(spot_of(cx), expected, "a terminal's own write: its pane");
        // A notice of the window's own (a path from a menu): the selected terminal's pane.
        cx.executor().advance_clock(LIFE + UP + Duration::from_millis(50));
        cx.run_until_parked();
        with_window(cx, window, |window, cx| crate::notice::post(window, Copied::Path, None, cx));
        cx.run_until_parked();
        assert_eq!(spot_of(cx), pane(cx, window, 0));
    });
}

/// A window of two terminals, drawn.
fn with_window_size(cx: &mut TestAppContext, body: impl FnOnce(&mut TestAppContext, &OpenWindow)) {
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        set_notices(cx, true);
        draw(cx, &window);
        draw(cx, &window);
        body(cx, &window);
    });
}

#[gpui_kit::test]
fn a_chip_goes_up_when_its_life_is_over_and_is_gone_when_it_is_out_of_sight(cx: &mut TestAppContext) {
    if isolate("a_chip_goes_up_when_its_life_is_over_and_is_gone_when_it_is_out_of_sight") {
        return;
    }
    with_window_size(cx, |cx, window| {
        store(cx, window, "twelve chars", true);
        assert_eq!(chips(cx)[0].0, Phase::Showing);
        cx.executor().advance_clock(LIFE - Duration::from_millis(10));
        cx.run_until_parked();
        assert_eq!(chips(cx)[0].0, Phase::Showing, "not yet");
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        assert_eq!(chips(cx)[0].0, Phase::Leaving);
        cx.executor().advance_clock(UP + Duration::from_millis(10));
        cx.run_until_parked();
        assert_eq!(chips(cx)[0].0, Phase::Hidden);
        // And the same words again, now that the two seconds have passed, are said again.
        store(cx, window, "twelve chars", true);
        assert_eq!(chips(cx)[0].0, Phase::Showing);
    });
}

#[gpui_kit::test]
fn each_window_has_its_own_chip_and_a_closed_windows_goes_with_it(cx: &mut TestAppContext) {
    if isolate("each_window_has_its_own_chip_and_a_closed_windows_goes_with_it") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let first = start(cx, 2);
        set_notices(cx, true);
        drive(cx, &first, |ws, window, cx| ws.tear_off_at(2, 600.0, 300.0, window, cx));
        cx.run_until_parked();
        let windows = cx.update(|cx| app(cx).windows.clone());
        assert_eq!(windows.len(), 2);
        let second = windows.into_iter().find(|w| w.id != first.id).expect("a second window");
        draw(cx, &first);
        draw(cx, &second);
        store(cx, &first, "abc", true);
        let terminal = read(cx, &second, |ws| ws.sessions[0].terminal().clone());
        cx.update(|cx| terminal.update(cx, |t, cx| t.store_clipboard_for_test("abcdef\nx", true, cx)));
        cx.run_until_parked();
        let mut said: Vec<String> = chips(cx).into_iter().map(|(_, words)| words).collect();
        said.sort();
        assert_eq!(said, ["Copied 2 lines", "Copied 3 characters"], "one each, in its own window");

        cx.update(|cx| second.handle.update(cx, |_, window, _| window.remove_window()).ok());
        cx.run_until_parked();
        assert_eq!(
            chips(cx),
            [(Phase::Showing, "Copied 3 characters".to_string())],
            "the closed one's is gone"
        );
    });
}
