//! What sits around the terminal in a real window: the status bar's count of other windows and the panel's
//! edge beside the inspector. The harness is `test_harness`.

use std::time::Duration;

use gpui_kit::{TestAppContext, px, rems, size};
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_core::view::layout::Edge;

use crate::app_state::app;
use crate::settings::SettingsStore;
use crate::style::motion::{MOTION_PANEL, MOTION_PANEL_EXIT};
use crate::terminal_view::TerminalEvent;
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

/// The test windows cut every slide (reduce motion), so this is the one place a side column slides:
/// coming in it is over the terminal and joins the layout when it lands; going out it leaves the
/// layout at once and is covered up meanwhile.
#[gpui_kit::test]
fn a_side_column_slides_over_the_terminal_and_joins_the_layout_when_it_lands(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_side_column_slides_over_the_terminal_and_joins_the_layout_when_it_lands")
    {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| cx.set_reduce_motion(false));
    cx.simulate_window_resize(window.handle, size(px(1800.0), px(900.0)));
    draw(cx, &window);
    let state = |cx: &mut TestAppContext| {
        read(cx, &window, |ws| {
            (ws.columns().panel > 0.0, ws.in_flow(Edge::Panel), ws.slide_motion().is_some())
        })
    };
    assert_eq!(state(cx), (true, true, false), "the panel is in at rest");

    drive(cx, &window, |ws, window, cx| ws.toggle_notes(window, cx));
    assert_eq!(state(cx), (false, true, true), "going out it has left the layout and is covered up");
    cx.executor().advance_clock(MOTION_PANEL_EXIT + Duration::from_millis(10));
    cx.run_until_parked();
    assert_eq!(state(cx), (false, true, false), "the slide ended");

    drive(cx, &window, |ws, window, cx| ws.toggle_notes(window, cx));
    assert_eq!(state(cx), (true, false, true), "coming in it lies over the terminal, out of the flow");
    draw(cx, &window);
    cx.executor().advance_clock(MOTION_PANEL + Duration::from_millis(10));
    cx.run_until_parked();
    assert_eq!(state(cx), (true, true, false), "landed, it joins the layout");
}

/// The bar's size item is the terminal's grid. A resize is found out by the frame that lays the grid
/// out, after the bar was built: the terminal says so (`Resized`), and the window, which draws the bar
/// from what it knew, is drawn again then, not on its tick.
#[gpui_kit::test]
fn the_status_bar_shows_a_new_grid_size_without_waiting_for_the_tick(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_status_bar_shows_a_new_grid_size_without_waiting_for_the_tick") {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| SettingsStore::update(cx, |s| s.status_bar = true));
    let settle = |cx: &mut TestAppContext| {
        for _ in 0..3 {
            draw(cx, &window);
            cx.run_until_parked();
        }
    };
    cx.simulate_window_resize(window.handle, size(px(1800.0), px(900.0)));
    settle(cx);
    let terminal = read(cx, &window, |ws| ws.sessions[0].terminal().clone());
    let resized = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = resized.clone();
    let _count = cx.update(|cx| {
        cx.subscribe(&terminal, move |_, event: &TerminalEvent, _| {
            if *event == TerminalEvent::Resized {
                counted.set(counted.get() + 1);
            }
        })
    });
    settle(cx);
    assert_eq!(resized.get(), 0, "a frame that lays out the same size says nothing");
    cx.simulate_window_resize(window.handle, size(px(1400.0), px(700.0)));
    settle(cx);
    assert!(resized.get() >= 1, "the grid has another size and the terminal says so");

    // The window follows it when the bar it drew is not what it would draw now, and only then.
    let wakes = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = wakes.clone();
    cx.update(|cx| {
        let workspace = window.workspace.upgrade().expect("the window is open");
        cx.observe(&workspace, move |_, _| counted.set(counted.get() + 1)).detach();
    });
    let said = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, window, cx| ws.on_terminal_event(1, TerminalEvent::Resized, window, cx))
    };
    said(cx);
    assert_eq!(wakes.get(), 0, "the bar already says the size");
    drive(cx, &window, |ws, _, _| ws.forget_drawn_size());
    said(cx);
    assert_eq!(wakes.get(), 1, "the bar is behind: the window is drawn again");
    draw(cx, &window);
    let (shown, grid) = read(cx, &window, |ws| (ws.drawn_size(), ws.sessions[0].pane.grid.size()));
    let shell = read(cx, &window, |ws| ws.active_session().pane.program());
    assert_eq!(
        shown,
        Some(wormhole_core::view::layout::status_bar::size(&shell, grid.0, grid.1)),
        "and the bar has the terminal's own"
    );
}
