use super::menus::{Command, Item, MenuState, dump, menus};
use super::sheet::{Keys, columns, filter, sections, sections_without};
use super::{
    ActionId, KeyUse, Scope, Stage, action_for, entries, entry, glyphs, list_action, terminal_action,
};
use super::{studio_action, terminal_yields, uses, with_key};
use crate::keys::keyspec::KeySpec;

fn spec(glyphs: &str) -> KeySpec {
    KeySpec::from_glyphs(glyphs).unwrap_or_else(|| panic!("{glyphs} parses"))
}

fn keys_of(id: ActionId) -> Vec<String> {
    entry(id).unwrap_or_else(|| panic!("{id:?} has a row")).keys.iter().map(KeySpec::glyphs).collect()
}

/// A row of the shortcut map: its label, its actions, their keys before and after the remap.
type ShortcutRow = (&'static str, Vec<ActionId>, Vec<&'static str>, Vec<&'static str>);

/// The shortcut map, row by row: the actions, their keys before the remap ("old", today's code and
/// the menu proposal) and after it ("new"). Two more rows are below.
#[test]
fn the_table_is_the_shortcut_map() {
    use ActionId::*;
    let terminals: Vec<_> = (1..=9).map(Terminal).collect();
    let terminal_keys: Vec<String> = (1..=9).map(|n| format!("⌘{n}")).collect();
    #[rustfmt::skip]
    let rows: Vec<ShortcutRow> = vec![
        // Windows & terminals (the tabs are gone: a terminal is a row of the sidebar)
        ("New Window", vec![NewWindow], vec!["⇧⌘N"], vec!["⌘N"]),
        ("New Terminal", vec![NewTerminal], vec!["⌘T"], vec!["⌘T"]),
        ("Open Folder…", vec![OpenFolder], vec![], vec!["⌘O"]),
        ("Close Terminal", vec![CloseTerminal], vec!["⌘W"], vec!["⌘W"]),
        ("Close Window", vec![CloseWindow], vec!["⇧⌘W"], vec!["⇧⌘W"]),
        // The keys the tabs had work here too, as second keys.
        ("Next / previous terminal", vec![NextTerminal, PreviousTerminal], vec![], vec!["⌃⌘↓", "⌃⌘↑"]),
        ("Go to waiting terminal", vec![GoToWaiting], vec!["⌥⌘J"], vec!["⌥⌘J"]),
        ("Split right · down · zoom pane", vec![SplitRight, SplitDown, ZoomPane], vec!["⌘D", "⇧⌘D", "⇧⌘↩"], vec!["⌘D", "⇧⌘D", "⇧⌘↩"]),
        ("Minimize · Hide · Hide Others · Quit", vec![Minimize, Hide, HideOthers, Quit],
            vec!["⌘M", "⌘H", "⌥⌘H", "⌘Q"], vec!["⌘M", "⌘H", "⌥⌘H", "⌘Q"]),
        // View & panels
        ("Show / hide sidebar", vec![ToggleSidebar], vec!["⌘B"], vec!["⌃⌘S"]),
        ("Show / hide right panel", vec![ToggleRightPanel], vec!["⌘J"], vec!["⌥⌘I"]),
        ("Changes · History · Notes · Usage · Project", vec![ShowChanges, ShowHistory, ShowNotes, ShowUsage, ShowProject],
            vec!["⇧⌘G", "⇧⌘H"], vec!["⌃⌘1", "⌃⌘2", "⌃⌘3", "⌃⌘4", "⌃⌘5"]),
        ("Focus terminal ↔ panel", vec![FocusTerminalOrPanel], vec!["⌘E"], vec!["⇧⌘E"]),
        ("Move focus to next / previous area", vec![FocusNextArea, FocusPreviousArea], vec![], vec!["F6", "⇧F6"]),
        ("Bigger · smaller · actual size", vec![Bigger, Smaller, ActualSize], vec![], vec!["⌘+", "⌘-", "⌘0"]),
        ("Full screen", vec![FullScreen], vec!["⌃⌘F"], vec!["⌃⌘F"]),
        ("Command palette", vec![CommandPalette], vec!["⌘P"], vec!["⇧⌘P"]),
        ("Wormhole Help", vec![Help], vec![], vec!["⌘?"]),
        // Edit & find (find, prompt marks and splits are in the backlog: see below)
        ("Undo · Redo · Cut · Copy · Paste · Select All", vec![Undo, Redo, Cut, Copy, Paste, SelectAll],
            vec!["⌘Z", "⇧⌘Z", "⌘X", "⌘C", "⌘V", "⌘A"], vec!["⌘Z", "⇧⌘Z", "⌘X", "⌘C", "⌘V", "⌘A"]),
        ("Clear buffer", vec![ClearBuffer], vec!["⌘K"], vec!["⌘K"]),
        ("Find · find next · find previous · use selection", vec![Find, FindNext, FindPrevious, UseSelectionForFind],
            vec!["⇧↩"], vec!["⌘F", "⌘G", "⇧⌘G", "⌘E"]),
        ("Previous / next prompt", vec![PreviousPrompt, NextPrompt], vec![], vec!["⌘↑", "⌘↓"]),
        ("Line start · end · delete line", vec![LineStart, LineEnd, DeleteLine],
            vec!["⌘←", "⌘→", "⌘⌫"], vec!["⌘←", "⌘→", "⌘⌫"]),
        // Notes
        ("New note", vec![NewNote], vec!["⌘N"], vec!["⌥⌘N"]),
        ("Stash typed prompt", vec![StashPrompt], vec!["⇧⌘S"], vec!["⌥⌘S"]),
        ("Move a note or step", vec![MoveItemUp, MoveItemDown], vec!["⌥⌘↑", "⌥⌘↓"], vec!["⌥⌘↑", "⌥⌘↓"]),
        // App
        ("Settings", vec![Settings], vec!["⌘,"], vec!["⌘,"]),
        ("Theme Studio", vec![ThemeStudio], vec!["⌥⌘T"], vec!["⌥⌘,"]),
        ("Inspector (debug builds)", vec![Inspector], vec!["⌥⌘I"], vec!["⇧⌥⌘I"]),
    ];
    for (label, actions, old, new) in rows {
        let now: Vec<String> = actions.iter().flat_map(|id| keys_of(*id).into_iter().take(1)).collect();
        let new: Vec<String> = new.iter().map(|g| spec(g).glyphs()).collect();
        assert_eq!(now, new, "{label}: the keys it should have");
        // A key that moved away is no longer this row's.
        for old in old.iter().filter(|o| !new.contains(&spec(o).glyphs())) {
            let holder = action_for(&spec(old), Scope::App).map(|e| e.id);
            assert!(holder.is_none_or(|id| !actions.contains(&id)), "{label}: {old} still runs it");
        }
    }
    // The numbered rows: ⌘1–9 are the sidebar's terminals, one number per row.
    assert_eq!(terminals.iter().map(|id| glyphs(*id)).collect::<Vec<_>>(), terminal_keys);
    // Where the old keys went.
    for (old, now) in [
        ("⌘T", NewTerminal),
        ("⇧⌘N", NewTerminal),
        ("⌘N", NewWindow),
        ("⌥⌘I", ToggleRightPanel),
        ("⇧⌘]", NextTerminal),
        ("⇧⌘[", PreviousTerminal),
    ] {
        assert_eq!(action_for(&spec(old), Scope::App).map(|e| e.id), Some(now), "{old}");
    }
    assert_eq!(entry(Inspector).map(|e| e.debug_only), Some(true));
    assert_eq!(entry(CommandPalette).map(|e| e.stage), Some(Stage::Now), "the palette is live");
    assert_eq!(entry(Help).map(|e| e.stage), Some(Stage::System), "the system's Help search");
}

/// ⌘/ shows or hides the optional status bar,
/// and the shortcut sheet moves from ⌘/ to ⌥⌘/.
#[test]
fn the_status_bar_has_command_slash_and_the_sheet_option_command_slash() {
    assert_eq!(keys_of(ActionId::ToggleStatusBar), ["⌘/"]);
    assert_eq!(keys_of(ActionId::KeyboardShortcuts), ["⌥⌘/"]);
    assert_eq!(entry(ActionId::KeyboardShortcuts).map(|e| e.stage), Some(Stage::Now));
}

#[test]
fn backlog_and_freed_keys_are_not_taken() {
    // The prompt marks are the command blocks', and the splits came out of the backlog
    // with iTerm2's keys; the freed keys stay free.
    for (key, id) in [
        ("⌘↑", ActionId::PreviousPrompt),
        ("⌘↓", ActionId::NextPrompt),
        ("⌘D", ActionId::SplitRight),
        ("⇧⌘D", ActionId::SplitDown),
        ("⇧⌘↩", ActionId::ZoomPane),
    ] {
        assert!(matches!(uses(&spec(key)), Some(KeyUse::Action(e, _)) if e.id == id), "{key}");
    }
    // ⌥⌘arrows move between panes from a terminal and move an item in a list: scopes apart.
    assert_eq!(terminal_action(&spec("⌥⌘↑")), Some(ActionId::FocusPaneUp));
    assert_eq!(terminal_action(&spec("⌥⌘←")), Some(ActionId::FocusPaneLeft));
    assert_eq!(list_action(&spec("⌥⌘↑")), Some(ActionId::MoveItemUp));
    assert!(!terminal_yields(&spec("⌥⌘↑")), "a terminal-scope key: the terminal's interceptor sees it first");
    for key in ["⌘B", "⌘J", "⌥⌘T", "⌥⌘W", "⌥⌘]", "⌥⌘[", "⌥⌘1", "⌥⌘9"] {
        assert_eq!(uses(&spec(key)), None, "{key} was freed by the remap");
    }
    // The global hotkey's example and the project actions' examples are free.
    for key in ["⇧⌘Space", "⌃⌘T", "⌃⌘L", "⌃⌘O"] {
        assert_eq!(uses(&spec(key)), None, "{key}");
    }
}

/// Reserving ⌘P and ⌘S for Print and Export Text As keeps them from projects and the hotkey,
/// though nothing binds them yet.
#[test]
fn print_and_export_keys_are_kept_for_terminal_app_meanings() {
    for (key, id, said) in [
        ("⌘P", ActionId::Print, "⌘P is kept for Print, as in Terminal.app"),
        ("⌘S", ActionId::ExportText, "⌘S is kept for Export Text As, as in Terminal.app"),
    ] {
        let used = uses(&spec(key));
        assert!(matches!(used, Some(KeyUse::Action(e, _)) if e.id == id), "{key}: {used:?}");
        assert_eq!(used.map(|u| u.to_string()).as_deref(), Some(said));
        let entry = entry(id).expect("a row");
        assert_eq!(entry.stage, Stage::Reserved);
        assert!(!entry.palette && !terminal_yields(&spec(key)) && terminal_action(&spec(key)).is_none());
    }
    let shown = dump(&menus(&front_window()));
    assert!(!shown.contains("Print") && !shown.contains("Export"), "in no menu yet");
}

/// The Theme Studio's ⌘S and ⇧⌘S are rows of the table, which
/// its window reads, so the sheet can list them.
#[test]
fn the_studio_keys_are_in_the_table() {
    assert_eq!(studio_action(&spec("⌘S")), Some(ActionId::SaveTheme));
    assert_eq!(studio_action(&spec("⇧⌘S")), Some(ActionId::SaveThemeCopy));
    assert_eq!(studio_action(&spec("⌥⌘S")), None, "the stash key is a terminal window's");
    assert_eq!(terminal_action(&spec("⌘S")), None, "the Studio's keys stay in the Studio");
    assert_eq!(list_action(&spec("⌘S")), None);
    for id in [ActionId::SaveTheme, ActionId::SaveThemeCopy] {
        let entry = entry(id).expect("a row");
        assert!(entry.is_live() && !entry.palette, "{id:?}");
    }
}

#[test]
fn no_two_actions_share_a_key_where_both_apply() {
    let all = entries();
    for (ix, a) in all.iter().enumerate() {
        for b in &all[ix + 1..] {
            if !a.scope.overlaps(b.scope) {
                continue;
            }
            for key in &a.keys {
                assert!(
                    !b.keys.iter().any(|k| k.same_key(key)),
                    "{} is {:?} and {:?}",
                    key.glyphs(),
                    a.id,
                    b.id
                );
            }
        }
    }
}

#[test]
fn app_keys_carry_command_except_f6() {
    for entry in entries().iter().filter(|e| e.scope == Scope::App) {
        for key in &entry.keys {
            let f6 = key.key == "f6" && !key.mods.cmd && !key.mods.ctrl && !key.mods.alt;
            // The groups' keys (⌃⌥[, ⌃⌥], ⌃⌥1–9) are the boards' choice: the terminal yields them.
            let group =
                matches!(entry.id, ActionId::PreviousGroup | ActionId::NextGroup | ActionId::Group(_))
                    && key.mods.ctrl
                    && key.mods.alt
                    && !key.mods.shift;
            assert!(key.mods.cmd || f6 || group, "{:?}: {} has no ⌘", entry.id, key.glyphs());
        }
    }
}

#[test]
fn option_and_control_letters_and_system_keys_are_never_bound() {
    for entry in entries() {
        for key in &entry.keys {
            let letter = key.key.chars().count() == 1 && key.key.chars().all(|c| c.is_ascii_alphabetic());
            assert!(!(letter && !key.mods.cmd && (key.mods.alt || key.mods.ctrl)), "{:?}", entry.id);
            assert!(uses_system(key).is_none(), "{:?}: {} is the system's", entry.id, key.glyphs());
        }
    }
    for key in ["⌘Space", "⌘⇥", "⌘`", "⌃↑", "⌃↓", "⌃←", "⌃→", "⇧⌘3", "⇧⌘4", "⇧⌘5", "⌃⌘Q", "⌃⌘Space", "⌃⌘D"]
    {
        assert!(matches!(uses(&spec(key)), Some(KeyUse::System(_))), "{key}");
    }
}

fn uses_system(key: &KeySpec) -> Option<&'static str> {
    super::SYSTEM.iter().find(|(k, _)| k.same_key(key)).map(|(_, what)| *what)
}

