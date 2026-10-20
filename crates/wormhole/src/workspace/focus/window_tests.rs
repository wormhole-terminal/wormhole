//! The keys in a real window (GPUI's test platform; see `view_tests` for how each test runs in
//! a child process with scratch data): ⇥ between stops, the arrows, ↩ and Space inside the
//! sidebar and the panel, Esc back to the terminal.
use gpui_kit::TestAppContext;
use wormhole_core::focus::Area;

use super::super::RightTab;
use super::super::app::OpenWindow;
use super::super::view_tests::{draw, drive, isolate_in, press, read, start};
use super::SidebarStop;

/// Press a key as the keyboard would, after a frame (the keys go where the last frame's focus is).
fn key(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
    draw(cx, window);
    press(cx, window, keys);
}

fn area(cx: &mut TestAppContext, window: &OpenWindow) -> Option<Area> {
    draw(cx, window);
    cx.update(|cx| {
        window.handle.update(cx, |_, win, cx| {
            let workspace = window.workspace.upgrade()?;
            workspace.read(cx).focus_area(win, cx)
        })
    })
    .ok()
    .flatten()
}

/// F6 until `to` has the keyboard (at most once round the window).
fn f6_to(cx: &mut TestAppContext, window: &OpenWindow, to: Area) {
    for _ in 0..Area::ALL.len() {
        if area(cx, window) == Some(to) {
            return;
        }
        key(cx, window, "f6");
    }
    assert_eq!(area(cx, window), Some(to), "F6 never reached {to:?}");
}

#[gpui_kit::test]
fn the_sidebar_is_walked_with_the_arrows_and_opened_with_return_or_space(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_sidebar_is_walked_with_the_arrows_and_opened_with_return_or_space") {
        return;
    }
    let window = start(cx, 3);
    let ids: Vec<i64> = read(cx, &window, |ws| ws.sessions.iter().map(|s| s.id).collect());
    let at = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.areas.sidebar_at.clone());
    let active = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.sessions[ws.active].id);

    f6_to(cx, &window, Area::Sidebar);
    assert_eq!(at(cx), Some(SidebarStop::Row(active(cx))), "the keys start on the selected terminal");
    key(cx, &window, "down");
    assert_eq!(at(cx), Some(SidebarStop::Row(ids[1])));
    key(cx, &window, "down");
    key(cx, &window, "down");
    assert_eq!(at(cx), Some(SidebarStop::Row(ids[2])), "the list doesn't wrap");
    key(cx, &window, "alt-up");
    assert_eq!(at(cx), Some(SidebarStop::Row(ids[0])), "⌥↑: the first row");

    // Space shows a terminal and keeps the keys in the list (browsing).
    key(cx, &window, "down");
    key(cx, &window, "space");
    assert_eq!(active(cx), ids[1]);
    assert_eq!(area(cx, &window), Some(Area::Sidebar));
    // Return opens it and gives it the keyboard.
    key(cx, &window, "down");
    key(cx, &window, "enter");
    assert_eq!(active(cx), ids[2]);
    assert_eq!(area(cx, &window), Some(Area::Terminal));

    // Back in the list the keys are where they were; Esc goes to the terminal.
    f6_to(cx, &window, Area::Sidebar);
    assert_eq!(at(cx), Some(SidebarStop::Row(ids[2])));
    key(cx, &window, "escape");
    assert_eq!(area(cx, &window), Some(Area::Terminal));
}

#[gpui_kit::test]
fn tab_steps_from_stop_to_stop_into_the_terminal(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "tab_steps_from_stop_to_stop_into_the_terminal") {
        return;
    }
    let window = start(cx, 1);
    f6_to(cx, &window, Area::Sidebar);
    // 2 sidebar → 3 tabs → 4 terminal.
    key(cx, &window, "tab");
    assert_eq!(area(cx, &window), Some(Area::Tabs));
    key(cx, &window, "shift-tab");
    assert_eq!(area(cx, &window), Some(Area::Sidebar), "⇧⇥ goes back");
    key(cx, &window, "tab");
    key(cx, &window, "tab");
    // There ⇥ is the program's: the terminal takes its keys before any binding (not pressed here,
    // where its echo would come from the PTY's own thread).
    assert_eq!(area(cx, &window), Some(Area::Terminal));
}

#[gpui_kit::test]
fn the_next_steps_are_walked_and_ticked_from_the_keyboard(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_next_steps_are_walked_and_ticked_from_the_keyboard") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, _, cx| {
        ws.add_task("Fix the flaky login test".into(), cx);
        ws.add_task("Update README".into(), cx);
        ws.show_tab(RightTab::Notes, cx);
    });
    assert!(read(cx, &window, |ws| ws.notes_visible()), "the panel is on screen");
    let done = |cx: &mut TestAppContext| {
        read(cx, &window, |ws| ws.active_session().tasks.iter().map(|t| t.done).collect::<Vec<_>>())
    };

    f6_to(cx, &window, Area::PanelContent);
    // The keys start on the Next steps band: ↓ to the first step, Space ticks it.
    key(cx, &window, "down");
    key(cx, &window, "space");
    assert_eq!(done(cx), [true, false]);
    assert_eq!(area(cx, &window), Some(Area::PanelContent), "the list keeps the keyboard");
    // ← on a step goes to its band; ← again folds the section.
    key(cx, &window, "left");
    key(cx, &window, "left");
    assert!(read(cx, &window, |ws| ws.active_session().tasks_collapsed), "← folded Next steps");
    key(cx, &window, "right");
    assert!(read(cx, &window, |ws| !ws.active_session().tasks_collapsed), "→ opened it");
    key(cx, &window, "escape");
    assert_eq!(area(cx, &window), Some(Area::Terminal));
}

