//! The terminal list in a real window: its groups, pins and folds, the ⌘ digits and stepping, and a hidden
//! sidebar that does not redraw for its times. The harness is `test_harness`.

use std::rc::Rc;

use gpui_kit::TestAppContext;
use wormhole_core::view::sidebar::{Entry, WindowSidebar};

use crate::app_state::OpenWindow;
use crate::window::test_harness::{draw, drive, hook, isolate_in, press, read, sidebar_of, start};

/// The sidebar's list as text: bands as `[band]`, rows as their session id and ⌘ number.
fn listed(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    cx.update(|cx| {
        window.workspace.read_with(cx, |ws, cx| {
            let listing = ws.session_listing(cx);
            listing
                .entries
                .iter()
                .map(|entry| match *entry {
                    Entry::Band { .. } => "[band]".to_string(),
                    Entry::Row { session, number, .. } => {
                        let id = ws.sessions[session].id;
                        number.map_or_else(|| id.to_string(), |n| format!("{id}:{n}"))
                    }
                })
                .collect()
        })
    })
    .expect("the window is open")
}

#[gpui_kit::test]
fn the_sidebar_lists_groups_pins_and_folds(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_sidebar_lists_groups_pins_and_folds") {
        return;
    }
    let window = start(cx, 3);
    draw(cx, &window);
    // A window that was never saved has no groups: the terminals are a card each, numbered top to
    // bottom.
    assert_eq!(listed(cx, &window), ["1:1", "2:2", "3:3"]);

    // A pinned terminal goes first, and ⌘1 follows it.
    drive(cx, &window, |ws, _, cx| ws.toggle_pin(2, cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["3:1", "1:2", "2:3"]);
    drive(cx, &window, |ws, _, cx| ws.toggle_pin(2, cx));

    // Two terminals grouped stand where the first stood, under a head of their own.
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2, 3], cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["1:1", "[band]", "2:2", "3:3"]);

    // A folded group keeps its terminals out of the list and out of the numbering.
    drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["1:1", "[band]"]);
    drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["1:1", "[band]", "2:2", "3:3"]);

    let saved = sidebar_of(cx, &window, |sidebar| sidebar.view().clone());
    // What is saved with the window brings all of it back.
    let record = cx
        .update(|cx| window.workspace.read_with(cx, |ws, cx| ws.window_record(cx)))
        .expect("the window is open");
    let layout: serde_json::Value =
        serde_json::from_str(record.layout.as_deref().expect("a layout")).expect("layout JSON");
    let restored: WindowSidebar =
        serde_json::from_value(layout["sidebar"].clone()).expect("the sidebar's state");
    assert_eq!(restored, saved);
    assert_eq!(record.sessions.iter().map(|s| s.pinned).collect::<Vec<_>>(), [false, false, false]);
}

#[gpui_kit::test]
fn command_digits_and_stepping_follow_what_the_sidebar_shows(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "command_digits_and_stepping_follow_what_the_sidebar_shows") {
        return;
    }
    let window = start(cx, 4);
    let active = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.sessions[ws.active].id);
    // The first two are a group, folded: its terminals are skipped, so ⌘1 is the third.
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1, 2], cx));
    drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["[band]", "3:1", "4:2"]);
    press(cx, &window, "cmd-1");
    assert_eq!(active(cx), 3, "⌘1 is the terminal its keycap is on");

    // Stepping goes between what is shown, past the folded ones, and wraps.
    press(cx, &window, "ctrl-cmd-down");
    assert_eq!(active(cx), 4);
    draw(cx, &window);
    press(cx, &window, "ctrl-cmd-down");
    assert_eq!(active(cx), 3, "the folded terminals are skipped, and it wraps");
}

#[gpui_kit::test]
fn a_hidden_sidebar_never_redraws_the_window_for_its_times(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_hidden_sidebar_never_redraws_the_window_for_its_times") {
        return;
    }
    let window = start(cx, 2);
    drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
    draw(cx, &window);
    // A finished agent's row would read "Finished · just now", then "· 1m ago", …
    hook(cx, &window, 2, "Stop", None);
    let workspace = window.workspace.upgrade().expect("the window is open");
    let redraws = Rc::new(std::cell::Cell::new(0));
    let _watch = cx.update(|cx| {
        let redraws = redraws.clone();
        cx.observe(&workspace, move |_, _| redraws.set(redraws.get() + 1))
    });
    for _ in 0..3 {
        drive(cx, &window, |ws, _, cx| ws.sidebar_tick(true, cx));
    }
    assert_eq!(redraws.get(), 0, "nothing on screen shows those times");
}

/// There are no tabs: each terminal is one row of the sidebar and the title row has its breadcrumb
/// and the split buttons, nothing else. Closing a terminal takes its row away.
#[gpui_kit::test]
fn each_terminal_gets_a_sidebar_row_and_no_tab_strip(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "each_terminal_gets_a_sidebar_row_and_no_tab_strip") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let rows = |cx: &mut TestAppContext| {
        cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.session_listing(cx)))
            .expect("open")
            .entries
            .iter()
            .filter(|entry| matches!(entry, Entry::Row { .. }))
            .count()
    };
    assert_eq!(rows(cx), 1);
    drive(cx, &window, |ws, window, cx| {
        ws.new_session(window, cx);
        ws.new_session(window, cx);
    });
    draw(cx, &window);
    assert_eq!(rows(cx), 3, "⌘T twice: three rows");
    let mut drawn = gpui_kit::VisualTestContext::from_window(window.handle, cx);
    for selector in ["tab-label-0", "sidebar-tab-0-0"] {
        assert!(drawn.debug_bounds(selector).is_none(), "{selector}: tabs are gone");
    }
    drive(cx, &window, |ws, window, cx| ws.request_close_session(1, window, cx));
    assert_eq!(rows(cx), 2);
}
