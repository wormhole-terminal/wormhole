//! The keys, F6 and the menus in a real window (GPUI's test platform; see `view_tests` for
//! how each test runs in a child process with scratch data).
use gpui_kit::{OwnedMenuItem, TestAppContext};
use wormhole_core::view::focus::Area;

use super::RightTab;
use super::app::{OpenWindow, app};
use super::view_tests::{draw, drive, hook, isolate_in, press, read, start};

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

#[gpui_kit::test]
fn the_shortcut_keys_reach_their_actions(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_shortcut_keys_reach_their_actions") {
        return;
    }
    let window = start(cx, 1);
    let terminals = |cx: &mut TestAppContext| read(cx, &window, |ws| (ws.sessions.len(), ws.active));

    key(cx, &window, "cmd-t");
    assert_eq!(terminals(cx), (2, 1), "⌘T: a new terminal");
    key(cx, &window, "cmd-1");
    assert_eq!(terminals(cx), (2, 0), "⌘1: the sidebar's first terminal");
    // A US keyboard reports ⇧⌘] as ⌘}.
    key(cx, &window, "cmd-}");
    assert_eq!(terminals(cx), (2, 1), "⇧⌘]: the next terminal");
    key(cx, &window, "shift-cmd-[");
    assert_eq!(terminals(cx), (2, 0), "⇧⌘[: the previous one");
    key(cx, &window, "ctrl-cmd-down");
    assert_eq!(terminals(cx), (2, 1), "⌃⌘↓: the next terminal too");

    key(cx, &window, "shift-cmd-n");
    assert_eq!(terminals(cx), (3, 2), "⇧⌘N: a new terminal, as before ⌘T was one");
    key(cx, &window, "cmd-2");
    assert_eq!(terminals(cx), (3, 1), "⌘2: the second terminal");

    key(cx, &window, "ctrl-cmd-2");
    read(cx, &window, |ws| assert!(ws.right_tab == RightTab::History, "⌃⌘2: History"));
    key(cx, &window, "ctrl-cmd-3");
    read(cx, &window, |ws| assert!(ws.right_tab == RightTab::Notes, "⌃⌘3: Notes"));
    // Usage is there while an agent is.
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[ws.active]
            .terminal()
            .update(cx, |t, _| t.agent = Some(wormhole_core::agents::AgentId::Claude));
    });
    // Usage is there while an agent is.
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[ws.active]
            .terminal()
            .update(cx, |t, _| t.agent = Some(wormhole_core::agents::AgentId::Claude));
    });
    key(cx, &window, "ctrl-cmd-4");
    read(cx, &window, |ws| assert!(ws.right_tab == RightTab::Usage, "⌃⌘4: Usage"));
    key(cx, &window, "ctrl-cmd-1");
    read(cx, &window, |ws| assert!(ws.right_tab == RightTab::Changes, "⌃⌘1: Changes"));

    let sidebar = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.panes.sidebar_open);
    let before = sidebar(cx);
    key(cx, &window, "ctrl-cmd-s");
    assert_eq!(sidebar(cx), !before, "⌃⌘S: the sidebar");
    key(cx, &window, "cmd-b");
    assert_eq!(sidebar(cx), !before, "⌘B is free now");
    let panel = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.panes.panel_wanted);
    let before = panel(cx);
    key(cx, &window, "alt-cmd-i");
    assert_eq!(panel(cx), !before, "⌥⌘I: the right panel, not GPUI Kit's inspector");

    key(cx, &window, "cmd-+");
    key(cx, &window, "cmd-=");
    read(cx, &window, |ws| assert_eq!(ws.font_zoom, 2.0, "⌘+ (and ⌘=): bigger"));
    key(cx, &window, "cmd--");
    read(cx, &window, |ws| assert_eq!(ws.font_zoom, 1.0, "⌘-: smaller"));
    key(cx, &window, "cmd-0");
    read(cx, &window, |ws| assert_eq!(ws.font_zoom, 0.0, "⌘0: the size in Settings"));

    key(cx, &window, "cmd-n");
    assert_eq!(cx.update(|cx| app(cx).windows.len()), 2, "⌘N: a new window");
}

#[gpui_kit::test]
fn f6_moves_the_keyboard_between_areas_even_from_the_terminal(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "f6_moves_the_keyboard_between_areas_even_from_the_terminal") {
        return;
    }
    let window = start(cx, 1);
    assert_eq!(area(cx, &window), Some(Area::Terminal));
    key(cx, &window, "f6");
    let next = area(cx, &window);
    assert!(next.is_some_and(|a| a != Area::Terminal), "F6 left the terminal: {next:?}");
    key(cx, &window, "shift-f6");
    assert_eq!(area(cx, &window), Some(Area::Terminal), "⇧F6 came back");
    key(cx, &window, "shift-f6");
    assert!(area(cx, &window).is_some_and(|a| a != Area::Terminal), "⇧F6 goes backwards too");
    key(cx, &window, "escape");
    assert_eq!(area(cx, &window), Some(Area::Terminal), "Esc goes back to the terminal");
}

