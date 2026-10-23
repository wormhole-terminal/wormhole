//! What closing the settings window must undo at that moment, not at the next opening: the page's
//! release puts the global hotkey back if its field was recording, and the app's tick stops reading the
//! notification permission for a window that is gone. Both hang on the window being held weakly.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::{AnyWindowHandle, TestAppContext};

use super::recorder::Recording;
use super::{Section, current_window, open_settings};
use crate::app_state::app;
use crate::window::test_harness::{isolate_in, start};

fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| open_settings(Some(Section::MenuBar), cx));
    cx.run_until_parked();
    cx.update(|cx| current_window(cx)).expect("the settings window opened").0
}

fn close(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update(|cx| handle.update(cx, |_, window, _| window.remove_window())).expect("open");
    cx.run_until_parked();
}

#[gpui_kit::test]
fn closing_settings_while_the_hotkey_records_puts_the_hotkey_back(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "closing_settings_while_the_hotkey_records_puts_the_hotkey_back") {
        return;
    }
    start(cx, 1);
    // The app registers the hotkey as the setting says, once, on its tick.
    cx.update(|cx| {
        crate::hotkey::install(smol::channel::unbounded().0, cx);
        let profile = app(cx).active_profile().clone();
        assert!(crate::hotkey::sync("alt-space", &profile, cx), "applied");
    });
    let handle = open(cx);
    // The page is let go of at once: a handle kept here would keep the window's page alive.
    cx.update(|cx| {
        let (_, page) = current_window(cx).expect("open");
        page.update(cx, |page, cx| {
            let recording = Recording::Keys { id: "hotkey", held: Default::default() };
            page.set_recording(Some(recording), cx);
        })
    });
    let (paused, _) = cx.update(|cx| crate::hotkey::state_for_test(cx));
    assert!(paused, "the hotkey's field records, so the hotkey is held off");

    close(cx, handle);

    let (paused, registered) = cx.update(|cx| crate::hotkey::state_for_test(cx));
    assert!(!paused, "the window is gone: the hotkey is no longer held off");
    // What the status item shows follows what is registered (a test cannot register a Carbon key off
    // the main thread, so either may be so, but the two agree).
    assert_eq!(cx.update(|cx| crate::hotkey::shown(cx)).is_some(), registered);
    assert!(cx.update(|cx| current_window(cx)).is_none(), "the closed window is not the current one");
}

#[gpui_kit::test]
fn the_permission_is_not_read_again_for_a_settings_window_that_is_gone(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_permission_is_not_read_again_for_a_settings_window_that_is_gone") {
        return;
    }
    start(cx, 1);
    let refreshes = |cx: &mut TestAppContext| cx.update(|cx| crate::notifications::permission_refreshes(cx));
    let handle = open(cx);

    let before = refreshes(cx);
    cx.update(|cx| crate::notifications::tick(5, cx));
    assert_eq!(refreshes(cx), before + 1, "an open Settings keeps the permission current");

    close(cx, handle);

    let before = refreshes(cx);
    cx.update(|cx| crate::notifications::tick(5, cx));
    cx.update(|cx| crate::notifications::tick(10, cx));
    assert_eq!(refreshes(cx), before, "no Settings window, no reading");

    // And opening it again works, with the window a new one.
    let again = open(cx);
    assert_ne!(again, handle);
    let before = refreshes(cx);
    cx.update(|cx| crate::notifications::tick(15, cx));
    assert_eq!(refreshes(cx), before + 1);
}
