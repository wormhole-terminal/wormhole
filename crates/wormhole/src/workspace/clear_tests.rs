//! Clearing a terminal (⌘K) erases its saved text from its profile's database at once (see
//! `view_tests` for how each test runs in a child process with scratch data).
use gpui_kit::TestAppContext;
use wormhole_core::store::Store;

use super::app::{app, app_mut};
use super::view_tests::{drive, isolate_in, start};
use crate::terminal_view::TerminalEvent;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

#[gpui_kit::test]
fn clearing_a_terminal_erases_its_saved_text_from_the_store(cx: &mut TestAppContext) {
    if isolate("clearing_a_terminal_erases_its_saved_text_from_the_store") {
        return;
    }
    let window = start(cx, 2);
    cx.update(|cx| app_mut(cx).store = Some(Store::open().expect("opens the scratch database")));
    let record = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.window_record(cx))).expect("open");
    cx.update(|cx| {
        let store = app_mut(cx).store.as_mut().expect("store");
        store.save_window(&record).expect("saves");
        store.save_scrollbacks(&[(1, "SECRET-MARKER-3"), (2, "kept")]).expect("saves");
    });

    drive(cx, &window, |ws, window, cx| ws.on_terminal_event(1, TerminalEvent::Cleared, window, cx));
    let saved = cx.update(|cx| {
        let windows = app(cx).store.as_ref().expect("store").load().expect("loads");
        windows[0].sessions.iter().map(|s| s.scrollback.clone()).collect::<Vec<_>>()
    });
    assert_eq!(saved, [None, Some("kept".to_string())]);
}