/// On a section's band ↩ does nothing and Space folds or opens it.
#[gpui_kit::test]
fn on_a_section_band_only_space_folds_it(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "on_a_section_band_only_space_folds_it") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, _, cx| {
        ws.add_task("Fix the flaky login test".into(), cx);
        ws.show_tab(RightTab::Notes, cx);
    });
    let folded = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.active_session().tasks_collapsed);

    // The keys start on the Next steps band.
    f6_to(cx, &window, Area::PanelContent);
    key(cx, &window, "enter");
    assert!(!folded(cx), "↩ on a band does nothing");
    assert_eq!(area(cx, &window), Some(Area::PanelContent));
    key(cx, &window, "space");
    assert!(folded(cx), "Space folded Next steps");
    key(cx, &window, "space");
    assert!(!folded(cx), "and opened it again");
    assert_eq!(area(cx, &window), Some(Area::PanelContent), "the list keeps the keyboard");
}

/// On a group's head in the sidebar too, ↩ does nothing and Space folds or opens it;
/// the list keeps the keyboard either way.
#[gpui_kit::test]
fn on_a_sidebar_group_only_space_folds_it(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "on_a_sidebar_group_only_space_folds_it") {
        return;
    }
    let window = start(cx, 2);
    drive(cx, &window, |ws, _, cx| {
        ws.group_terminals(&[1, 2], cx);
    });
    let folded = |cx: &mut TestAppContext| {
        read(cx, &window, |ws| match &ws.areas.sidebar_at {
            Some(SidebarStop::Group(group)) => ws.sidebar.view.is_group_folded(*group),
            at => panic!("the keys are on {at:?}, not a group"),
        })
    };

    f6_to(cx, &window, Area::Sidebar);
    // ⌥↑: the first line, the head both terminals are under.
    key(cx, &window, "alt-up");
    assert!(!folded(cx), "the group starts open");
    key(cx, &window, "enter");
    assert!(!folded(cx), "↩ on a head does nothing");
    assert_eq!(area(cx, &window), Some(Area::Sidebar));
    key(cx, &window, "space");
    assert!(folded(cx), "Space folded the group");
    key(cx, &window, "space");
    assert!(!folded(cx), "and opened it again");
    assert_eq!(area(cx, &window), Some(Area::Sidebar), "the list keeps the keyboard");
}

/// ⌃↩ in the sidebar opens the row's menu, the one a right-click shows. The menu is a native one
/// that holds the keys itself while it is open (a test window records it instead), so the list keeps
/// them here. In a list without row menus the key is taken all the same: it never reaches the
/// terminal while a list has the keyboard.
#[gpui_kit::test]
fn ctrl_return_opens_the_rows_menu_from_the_keyboard(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "ctrl_return_opens_the_rows_menu_from_the_keyboard") {
        return;
    }
    let window = start(cx, 2);
    crate::native::record::close();

    f6_to(cx, &window, Area::Sidebar);
    key(cx, &window, "ctrl-enter");
    draw(cx, &window);
    let menu = crate::native::record::open().expect("⌃↩ opened the row's menu");
    assert_eq!(menu.outline().first().map(String::as_str), Some("- Rename…"), "the row's own menu");
    assert_eq!(area(cx, &window), Some(Area::Sidebar), "the list keeps the keys; the menu is the system's");
    assert!(crate::native::record::close().is_some(), "and it can be dismissed");

    drive(cx, &window, |ws, _, cx| ws.show_tab(RightTab::Notes, cx));
    f6_to(cx, &window, Area::PanelContent);
    key(cx, &window, "ctrl-enter");
    assert!(crate::native::record::open().is_none(), "the panel's rows have no menu yet");
    assert_eq!(area(cx, &window), Some(Area::PanelContent), "the list kept the key");
}

/// A diff covers the terminal: Esc closes it first ("close the diff first"), as its header
/// says, though the terminal under it still has the keys; the shell never sees that Esc.
#[gpui_kit::test]
fn esc_closes_a_diff_over_the_terminal_first(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "esc_closes_a_diff_over_the_terminal_first") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| {
        ws.focus_terminal(window, cx);
        let change = wormhole_core::git::FileChange {
            path: "src/lib.rs".into(),
            original_path: None,
            kind: wormhole_core::git::ChangeKind::Modified,
            added: Some(1),
            removed: Some(0),
            lfs: false,
        };
        let spec = wormhole_core::git::DiffSpec { old: "HEAD".into(), new: None };
        ws.open_diff("/nonexistent".into(), change, spec, std::rc::Rc::default(), None, cx);
    });
    assert_eq!(area(cx, &window), Some(Area::Terminal), "the click left the keys in the terminal");
    key(cx, &window, "escape");
    assert!(read(cx, &window, |ws| ws.diff_view.is_none()), "Esc closed the diff");
    assert_eq!(area(cx, &window), Some(Area::Terminal));
}