#[test]
fn every_key_round_trips_through_its_glyphs() {
    for entry in entries() {
        for key in &entry.keys {
            assert_eq!(KeySpec::from_glyphs(&key.glyphs()).as_ref(), Some(key), "{:?}", entry.id);
            let text = key.to_string();
            assert_eq!(text.parse::<KeySpec>().as_ref(), Ok(key), "{text}");
        }
    }
    assert_eq!(glyphs(ActionId::Terminal(1)), "⌘1");
    assert_eq!(with_key("Hide sidebar", ActionId::ToggleSidebar), "Hide sidebar (⌃⌘S)");
    assert_eq!(with_key("Close terminal", ActionId::CloseTerminal), "Close terminal (⌘W)");
    assert_eq!(with_key("Pin", ActionId::PinTerminal), "Pin", "no key, nothing in brackets");
}

#[test]
fn who_has_a_key() {
    assert_eq!(uses(&spec("⌘T")).map(|u| u.to_string()).as_deref(), Some("⌘T is New Terminal"));
    assert_eq!(uses(&spec("⌃⌘S")).map(|u| u.to_string()).as_deref(), Some("⌃⌘S is Show / Hide Sidebar"));
    // A US keyboard's ⌘} is ⇧⌘].
    assert_eq!(
        uses(&KeySpec::new(spec("⌘T").mods, "}")).map(|u| u.to_string()).as_deref(),
        Some("⇧⌘] is Next Terminal")
    );
    assert_eq!(uses(&spec("⇧⌘P")).map(|u| u.to_string()).as_deref(), Some("⇧⌘P is Command Palette…"));
    assert_eq!(uses(&spec("⌘Space")).map(|u| u.to_string()).as_deref(), Some("macOS uses it for Spotlight"));
    assert!(matches!(uses(&spec("⌥F")), Some(KeyUse::Shell(_))));
    assert!(matches!(uses(&spec("⌃T")), Some(KeyUse::Shell(_))));
    assert!(matches!(uses(&spec("T")), Some(KeyUse::Shell(_))));
    assert_eq!(uses(&spec("F5")), None);
    assert_eq!(uses(&spec("⇧F5")), None);
    assert!(matches!(uses(&spec("F6")), Some(KeyUse::Action(e, _)) if e.id == ActionId::FocusNextArea));
}

