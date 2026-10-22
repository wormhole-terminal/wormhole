//! Restore terminal text after restart turned off, and "Don't keep this terminal's text": nothing of
//! it stays in the profile's store (see `view_tests` for how each test runs in a child process with
//! scratch data; the store's key is the development build's key file there, never the Keychain).
use gpui_kit::TestAppContext;
use wormhole_core::storage::store::Store;

use crate::workspace::app::OpenWindow;
use crate::workspace::app::{app, app_mut};
use crate::workspace::view_tests::{drive, isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// Two terminals, both with saved text.
fn with_saved_text(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 2);
    cx.update(|cx| app_mut(cx).store = Some(Store::open().expect("opens the scratch database")));
    let record = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.window_record(cx))).expect("open");
    cx.update(|cx| {
        let store = app_mut(cx).store.as_mut().expect("store");
        store.save_window(&record).expect("saves");
        assert_eq!(store.save_scrollbacks(&[(1, "SECRET-MARKER-7"), (2, "two")]).expect("saves"), 2);
    });
    window
}

fn saved(cx: &mut TestAppContext) -> Vec<Option<String>> {
    cx.update(|cx| {
        let windows = app(cx).store.as_ref().expect("store").load().expect("loads");
        windows[0].sessions.iter().map(|s| s.scrollback.clone()).collect()
    })
}

#[gpui_kit::test]
fn a_terminal_whose_text_is_not_kept_has_it_erased_and_never_saved(cx: &mut TestAppContext) {
    if isolate("a_terminal_whose_text_is_not_kept_has_it_erased_and_never_saved") {
        return;
    }
    let window = with_saved_text(cx);
    drive(cx, &window, |ws, _, cx| ws.toggle_forget_text(0, cx));
    assert_eq!(saved(cx), [None, Some("two".to_string())], "erased at once");
    // The minute's save skips it; the choice is saved with the window.
    drive(cx, &window, |ws, _, cx| {
        ws.save(cx);
        ws.save_scrollback(cx);
    });
    assert_eq!(saved(cx)[0], None);
    let kept = cx.update(|cx| app(cx).store.as_ref().expect("store").load().expect("loads"));
    assert!(kept[0].sessions[0].forget_text && !kept[0].sessions[1].forget_text);
}

#[gpui_kit::test]
fn turning_restore_off_erases_the_text_and_stops_saving(cx: &mut TestAppContext) {
    if isolate("turning_restore_off_erases_the_text_and_stops_saving") {
        return;
    }
    let window = with_saved_text(cx);
    cx.update(crate::workspace::saved_text::init);
    assert_eq!(saved(cx), [Some("SECRET-MARKER-7".to_string()), Some("two".to_string())], "on: kept");
    cx.update(|cx| crate::workspace::settings_page::update_settings(cx, |s| s.restore_terminal_text = false));
    cx.run_until_parked();
    assert_eq!(saved(cx), [None, None]);
    drive(cx, &window, |ws, _, cx| ws.save_scrollback(cx));
    let rows = cx.update(|cx| app(cx).store.as_ref().expect("store").erase_saved_text().expect("erases"));
    assert_eq!(rows, 0, "nothing was saved while it is off");
}

#[gpui_kit::test]
fn the_erase_runs_once_per_turn_off_not_on_every_settings_change(cx: &mut TestAppContext) {
    if isolate("the_erase_runs_once_per_turn_off_not_on_every_settings_change") {
        return;
    }
    let _window = with_saved_text(cx);
    cx.update(crate::workspace::saved_text::init);
    cx.update(|cx| crate::workspace::settings_page::update_settings(cx, |s| s.restore_terminal_text = false));
    cx.run_until_parked();
    assert_eq!(saved(cx), [None, None]);
    // Written behind the setting's back: only a new turn-off, a launch or a profile switch erases.
    cx.update(|cx| {
        let store = app_mut(cx).store.as_mut().expect("store");
        assert_eq!(store.save_scrollbacks(&[(1, "late")]).expect("saves"), 1);
    });
    cx.update(|cx| crate::workspace::settings_page::update_settings(cx, |s| s.minimap = !s.minimap));
    cx.run_until_parked();
    assert_eq!(saved(cx)[0].as_deref(), Some("late"), "another setting changed: not erased again");
}
