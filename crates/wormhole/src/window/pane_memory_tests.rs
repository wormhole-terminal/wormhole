//! View tests of the remembered side columns: a window opened with no saved layout starts with
//! the columns as they were last left, app-wide (see `view_tests` for how each test runs in a
//! child process with scratch data).
use gpui_kit::{TestAppContext, px, size};
use wormhole_core::storage::store::Store;
use wormhole_core::view::layout::{Panes, REMEMBERED_PANES_KEY, RememberedPanes, WINDOW_MIN_WIDTH};

use crate::app_state::{OpenWindow, app, app_mut};
use crate::workspace::view_tests::{draw, drive, isolate_in, read, start};
use crate::workspace::window_lifecycle::open_workspace_window;
use crate::workspace::{RightTab, Workspace};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window opened the way the Dock icon does with none open: no record.
fn open_blank_window(cx: &mut TestAppContext) -> OpenWindow {
    cx.update(|cx| open_workspace_window(None, None, cx));
    cx.run_until_parked();
    cx.update(|cx| app(cx).windows.last().cloned()).expect("the window opened")
}

fn panes(cx: &mut TestAppContext, window: &OpenWindow) -> Panes {
    read(cx, window, |ws: &Workspace| ws.panes)
}

#[gpui_kit::test]
fn a_first_window_has_the_sidebar_open_and_the_panel_closed(cx: &mut TestAppContext) {
    if isolate("a_first_window_has_the_sidebar_open_and_the_panel_closed") {
        return;
    }
    // Nothing remembered: the app's own start, not `Panes::default()` (the panel wanted).
    let first = start(cx, 1);
    let blank = open_blank_window(cx);
    let blank = panes(cx, &blank);
    assert!(blank.sidebar_open && !blank.panel_wanted, "{blank:?}");
    assert_eq!(panes(cx, &first), Panes::default(), "a window with a record is as that says");
}

#[gpui_kit::test]
fn a_window_without_a_layout_starts_as_the_columns_were_last_left(cx: &mut TestAppContext) {
    if isolate("a_window_without_a_layout_starts_as_the_columns_were_last_left") {
        return;
    }
    let first = start(cx, 1);
    // Close the sidebar, open the panel on another tab.
    drive(cx, &first, |ws, _, cx| ws.toggle_sidebar(cx));
    drive(cx, &first, |ws, _, cx| ws.show_tab(RightTab::Changes, cx));
    let blank = open_blank_window(cx);
    let started = panes(cx, &blank);
    assert!(!started.sidebar_open && started.panel_wanted, "{started:?}");

    // Closing the panel is remembered the same way, and the sidebar back.
    drive(cx, &first, |ws, window, cx| ws.toggle_notes(window, cx));
    drive(cx, &first, |ws, _, cx| ws.toggle_sidebar(cx));
    let blank = open_blank_window(cx);
    let started = panes(cx, &blank);
    assert!(started.sidebar_open && !started.panel_wanted, "{started:?}");

    // A saved window keeps its own layout whatever was remembered since.
    assert_eq!(panes(cx, &first), started);
    drive(cx, &first, |ws, window, cx| ws.toggle_notes(window, cx));
    assert!(read(cx, &blank, |ws| !ws.panes.panel_wanted), "another window is not followed");
}

#[gpui_kit::test]
fn a_panel_hidden_for_room_is_remembered_as_wanted(cx: &mut TestAppContext) {
    if isolate("a_panel_hidden_for_room_is_remembered_as_wanted") {
        return;
    }
    let window = start(cx, 1);
    cx.simulate_window_resize(window.handle, size(px(WINDOW_MIN_WIDTH + 40.0), px(900.0)));
    draw(cx, &window);
    read(cx, &window, |ws| {
        assert!(ws.panes.panel_wanted && !ws.notes_visible(), "wanted, with no room for it");
    });
    // Any change of the columns writes what was wanted, not what was drawn.
    drive(cx, &window, |ws, _, cx| ws.remember_panes(cx));
    let remembered = cx.update(|cx| app(cx).last_panes);
    assert!(remembered.panel_open, "{remembered:?}");
}

#[gpui_kit::test]
fn the_columns_are_kept_in_the_store(cx: &mut TestAppContext) {
    if isolate("the_columns_are_kept_in_the_store") {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| app_mut(cx).store = Some(Store::open().expect("opens the scratch database")));
    let stored = |cx: &mut TestAppContext| {
        let json = cx.update(|cx| app(cx).store.as_ref().and_then(|s| s.meta(REMEMBERED_PANES_KEY).ok()));
        json.flatten().map(|json| RememberedPanes::parse(Some(&json)))
    };
    assert_eq!(stored(cx), None, "nothing is written until a column changes");
    drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
    let after = stored(cx).expect("written when the sidebar was toggled");
    assert!(!after.sidebar_open && after.panel_open, "{after:?}");
    // The same state again writes nothing new (compared, not timed: the value is what it was).
    drive(cx, &window, |ws, _, cx| ws.remember_panes(cx));
    assert_eq!(stored(cx), Some(after));
}