#[test]
fn what_a_focused_terminal_keeps_and_gives_up() {
    assert!(
        terminal_yields(&spec("F6")) && terminal_yields(&spec("⇧F6")),
        "focus moves even from a terminal"
    );
    assert!(!terminal_yields(&spec("F5")), "other F-keys are the program's");
    assert!(terminal_yields(&spec("⇧⌘P")), "the palette opens from the terminal too");
    assert_eq!(terminal_action(&spec("⌘C")), Some(ActionId::Copy));
    assert_eq!(terminal_action(&spec("⌘K")), Some(ActionId::ClearBuffer));
    assert_eq!(terminal_action(&spec("⌘⌫")), Some(ActionId::DeleteLine));
    assert_eq!(terminal_action(&spec("⌘←")), Some(ActionId::LineStart));
    assert_eq!(terminal_action(&spec("⌘↑")), Some(ActionId::PreviousPrompt));
    assert_eq!(terminal_action(&spec("⌘↓")), Some(ActionId::NextPrompt));
    assert_eq!(terminal_action(&spec("⌘T")), None, "⌘T belongs to the app");
    assert_eq!(terminal_action(&spec("⌘1")), None, "so do the digits");
    assert_eq!(list_action(&spec("⌥⌘↑")), Some(ActionId::MoveItemUp));
    assert_eq!(list_action(&spec("⌥⌘↓")), Some(ActionId::MoveItemDown));
    assert_eq!(list_action(&spec("⌘↑")), None);
    // ⌃↩ opens the row's menu in a list; a focused terminal keeps it.
    assert_eq!(list_action(&spec("⌃↩")), Some(ActionId::RowMenu));
    assert!(!terminal_yields(&spec("⌃↩")) && terminal_action(&spec("⌃↩")).is_none());
}