#[gpui_kit::test]
fn the_menus_follow_the_window_in_front(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_menus_follow_the_window_in_front") {
        return;
    }
    let window = start(cx, 2);
    // What the menus say now, without asking them to catch up first.
    let items = |cx: &mut TestAppContext, menu: &str| {
        let menus = cx.update(|cx| cx.get_menus()).unwrap_or_default();
        let menu = menus.into_iter().find(|m| m.name.as_ref() == menu).unwrap_or_else(|| panic!("{menu}"));
        menu.items
            .iter()
            .filter_map(|item| match item {
                OwnedMenuItem::Action { name, checked, .. } => {
                    Some(format!("{}{name}", if *checked { "✓ " } else { "" }))
                }
                OwnedMenuItem::Submenu(menu) => Some(format!("{} ▸", menu.name)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    draw(cx, &window);
    cx.update(super::app_menu::sync);
    let titles: Vec<String> =
        cx.update(|cx| cx.get_menus()).unwrap_or_default().iter().map(|m| m.name.to_string()).collect();
    assert_eq!(&titles[..7], ["Wormhole", "Shell", "Edit", "View", "Terminal", "Window", "Help"]);

    let sidebar = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.panes.sidebar_open);
    let label = |open: bool| if open { "Hide Sidebar" } else { "Show Sidebar" };
    let open = sidebar(cx);
    assert!(items(cx, "View").contains(&label(open).to_string()));
    key(cx, &window, "ctrl-cmd-s");
    assert!(items(cx, "View").contains(&label(!open).to_string()), "the label flips at once, not on a tick");
    assert!(items(cx, "View").contains(&"✓ Changes".to_string()), "the panel's tab is checked");

    // The sidebar's numbered terminals by name, the current one checked.
    let names = cx
        .update(|cx| {
            window.workspace.read_with(cx, |ws, cx| {
                let numbered = ws.session_listing(cx).numbered();
                let current = numbered.iter().position(|&ix| ix == ws.active);
                let names: Vec<String> =
                    numbered.iter().map(|&ix| ws.sessions[ix].name(cx).to_string()).collect();
                names
                    .into_iter()
                    .enumerate()
                    .map(|(ix, name)| if Some(ix) == current { format!("✓ {name}") } else { name })
                    .collect::<Vec<_>>()
            })
        })
        .expect("the window is open");
    assert_eq!(names.len(), 2);
    let window_menu = items(cx, "Window");
    let first = window_menu.iter().position(|item| *item == names[0]).expect("the terminals are listed");
    assert_eq!(window_menu[first..first + 2], names[..], "in the sidebar's order: {window_menu:?}");
}

#[gpui_kit::test]
fn a_menu_item_that_cannot_act_is_not_available(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_menu_item_that_cannot_act_is_not_available") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let available = |cx: &mut TestAppContext, action: &dyn gpui_kit::Action| {
        cx.update(|cx| window.handle.update(cx, |_, win, cx| win.is_action_available(action, cx)))
            .expect("the window is open")
    };
    assert!(!available(cx, &super::actions::ShowProject), "the shell hasn't said where it is yet");
    assert!(available(cx, &super::actions::ShowChanges));
    assert!(!available(cx, &super::actions::RevealInFinder), "the shell hasn't said where it is yet");
    drive(cx, &window, |ws, _, cx| {
        ws.active_terminal().update(cx, |t, _| t.cwd = Some(std::env::temp_dir()));
    });
    draw(cx, &window);
    assert!(available(cx, &super::actions::RevealInFinder), "now it has a folder");
    assert!(available(cx, &super::actions::CopyPath));
    assert!(available(cx, &super::actions::ShowProject), "the Project tab needs a folder, not a project");
}

#[gpui_kit::test]
fn open_folder_starts_a_terminal_in_the_picked_folder(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "open_folder_starts_a_terminal_in_the_picked_folder") {
        return;
    }
    let window = start(cx, 1);
    let folder = std::env::temp_dir().join(format!("wormhole-open-folder-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("creates the folder");
    key(cx, &window, "cmd-o");
    assert!(cx.did_prompt_for_paths(), "⌘O asks for a folder");
    let picked = folder.clone();
    cx.simulate_path_prompt_response(move |options| {
        assert!(options.directories && !options.files);
        Some(vec![picked])
    });
    cx.run_until_parked();
    let cwd = cx.update(|cx| {
        window
            .workspace
            .read_with(cx, |ws, cx| (ws.sessions.len(), ws.active_terminal().read(cx).cwd.clone()))
    });
    assert_eq!(cwd.ok(), Some((2, Some(folder.clone()))), "a new terminal there, selected");
    std::fs::remove_dir_all(&folder).ok();
}

#[gpui_kit::test]
fn the_jump_key_is_the_one_in_the_settings(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_jump_key_is_the_one_in_the_settings") {
        return;
    }
    let window = start(cx, 2);
    hook(cx, &window, 2, "Notification", Some("Allow Bash?"));
    let active = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.sessions[ws.active].id);
    assert_eq!(active(cx), 1);
    key(cx, &window, "alt-cmd-j");
    assert_eq!(active(cx), 2, "⌥⌘J: to the terminal that waits");

    // Another key recorded in Settings takes over, and ⌥⌘J lets go.
    cx.update(|cx| crate::settings::SettingsStore::update(cx, |s| s.jump_waiting_key = "ctrl-cmd-j".into()));
    cx.run_until_parked();
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    key(cx, &window, "alt-cmd-j");
    assert_eq!(active(cx), 1, "⌥⌘J is free again");
    key(cx, &window, "ctrl-cmd-j");
    assert_eq!(active(cx), 2, "the recorded key jumps");
}
