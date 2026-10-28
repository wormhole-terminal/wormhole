//! The picker in a real window (see `window::test_harness`: each test runs in a child process with scratch
//! data and `/bin/cat` for a shell, which echoes what is inserted, so the screen shows the path).

use std::path::Path;
use std::time::{Duration, Instant};

use gpui_kit::{Focusable as _, Modifiers, TestAppContext, VisualTestContext, point, px};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};

use super::FolderPicker;
use crate::app_state::OpenWindow;
use crate::settings::update_settings;
use crate::terminal_view::TerminalView;
use crate::window::test_harness::{draw, drive, isolate_in, press, read, read_cx, start_with, with_window};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn flag(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        update_settings(cx, |s| _ = s.feature_flags.insert(FeatureFlag::FolderPicker.key().into(), on))
    });
}

/// A folder tree to list, made inside the scratch data folder the harness gave this test (so nothing of the
/// person's is read): two folders, one with a space and one dot-folder, a folder with a folder in it, a file.
struct Tree(std::path::PathBuf);

impl Tree {
    fn path(&self) -> &Path {
        &self.0
    }
}

fn tree() -> Tree {
    let data = std::env::var_os("WORMHOLE_DATA_DIR").expect("the harness's scratch data folder");
    let root = std::path::PathBuf::from(data).join("picker-tree");
    for path in ["alpha", "beta/inner", "My Folder", ".hidden"] {
        std::fs::create_dir_all(root.join(path)).expect("a folder");
    }
    std::fs::write(root.join("file.txt"), "x").expect("a file");
    Tree(root)
}

/// A window whose terminal starts in `folder`, drawn, the flag on.
fn open_in(cx: &mut TestAppContext, folder: &Path) -> OpenWindow {
    let record = WindowRecord {
        id: 1,
        sessions: vec![SessionRecord::new(1, None, Some(folder.to_string_lossy().into_owned()))],
        ..Default::default()
    };
    let window = start_with(cx, record);
    flag(cx, true);
    draw(cx, &window);
    window
}

fn terminal(cx: &mut TestAppContext, window: &OpenWindow) -> gpui_kit::Entity<TerminalView> {
    read(cx, window, |ws| ws.active_terminal().clone())
}

fn terminal_focused(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    with_window(cx, window, |win, cx| {
        let workspace = window.workspace.upgrade().expect("the window is open");
        workspace.read(cx).active_terminal().focus_handle(cx).is_focused(win)
    })
}

/// What the open picker lists, as `devctl`'s `state` would.
fn rows(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    read_cx(cx, window, |ws, cx| {
        let picker = ws.overlay_of::<FolderPicker>().expect("the picker is open");
        picker.read(cx).state.rows().iter().map(|r| r.name.clone()).collect()
    })
}

fn typed(cx: &mut TestAppContext, window: &OpenWindow, text: &str) {
    let text = text.to_string();
    drive(cx, window, move |ws, window, cx| {
        let picker = ws.overlay_of::<FolderPicker>().expect("the picker is open");
        picker.update(cx, |picker, cx| {
            picker.field.update(cx, |field, cx| field.set_value(text.clone(), window, cx));
            picker.typed(&text, cx);
        });
    });
    draw(cx, window);
}

/// Wait for the screen of the terminal to show `what`: the shell is `cat`, which echoes after a moment.
fn screen_shows(cx: &mut TestAppContext, window: &OpenWindow, what: &str) {
    let terminal = terminal(cx, window);
    let limit = Instant::now() + Duration::from_secs(10);
    while !cx.update(|cx| terminal.read(cx).screen_text().contains(what)) {
        assert!(Instant::now() < limit, "the screen never showed {what:?}");
        std::thread::sleep(Duration::from_millis(10));
        cx.run_until_parked();
    }
}

fn open_picker(cx: &mut TestAppContext, window: &OpenWindow) {
    draw(cx, window);
    press(cx, window, "cmd-j");
    draw(cx, window);
}

#[gpui_kit::test]
fn cmd_j_opens_the_picker_over_the_terminals_folders_and_esc_gives_the_keyboard_back(
    cx: &mut TestAppContext,
) {
    if isolate("cmd_j_opens_the_picker_over_the_terminals_folders_and_esc_gives_the_keyboard_back") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    assert!(terminal_focused(cx, &window));
    open_picker(cx, &window);
    assert!(read(cx, &window, |ws| ws.folder_picker_open()), "⌘J opens it");
    assert!(!terminal_focused(cx, &window), "its field has the keyboard");
    assert_eq!(rows(cx, &window), ["alpha", "beta", "My Folder"], "ls order, no files, no dot-folders");
    press(cx, &window, "escape");
    assert!(!read(cx, &window, |ws| ws.folder_picker_open()));
    assert!(terminal_focused(cx, &window), "the keyboard is back in the terminal");

    // ⌘J again toggles it.
    open_picker(cx, &window);
    assert!(read(cx, &window, |ws| ws.folder_picker_open()));
    press(cx, &window, "cmd-j");
    assert!(!read(cx, &window, |ws| ws.folder_picker_open()), "⌘J closes it");
    assert!(terminal_focused(cx, &window));
}