fn front_window() -> MenuState {
    use ActionId::*;
    MenuState {
        off: vec![ToggleStatusBar, ToggleMinimap, FullScreen, ZoomPane],
        checked: vec![ShowChanges],
        disabled: vec![],
        hidden: vec![],
        terminals: ["sidebar polish", "diff perf", "zsh", "auth tokens", "rate limits"]
            .map(String::from)
            .to_vec(),
        current_terminal: Some(0),
        recent: vec![],
        project_actions: vec![],
        shells: vec![],
        profiles: vec!["Default".into(), "Work".into()],
        current_profile: Some(0),
        window_profile: Some(0),
        split: false,
        debug: true,
        persistent: false,
    }
}

/// The menu bar for the window in front. Not in it yet, for now: Find (backlog),
/// Move to Folder and Open in New Window (backlog), Wormhole Help (the system's search until
/// there is help to show) and Release Notes (nothing to show yet). Show
/// Status Bar is ⌘/, so Keyboard Shortcuts is ⌥⌘/. Also in it: New Terminal With, the shell
/// profiles (as Terminal.app's "New Tab with Profile"), and the panes: Split Right /
/// Down and Move Pane Out of Split under the Shell menu's New items, Zoom Pane and the Layout
/// submenu in View, Select Pane in Window. The tabs are gone: so are New Tab, Close Tab and Next /
/// Previous Tab; ⌘T makes a terminal and ⌘W closes the selected one.
#[test]
fn the_menus_are_the_designed_ones() {
    let expected = "\
Wormhole
  About Wormhole
  Check for Updates…
  ---
  Settings…  ⌘,
  Theme Studio  ⌥⌘,
  ---
  Services ▸
  ---
  Hide Wormhole  ⌘H
  Hide Others  ⌥⌘H
  Show All
  ---
  Quit Wormhole  ⌘Q
Shell
  New Window  ⌘N
  New Terminal  ⌘T
  New Terminal in No Group  ⇧⌘T
  New Terminal With ▸
    No Shells  (disabled)
  New Terminal in New Worktree…
  ---
  Split Right  ⌘D
  Split Down  ⇧⌘D
  Move Pane Out of Split
  ---
  Open Folder…  ⌘O
  Open Recent ▸
    No Recent Folders  (disabled)
  ---
  Profile ▸
    ✓ Default
    Work
    ---
    Open Window in ▸
      Default
      Work
    Move Window to ▸
      Work
    ---
    New Profile…
    Manage Profiles…
  ---
  Close Terminal  ⌘W
  Close Window  ⇧⌘W
Edit
  Undo  ⌘Z
  Redo  ⇧⌘Z
  ---
  Cut  ⌘X
  Copy  ⌘C
  Paste  ⌘V
  Select All  ⌘A
  ---
  Clear Buffer  ⌘K
  ---
  Find…  ⌘F
  Find Next  ⌘G
  Find Previous  ⇧⌘G
  Use Selection for Find  ⌘E
  ---
  Previous Prompt  ⌘↑
  Next Prompt  ⌘↓
  Select Output
  Copy Output
  Copy Command
  Put Command at Prompt
View
  Hide Sidebar  ⌃⌘S
  Hide Right Panel  ⌥⌘I
  Show Status Bar  ⌘/
  Show Minimap
  ---
  Zoom Pane  ⇧⌘↩
  Layout ▸
    2 Columns
    2 Rows
    2×2 Grid
    Single Pane
  ---
  ✓ Changes  ⌃⌘1
  History  ⌃⌘2
  Notes  ⌃⌘3
  Usage  ⌃⌘4
  Project  ⌃⌘5
  ---
  Show Terminals in Sidebar
  ---
  Bigger  ⌘+
  Smaller  ⌘-
  Actual Size  ⌘0
  ---
  Command Palette…  ⇧⌘P
  ---
  Enter Full Screen  ⌃⌘F
Terminal
  Pin to Top
  ---
  Focus Terminal / Panel  ⇧⌘E
  Focus Next Area  F6
  Focus Previous Area  ⇧F6
  New Note  ⌥⌘N
  Stash Prompt  ⌥⌘S
  Project Actions ▸
    No Project Actions  (disabled)
  ---
  Reveal in Finder
  Copy Path
Window
  Minimize  ⌘M
  Zoom
  ---
  Next Terminal  ⌃⌘↓
  Previous Terminal  ⌃⌘↑
  Next Group  ⌃⌥]
  Previous Group  ⌃⌥[
  Go to Waiting Terminal  ⌥⌘J
  Go to Finished Terminal  ⌥⇧⌘J
  Select Pane ▸
    Left  ⌥⌘←
    Right  ⌥⌘→
    Above  ⌥⌘↑
    Below  ⌥⌘↓
  ---
  ✓ sidebar polish  ⌘1
  diff perf  ⌘2
  zsh  ⌘3
  auth tokens  ⌘4
  rate limits  ⌘5
  ---
  Bring All to Front
Help
  Keyboard Shortcuts  ⌥⌘/
  Welcome to Wormhole
  ---
  Licenses
  Show Logs in Finder
Developer
  Inspector  ⌥⇧⌘I
";
    assert_eq!(dump(&menus(&front_window())), expected);
}

