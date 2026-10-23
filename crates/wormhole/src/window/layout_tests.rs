//! What sits around the terminal in a real window: the status bar's count of other windows and the panel's
//! edge beside the inspector. The harness is `test_harness`.

use gpui_kit::{TestAppContext, px, rems, size};
use wormhole_core::storage::store::{SessionRecord, WindowRecord};

use crate::app_state::app;
use crate::settings::SettingsStore;
use crate::window::status_bar::StatusItem;
use crate::window::test_harness::{draw, drive, hook, isolate_in, own_window, read, start, with_window};
use crate::window::window_lifecycle::open_workspace_window;

/// The status bar counts the agents of every window without its frame reading the others: GPUI
/// makes an entity read while a window draws that window's, so another window's workspace would
/// run its tick there, and lose it when that window closed.
#[gpui_kit::test]
fn the_status_bar_counts_other_windows_without_taking_them_over(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_status_bar_counts_other_windows_without_taking_them_over") {
        return;
    }
    let first = start(cx, 1);
    cx.update(|cx| SettingsStore::update(cx, |s| s.status_bar = true));
    cx.update(|cx| {
        let record =
            WindowRecord { id: 2, sessions: vec![SessionRecord::new(9, None, None)], ..Default::default() };
        open_workspace_window(Some(record), None, cx);
    });
    cx.run_until_parked();
    let second =
        cx.update(|cx| app(cx).windows.iter().find(|w| w.id == 2).cloned()).expect("the second window");
    hook(cx, &second, 9, "UserPromptSubmit", None);
    drive(cx, &first, |ws, _, cx| ws.refresh_status_bar(cx));
    draw(cx, &second);
    draw(cx, &first);
    read(cx, &first, |ws| {
        assert!(ws.status_items().contains(&StatusItem::Working(9)), "the other window's agent is counted")
    });

    // The window that drew last goes; the other keeps its own window (and so its tick).
    cx.update(|cx| first.handle.update(cx, |_, window, _| window.remove_window()))
        .expect("the window is open");
    cx.run_until_parked();
    assert_eq!(own_window(cx, &second), Some(second.handle.window_id()));
}

/// With the debug Inspector open, GPUI lays the workspace out in the window less the Inspector's
/// panel: the right panel's edge, its handle and its fit follow that column, not the window.
#[cfg(any(feature = "inspector", debug_assertions))]
#[gpui_kit::test]
fn the_panels_edge_follows_the_workspaces_column_beside_the_inspector(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_panels_edge_follows_the_workspaces_column_beside_the_inspector") {
        return;
    }
    let window = start(cx, 1);
    // Room for the panel next to the Inspector.
    cx.simulate_window_resize(window.handle, size(px(1800.0), px(900.0)));
    draw(cx, &window);
    let column = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.window_width);
    let mut drawn = gpui_kit::VisualTestContext::from_window(window.handle, cx);
    let mut handle_x =
        || f32::from(drawn.debug_bounds("panel-resize").expect("the handle is drawn").origin.x);
    let at_edge = 1800.0 - wormhole_core::view::layout::PANEL_DEFAULT - crate::style::RESIZE_HANDLE_W / 2.0;
    assert_eq!((column(cx), handle_x()), (1800.0, at_edge));

    with_window(cx, &window, |window, cx| window.toggle_inspector(cx));
    let inspector = with_window(cx, &window, |window, _| f32::from(rems(30.0).to_pixels(window.rem_size())));
    // The first frame measures the column; the next one is laid out with it.
    draw(cx, &window);
    draw(cx, &window);
    assert_eq!(column(cx), 1800.0 - inspector);
    assert_eq!(handle_x(), at_edge - inspector, "the handle sits on the panel's edge, not the Inspector's");

    with_window(cx, &window, |window, cx| window.toggle_inspector(cx));
    draw(cx, &window);
    draw(cx, &window);
    assert_eq!((column(cx), handle_x()), (1800.0, at_edge), "closing it gives the window back");
}
