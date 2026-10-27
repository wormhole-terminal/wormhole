//! The unlisted key that unlocks the feature flags in a Stable release (⌃⌥⌘F): pressed in the settings
//! window it writes the unlock entry and opens the flags, pressed again it takes the entry out. Which
//! build shows the page with the entry is core's (`features::FlagMode`); a test is always a dev build.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::{AnyWindowHandle, Keystroke, TestAppContext};
use wormhole_core::config::features;

use super::{Section, current_window, open_settings};
use crate::settings::settings;
use crate::window::test_harness::{isolate_in, start};

fn press(cx: &mut TestAppContext, handle: AnyWindowHandle, keys: &str) {
    let keystroke = Keystroke::parse(keys).expect("a keystroke");
    cx.update(|cx| handle.update(cx, |_, window, cx| window.dispatch_keystroke(keystroke, cx)))
        .expect("open");
    cx.run_until_parked();
}

fn unlocked(cx: &mut TestAppContext) -> bool {
    cx.update(|cx| features::unlocked(&settings(cx).feature_flags))
}

fn section(cx: &mut TestAppContext) -> Section {
    cx.update(|cx| current_window(cx).expect("open").1.read(cx).section)
}

#[gpui_kit::test]
fn the_unlisted_key_unlocks_the_flags_and_locks_them_again(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_unlisted_key_unlocks_the_flags_and_locks_them_again") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::MenuBar), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx)).expect("the settings window opened").0;
    assert!(!unlocked(cx));

    press(cx, handle, "ctrl-alt-shift-cmd-f");
    assert!(!unlocked(cx), "only the exact keys unlock");

    press(cx, handle, "ctrl-alt-cmd-f");
    assert!(unlocked(cx), "the entry is written");
    assert_eq!(section(cx), Section::FeatureFlags, "and the flags open");

    press(cx, handle, "ctrl-alt-cmd-f");
    assert!(!unlocked(cx), "pressed again, it locks");
    assert!(
        cx.update(|cx| !settings(cx).feature_flags.contains_key(features::UNLOCKED_KEY)),
        "by taking the entry out, so the file holds only what is chosen"
    );
}