/// Quit and End All Terminals is about terminals that outlive the app, so it exists only while
/// persistent terminals are on: under Quit in the Wormhole menu, in the palette, with no key.
#[test]
fn quit_and_end_all_terminals_needs_persistent_terminals() {
    use super::palette;
    let at_wormhole_menu = |state: &MenuState| {
        let all = menus(state);
        let items = all.iter().find(|m| m.title == "Wormhole").expect("the app menu").items.clone();
        items
            .iter()
            .filter_map(|item| match item {
                Item::Action { command: Command::Action(id), title, .. } => Some((*id, title.clone())),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let mut state = front_window();
    assert!(!at_wormhole_menu(&state).iter().any(|(id, _)| *id == ActionId::QuitAndEndAll));
    assert!(!palette::actions(&state).iter().any(|a| a.id == ActionId::QuitAndEndAll));
    state.persistent = true;
    let items = at_wormhole_menu(&state);
    let quit = items.iter().position(|(id, _)| *id == ActionId::Quit).expect("Quit");
    assert_eq!(items[quit + 1], (ActionId::QuitAndEndAll, "Quit and End All Terminals".to_string()));
    assert!(palette::actions(&state).iter().any(|a| a.id == ActionId::QuitAndEndAll));
    assert!(keys_of(ActionId::QuitAndEndAll).is_empty(), "no key");
    assert!(dump(&menus(&state)).contains("  Quit Wormhole  ⌘Q\n  Quit and End All Terminals\n"));
}

#[test]
fn menus_follow_the_window_in_front() {
    use ActionId::*;
    let mut state = front_window();
    state.off = vec![ToggleSidebar, ToggleRightPanel];
    state.debug = false;
    state.recent = vec!["terminal".into(), "dotfiles".into()];
    state.project_actions = vec![("test".into(), "Run tests".into())];
    state.shells = vec!["zsh".into(), "fish".into(), "Claude".into()];
    state.disabled = vec![ShowProject];
    state.split = true;
    state.checked.push(LayoutGrid);
    let text = dump(&menus(&state));
    for line in [
        "Show Sidebar  ⌃⌘S",
        "Close Pane  ⌘W",
        "Unzoom Pane  ⇧⌘↩",
        "✓ 2×2 Grid",
        "Show Right Panel  ⌥⌘I",
        "Hide Status Bar  ⌘/",
        "Hide Minimap\n",
        "Exit Full Screen  ⌃⌘F",
        "Project  ⌃⌘5  (disabled)",
        "Open Recent ▸\n    terminal\n    dotfiles",
        "Project Actions ▸\n    Run tests",
        "New Terminal With ▸\n    zsh\n    fish\n    Claude\n",
    ] {
        assert!(text.contains(line), "{line:?} in\n{text}");
    }
    assert!(!text.contains("Developer"), "only in debug builds");
    let menus = menus(&state);
    let shell = menus.iter().find(|m| m.title == "Shell").expect("a Shell menu");
    let submenu = |title: &str| match shell
        .items
        .iter()
        .find(|i| matches!(i, Item::Submenu { title: t, .. } if t == title))
    {
        Some(Item::Submenu { items, .. }) => items.clone(),
        other => panic!("{title}: {other:?}"),
    };
    assert!(matches!(&submenu("Open Recent")[1], Item::Action { command: Command::OpenRecent(1), .. }));
    // Each shell item names its place in the list, which the app turns back into the shell's key.
    assert!(matches!(
        &submenu("New Terminal With")[2],
        Item::Action { command: Command::NewTerminalWith(2), .. }
    ));
}

#[test]
fn a_renamed_terminal_is_a_change_of_names_only() {
    let before = front_window();
    let mut renamed = before.clone();
    renamed.terminals[1] = "⠂ diff perf".into();
    assert!(before.differs_only_in_names(&renamed));
    assert!(!before.differs_only_in_names(&before), "no change at all");
    let mut moved = renamed.clone();
    moved.current_terminal = Some(1);
    assert!(!before.differs_only_in_names(&moved), "the check mark moved");
    let mut fewer = before.clone();
    fewer.terminals.pop();
    assert!(!before.differs_only_in_names(&fewer), "a terminal went away");
    let mut shells = renamed.clone();
    shells.shells.push("fish".into());
    assert!(!before.differs_only_in_names(&shells), "a shell was added");
}

#[test]
fn every_menu_item_is_a_live_action_and_separators_are_tidy() {
    fn walk(items: &[Item]) {
        assert!(items.first() != Some(&Item::Separator) && items.last() != Some(&Item::Separator));
        for pair in items.windows(2) {
            assert!(!(pair[0] == Item::Separator && pair[1] == Item::Separator));
        }
        for item in items {
            match item {
                Item::Action { command: Command::Action(id), .. } => {
                    let entry = entry(*id).unwrap_or_else(|| panic!("{id:?} has a row"));
                    assert_eq!(entry.stage, Stage::Now, "{id:?}");
                }
                Item::Submenu { items, .. } => walk(items),
                _ => {}
            }
        }
    }
    let mut state = front_window();
    state.terminals.clear();
    for menu in menus(&state) {
        assert!(!menu.items.is_empty(), "{}", menu.title);
        walk(&menu.items);
    }
}

#[test]
fn the_sheet_lists_the_keys_from_the_table() {
    let sections = sections();
    let titles: Vec<_> = sections.iter().map(|s| s.title).collect();
    assert_eq!(titles, ["Windows & terminals", "View & panels", "Terminal", "Find anything", "Notes", "App"]);
    let row = |label: &str| {
        sections
            .iter()
            .flat_map(|s| &s.rows)
            .find(|r| r.label == label)
            .unwrap_or_else(|| panic!("{label} is on the sheet"))
            .keys_text()
    };
    assert_eq!(row("Previous / next terminal"), "⌃⌘↑ ⌃⌘↓");
    assert_eq!(row("Terminals 1–9 (hold ⌘ to see them)"), "⌘1 … ⌘9");
    assert_eq!(row("Changes · History · Notes · Usage · Project"), "⌃⌘1 … ⌃⌘5");
    // With notes and project folders off (a release build), the row names and keys what is left.
    let hidden = sections_without(&[ActionId::ShowNotes, ActionId::ShowProject, ActionId::NewNote]);
    let tabs = hidden.iter().flat_map(|s| &s.rows).find(|r| r.label.starts_with("Changes")).expect("tab row");
    assert_eq!(
        (tabs.label.as_ref(), tabs.keys_text().as_str()),
        ("Changes · History · Usage", "⌃⌘1 ⌃⌘2 ⌃⌘4")
    );
    assert_eq!(row("Next / previous area"), "F6 ⇧F6");
    assert_eq!(row("Bigger / smaller / actual size"), "⌘+ ⌘- ⌘0");
    assert_eq!(row("Hide / show status bar"), "⌘/");
    // The panes; the layouts have no keys and so no row.
    assert_eq!(row("Split right / down"), "⌘D ⇧⌘D");
    assert_eq!(row("Zoom pane"), "⇧⌘↩");
    assert_eq!(row("Pane left / right / above / below"), "⌥⌘← ⌥⌘→ ⌥⌘↑ ⌥⌘↓");
    assert_eq!(row("Close terminal (pane in a split)"), "⌘W");
    // Find, in the Terminal section.
    assert_eq!(row("Find in scrollback"), "⌘F");
    assert_eq!(row("Find next / previous"), "⌘G ⇧⌘G");
    assert_eq!(row("Use selection for find"), "⌘E");
    // The command blocks: the jump keys are the table's; the rest have no key and stay off the sheet.
    assert_eq!(row("Previous / next prompt"), "⌘↑ ⌘↓");
    assert!(!sections.iter().flat_map(|s| &s.rows).any(|r| r.actions.contains(&ActionId::CopyOutput)));
    assert_eq!(row("Wormhole help"), "⌘?");
    assert_eq!(row("Go to the terminal that needs input"), "⌥⌘J");
    // The palette and sheet rows.
    assert_eq!(row("Command palette"), "⇧⌘P");
    assert_eq!(row("Keyboard shortcuts (this sheet)"), "⌥⌘/");
    // The Studio's own keys.
    assert_eq!(row("Save theme (in the Studio)"), "⌘S");
    assert_eq!(row("Save Copy (in the Studio)"), "⇧⌘S");
}

/// An empty Open Recent or Project Actions says so in a greyed item, as Finder does.
#[test]
fn empty_submenus_say_what_they_would_hold() {
    let menus = menus(&front_window());
    let submenu = |menu: &str, title: &str| -> Vec<Item> {
        let menu = menus.iter().find(|m| m.title == menu).expect("menu");
        match menu.items.iter().find(|i| matches!(i, Item::Submenu { title: t, .. } if t == title)) {
            Some(Item::Submenu { items, enabled: true, .. }) => items.clone(),
            other => panic!("{title}: {other:?}"),
        }
    };
    assert_eq!(submenu("Shell", "Open Recent"), [Item::Placeholder("No Recent Folders")]);
    assert_eq!(submenu("Terminal", "Project Actions"), [Item::Placeholder("No Project Actions")]);
    assert_eq!(submenu("Shell", "New Terminal With"), [Item::Placeholder("No Shells")]);
}

/// The sheet is the table seen through its sections: every key on it is an action's key in the
/// table, so a key changed there changes here, and a row whose action isn't live isn't shown.
#[test]
fn the_sheet_is_a_projection_of_the_table() {
    for section in sections() {
        for row in &section.rows {
            let entries: Vec<_> = row.actions.iter().map(|id| entry(*id).expect("a table row")).collect();
            assert!(entries.iter().all(|e| matches!(e.stage, Stage::Now | Stage::System)), "{}", row.label);
            let first: Vec<KeySpec> = entries.iter().filter_map(|e| e.key().cloned()).collect();
            let shown: Vec<KeySpec> = match row.style {
                Keys::Each => first,
                Keys::Range => [first.first(), first.last()].into_iter().flatten().cloned().collect(),
            };
            assert_eq!(row.keys, shown, "{}", row.label);
        }
    }
}

#[test]
fn the_sheet_filter_finds_labels_and_keys() {
    let sheet = sections();
    let labels = |query: &str| -> Vec<String> {
        filter(&sheet, query)
            .sections
            .iter()
            .flat_map(|(_, rows)| rows.iter().map(|(r, _)| r.label.to_string()))
            .collect()
    };
    // "⌘W" finds Close terminal, and not Close window's ⇧⌘W.
    assert_eq!(labels("⌘W"), ["Close terminal (pane in a split)"]);
    assert_eq!(labels("⇧⌘W"), ["Close window"]);
    // A key inside a range.
    assert_eq!(labels("⌘5"), ["Terminals 1–9 (hold ⌘ to see them)"]);
    // Text: in the label (marked) or the keys as written.
    let split = filter(&sheet, "split");
    assert_eq!(split.sections.len(), 1, "matches stay in their section");
    assert_eq!(split.sections[0].0, "Windows & terminals");
    let marked: Vec<&str> = split.sections[0]
        .1
        .iter()
        .filter_map(|(row, hit)| hit.label.clone().map(|r| &row.label[r]))
        .collect();
    assert_eq!(marked, ["split", "Split"]);
    assert_eq!(labels("f6"), ["Next / previous area"]);
    // The count the footer gives.
    assert_eq!(split.shown, 2);
    assert_eq!(filter(&sheet, "").shown, filter(&sheet, "").total);
    assert!(filter(&sheet, "zebra").sections.is_empty());
}

#[test]
fn the_sheet_balances_its_columns() {
    // Windows… alone, View & panels with Terminal, the last three together.
    let heights: Vec<usize> = sections().iter().map(|s| s.rows.len() + 1).collect();
    assert_eq!(columns(&heights, 3), [0..1, 1..3, 3..6]);
    assert_eq!(columns(&[3, 3, 3], 3), [0..1, 1..2, 2..3]);
    assert_eq!(columns(&[5], 3).as_slice(), std::slice::from_ref(&(0..1)), "never an empty column");
    assert!(columns(&[], 3).is_empty());
}