#[gpui_kit::test]
fn with_the_flag_off_cmd_j_does_nothing_and_the_menu_has_no_item(cx: &mut TestAppContext) {
    if isolate("with_the_flag_off_cmd_j_does_nothing_and_the_menu_has_no_item") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    flag(cx, false);
    open_picker(cx, &window);
    assert!(!read(cx, &window, |ws| ws.folder_picker_open()));
    assert!(terminal_focused(cx, &window));
    let hidden = |cx: &mut TestAppContext, window: &OpenWindow| {
        read_cx(cx, window, |ws, cx| ws.menu_state(cx).hidden.contains(&ActionId::InsertFolderPath))
    };
    assert!(hidden(cx, &window), "no menu item, palette row or sheet line");
    flag(cx, true);
    assert!(!hidden(cx, &window));
}

#[gpui_kit::test]
fn typing_narrows_the_list_and_return_inserts_the_path_at_the_prompt(cx: &mut TestAppContext) {
    if isolate("typing_narrows_the_list_and_return_inserts_the_path_at_the_prompt") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    press(cx, &window, "b");
    assert_eq!(rows(cx, &window), ["beta"]);
    press(cx, &window, "enter");
    assert!(!read(cx, &window, |ws| ws.folder_picker_open()), "↩ closes it");
    assert!(terminal_focused(cx, &window));
    screen_shows(cx, &window, "beta/");
}

#[gpui_kit::test]
fn the_arrows_move_tab_descends_and_a_path_comes_out_as_it_was_typed(cx: &mut TestAppContext) {
    if isolate("the_arrows_move_tab_descends_and_a_path_comes_out_as_it_was_typed") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    press(cx, &window, "down");
    press(cx, &window, "down");
    press(cx, &window, "up");
    // Selected: beta. ⇥ writes it into the field and lists its folders.
    press(cx, &window, "tab");
    draw(cx, &window);
    assert_eq!(rows(cx, &window), ["inner"]);
    let field = read_cx(cx, &window, |ws, cx| {
        let picker = ws.overlay_of::<FolderPicker>().expect("open");
        picker.read(cx).field.read(cx).value().to_string()
    });
    assert_eq!(field, "beta/");
    press(cx, &window, "enter");
    screen_shows(cx, &window, "beta/inner/");
}

#[gpui_kit::test]
fn a_name_with_a_space_is_escaped_for_the_shell(cx: &mut TestAppContext) {
    if isolate("a_name_with_a_space_is_escaped_for_the_shell") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    typed(cx, &window, "My");
    assert_eq!(rows(cx, &window), ["My Folder"]);
    press(cx, &window, "enter");
    screen_shows(cx, &window, r"My\ Folder/");
}

#[gpui_kit::test]
fn a_path_walks_up_and_a_dot_shows_dot_folders_and_nothing_lists_as_no_folders(cx: &mut TestAppContext) {
    if isolate("a_path_walks_up_and_a_dot_shows_dot_folders_and_nothing_lists_as_no_folders") {
        return;
    }
    let dir = tree();
    let inner = dir.path().join("beta/inner");
    let window = open_in(cx, &inner);
    open_picker(cx, &window);
    assert!(rows(cx, &window).is_empty(), "a folder with no folders");
    assert!(read_cx(cx, &window, |ws, cx| ws
        .overlay_of::<FolderPicker>()
        .expect("open")
        .read(cx)
        .state
        .loaded()));
    typed(cx, &window, "../../");
    assert_eq!(rows(cx, &window), ["alpha", "beta", "My Folder"]);
    typed(cx, &window, "../../.");
    assert_eq!(rows(cx, &window), [".hidden"]);
    typed(cx, &window, "../../zzz");
    assert!(rows(cx, &window).is_empty());
    typed(cx, &window, "../../a");
    press(cx, &window, "enter");
    screen_shows(cx, &window, "../../alpha/");
}

#[gpui_kit::test]
fn a_click_on_a_row_inserts_it_and_a_click_outside_closes_it(cx: &mut TestAppContext) {
    if isolate("a_click_on_a_row_inserts_it_and_a_click_outside_closes_it") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    let card = |cx: &mut TestAppContext| {
        VisualTestContext::from_window(window.handle, cx)
            .debug_bounds("folder-picker")
            .expect("the card is drawn")
    };
    // The first row: past the card's padding, at its middle.
    let first = card(cx).origin + point(px(60.0), px(4.0 + 14.0));
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    pointer.simulate_click(first, Modifiers::none());
    assert!(!read(cx, &window, |ws| ws.folder_picker_open()), "a click on a row picks it");
    screen_shows(cx, &window, "alpha/");

    // Open again; a click in the terminal, away from the card, closes it and the terminal gets the keyboard.
    open_picker(cx, &window);
    assert!(read(cx, &window, |ws| ws.folder_picker_open()));
    let size = with_window(cx, &window, |win, _| win.viewport_size());
    let away = point(size.width - px(60.0), size.height - px(60.0));
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    pointer.simulate_click(away, Modifiers::none());
    assert!(!read(cx, &window, |ws| ws.folder_picker_open()), "a click outside closes it");
    assert!(terminal_focused(cx, &window));
}
