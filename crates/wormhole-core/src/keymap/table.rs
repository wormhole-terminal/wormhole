//! The rows of the action table, in order. Each key is written as people read it.

use super::{ActionId, Entry, Scope, Stage};
use crate::keyspec::{KeyMods, KeySpec};

const fn mods(ctrl: bool, alt: bool, shift: bool, cmd: bool) -> KeyMods {
    KeyMods { cmd, ctrl, alt, shift }
}

const PLAIN: KeyMods = mods(false, false, false, false);
const SHIFT: KeyMods = mods(false, false, true, false);
const CTRL: KeyMods = mods(true, false, false, false);
const CMD: KeyMods = mods(false, false, false, true);
const SHIFT_CMD: KeyMods = mods(false, false, true, true);
const ALT_CMD: KeyMods = mods(false, true, false, true);
const CTRL_CMD: KeyMods = mods(true, false, false, true);
const SHIFT_ALT_CMD: KeyMods = mods(false, true, true, true);
const CTRL_ALT_CMD: KeyMods = mods(true, true, false, true);

const TERMINAL_TITLES: [&str; 9] = [
    "Terminal 1",
    "Terminal 2",
    "Terminal 3",
    "Terminal 4",
    "Terminal 5",
    "Terminal 6",
    "Terminal 7",
    "Terminal 8",
    "Terminal 9",
];
const DIGITS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

/// A row: an app action until said otherwise, in the palette, live.
fn row(id: ActionId, title: &'static str, keys: &[(KeyMods, &str)]) -> Entry {
    Entry {
        id,
        title,
        title_off: None,
        name: None,
        keys: keys.iter().map(|&(mods, key)| KeySpec::new(mods, key)).collect(),
        scope: Scope::App,
        stage: Stage::Now,
        palette: true,
        debug_only: false,
    }
}

impl Entry {
    fn scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    fn system(mut self) -> Self {
        self.stage = Stage::System;
        self
    }

    fn reserved(mut self) -> Self {
        self.stage = Stage::Reserved;
        self
    }

    fn not_in_palette(mut self) -> Self {
        self.palette = false;
        self
    }

    /// A toggle: `title` while on, `off` while off, `name` elsewhere.
    fn toggle(mut self, off: &'static str, name: &'static str) -> Self {
        self.title_off = Some(off);
        self.name = Some(name);
        self
    }

    fn named(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    fn debug_only(mut self) -> Self {
        self.debug_only = true;
        self
    }
}

pub(super) fn entries() -> Vec<Entry> {
    use ActionId::*;
    let mut rows = vec![
        // Windows and terminals. A terminal is a row of the sidebar; there are no tabs.
        row(NewWindow, "New Window", &[(CMD, "n")]),
        // ⇧⌘N, the key it had before ⌘T became its own, still works.
        row(NewTerminal, "New Terminal", &[(CMD, "t"), (SHIFT_CMD, "n")]),
        row(NewTerminalInWorktree, "New Terminal in New Worktree…", &[]),
        row(OpenFolder, "Open Folder…", &[(CMD, "o")]),
        row(PinTerminal, "Pin to Top", &[]),
        row(RevealInFinder, "Reveal in Finder", &[]),
        row(CopyPath, "Copy Path", &[]),
        // The selected terminal, also inside a split; the menu says "Close Pane" there. No key
        // for the others: ⌥⌘W closes all or the other tabs in Finder, Safari and Terminal.app.
        row(CloseTerminal, "Close Terminal", &[(CMD, "w")]),
        row(CloseWindow, "Close Window", &[(SHIFT_CMD, "w")]),
        // The keys the tabs had (⇧⌘] and ⇧⌘[) step through the terminals now.
        row(NextTerminal, "Next Terminal", &[(CTRL_CMD, "down"), (SHIFT_CMD, "]")]),
        row(PreviousTerminal, "Previous Terminal", &[(CTRL_CMD, "up"), (SHIFT_CMD, "[")]),
    ];
    for (n, (title, digit)) in TERMINAL_TITLES.iter().zip(DIGITS).enumerate() {
        rows.push(row(Terminal(n as u8 + 1), title, &[(CMD, digit)]).not_in_palette());
    }
    rows.extend([
        // Panes: iTerm2's keys for the splits and the zoom; ⌥⌘arrows move between panes
        // while a terminal has the keyboard (a list's ⌥⌘↑ / ⌥⌘↓ move its item). The layouts
        // have no keys.
        row(SplitRight, "Split Right", &[(CMD, "d")]),
        row(SplitDown, "Split Down", &[(SHIFT_CMD, "d")]),
        row(ZoomPane, "Unzoom Pane", &[(SHIFT_CMD, "enter")]).toggle("Zoom Pane", "Zoom Pane"),
        row(LeaveSplit, "Move Pane Out of Split", &[]),
        row(FocusPaneLeft, "Left", &[(ALT_CMD, "left")])
            .scope(Scope::Terminal)
            .named("Select Pane on the Left"),
        row(FocusPaneRight, "Right", &[(ALT_CMD, "right")])
            .scope(Scope::Terminal)
            .named("Select Pane on the Right"),
        row(FocusPaneUp, "Above", &[(ALT_CMD, "up")]).scope(Scope::Terminal).named("Select Pane Above"),
        row(FocusPaneDown, "Below", &[(ALT_CMD, "down")]).scope(Scope::Terminal).named("Select Pane Below"),
        row(LayoutColumns, "2 Columns", &[]).named("Layout: 2 Columns"),
        row(LayoutRows, "2 Rows", &[]).named("Layout: 2 Rows"),
        row(LayoutGrid, "2×2 Grid", &[]).named("Layout: 2×2 Grid"),
        row(LayoutSingle, "Single Pane", &[]).named("Layout: Single Pane"),
        // The default: the user may record another key (`jump_waiting_key`), which the app binds.
        row(GoToWaiting, "Go to Waiting Terminal", &[(ALT_CMD, "j")]),
        row(Minimize, "Minimize", &[(CMD, "m")]),
        row(Zoom, "Zoom", &[]),
        row(BringAllToFront, "Bring All to Front", &[]),
        // View and panels.
        row(ToggleSidebar, "Hide Sidebar", &[(CTRL_CMD, "s")]).toggle("Show Sidebar", "Show / Hide Sidebar"),
        row(ToggleRightPanel, "Hide Right Panel", &[(ALT_CMD, "i")])
            .toggle("Show Right Panel", "Show / Hide Right Panel"),
        // ⌘/ shows the optional status bar, the sheet moved to ⌥⌘/.
        row(ToggleStatusBar, "Hide Status Bar", &[(CMD, "/")])
            .toggle("Show Status Bar", "Show / Hide Status Bar"),
        row(ToggleMinimap, "Hide Minimap", &[]).toggle("Show Minimap", "Show / Hide Minimap"),
        row(ShowChanges, "Changes", &[(CTRL_CMD, "1")]).named("Show Changes"),
        row(ShowHistory, "History", &[(CTRL_CMD, "2")]).named("Show History"),
        row(ShowNotes, "Notes", &[(CTRL_CMD, "3")]).named("Show Notes"),
        row(ShowUsage, "Usage", &[(CTRL_CMD, "4")]).named("Show Usage"),
        row(ShowProject, "Project", &[(CTRL_CMD, "5")]).named("Show Project"),
        row(GroupByFolder, "Folder", &[]).named("Group Terminals by Folder"),
        row(GroupByNone, "None", &[]).named("Don't Group Terminals"),
        row(CompactRows, "Compact Rows", &[]),
        row(FocusTerminalOrPanel, "Focus Terminal / Panel", &[(SHIFT_CMD, "e")]),
        row(FocusNextArea, "Focus Next Area", &[(PLAIN, "f6")]),
        row(FocusPreviousArea, "Focus Previous Area", &[(SHIFT, "f6")]),
        // ⌘+ is ⇧⌘= on a US keyboard; ⌘= without ⇧ works too, as in Terminal.app.
        row(Bigger, "Bigger", &[(CMD, "+"), (CMD, "=")]),
        row(Smaller, "Smaller", &[(CMD, "-")]),
        row(ActualSize, "Actual Size", &[(CMD, "0")]),
        row(FullScreen, "Exit Full Screen", &[(CTRL_CMD, "f")]).toggle("Enter Full Screen", "Full Screen"),
        row(CommandPalette, "Command Palette…", &[(SHIFT_CMD, "p")]).not_in_palette(),
        row(KeyboardShortcuts, "Keyboard Shortcuts", &[(ALT_CMD, "/")]),
        row(Help, "Wormhole Help", &[(CMD, "?")]).system(),
        // Editing: text fields answer these, and so does the terminal (copy, paste, select all).
        row(Undo, "Undo", &[(CMD, "z")]).scope(Scope::Edit).not_in_palette(),
        row(Redo, "Redo", &[(SHIFT_CMD, "z")]).scope(Scope::Edit).not_in_palette(),
        row(Cut, "Cut", &[(CMD, "x")]).scope(Scope::Edit).not_in_palette(),
        row(Copy, "Copy", &[(CMD, "c")]).scope(Scope::Edit).not_in_palette(),
        row(Paste, "Paste", &[(CMD, "v")]).scope(Scope::Edit).not_in_palette(),
        row(SelectAll, "Select All", &[(CMD, "a")]).scope(Scope::Edit).not_in_palette(),
        row(ClearBuffer, "Clear Buffer", &[(CMD, "k")]).scope(Scope::Terminal),
        // Find in scrollback, Apple's keys. App keys: they reach the terminal in front from
        // anywhere in the window, its find bar included. ⇧↩ and ↩ step in the bar too.
        row(Find, "Find…", &[(CMD, "f")]),
        row(FindNext, "Find Next", &[(CMD, "g")]).not_in_palette(),
        row(FindPrevious, "Find Previous", &[(SHIFT_CMD, "g")]).not_in_palette(),
        row(UseSelectionForFind, "Use Selection for Find", &[(CMD, "e")]).not_in_palette(),
        // As Terminal.app and iTerm2: sent to the shell as ⌃A, ⌃E and ⌃U.
        row(LineStart, "Line Start", &[(CMD, "left")]).scope(Scope::Terminal).not_in_palette(),
        row(LineEnd, "Line End", &[(CMD, "right")]).scope(Scope::Terminal).not_in_palette(),
        row(DeleteLine, "Delete Line", &[(CMD, "backspace")]).scope(Scope::Terminal).not_in_palette(),
        // Command blocks: ⌘↑ / ⌘↓ are Terminal.app's marks. The rest act on the
        // current block (the one jumped to, else the last finished one) and have no key.
        row(PreviousPrompt, "Previous Prompt", &[(CMD, "up")]).scope(Scope::Terminal).not_in_palette(),
        row(NextPrompt, "Next Prompt", &[(CMD, "down")]).scope(Scope::Terminal).not_in_palette(),
        row(SelectOutput, "Select Output", &[]).scope(Scope::Terminal),
        row(CopyOutput, "Copy Output", &[]).scope(Scope::Terminal),
        row(CopyCommand, "Copy Command", &[]).scope(Scope::Terminal),
        row(RerunCommand, "Rerun Command", &[]).scope(Scope::Terminal),
        // Notes.
        row(NewNote, "New Note", &[(ALT_CMD, "n")]),
        row(StashPrompt, "Stash Prompt", &[(ALT_CMD, "s")]),
        row(MoveItemUp, "Move Up", &[(ALT_CMD, "up")]).scope(Scope::List).not_in_palette(),
        row(MoveItemDown, "Move Down", &[(ALT_CMD, "down")]).scope(Scope::List).not_in_palette(),
        // The "Menu" key, ⌃↩ as in macOS's lists under Full Keyboard
        // Access. A focused terminal keeps it.
        row(RowMenu, "Show Context Menu", &[(CTRL, "enter")]).scope(Scope::List).not_in_palette(),
        // The app.
        row(About, "About Wormhole", &[]),
        row(CheckForUpdates, "Check for Updates…", &[]),
        row(Settings, "Settings…", &[(CMD, ",")]),
        row(ThemeStudio, "Theme Studio", &[(ALT_CMD, ",")]),
        row(Hide, "Hide Wormhole", &[(CMD, "h")]),
        row(HideOthers, "Hide Others", &[(ALT_CMD, "h")]),
        row(ShowAll, "Show All", &[]),
        row(Quit, "Quit Wormhole", &[(CMD, "q")]),
        row(ShowLogs, "Show Logs in Finder", &[]),
        row(ShowIntro, "Welcome to Wormhole", &[]),
        // While three intro styles are on trial: a key each, to compare them quickly.
        row(PlayIntroEmerge, "Play Intro: Emerge", &[(CTRL_ALT_CMD, "1")]),
        row(PlayIntroVortex, "Play Intro: Edge Vortex", &[(CTRL_ALT_CMD, "2")]),
        row(PlayIntroCollapse, "Play Intro: Collapse", &[(CTRL_ALT_CMD, "3")]),
        row(PlayIntroFtl, "Play Intro: FTL Jump", &[(CTRL_ALT_CMD, "4")]),
        row(PlayIntroAurora, "Play Intro: Aurora", &[(CTRL_ALT_CMD, "5")]),
        row(PlayIntroTentacles, "Play Intro: Tentacles", &[(CTRL_ALT_CMD, "6")]),
        row(PlayIntroVoyage, "Play Intro: Voyage", &[(CTRL_ALT_CMD, "7")]),
        row(Inspector, "Inspector", &[(SHIFT_ALT_CMD, "i")]).debug_only().not_in_palette(),
        // The freed keys that stay Terminal.app's: nothing binds them, nobody else may.
        row(Print, "Print…", &[(CMD, "p")])
            .scope(Scope::Terminal)
            .reserved()
            .named("kept for Print, as in Terminal.app")
            .not_in_palette(),
        row(ExportText, "Export Text As…", &[(CMD, "s")])
            .scope(Scope::Terminal)
            .reserved()
            .named("kept for Export Text As, as in Terminal.app")
            .not_in_palette(),
        // The Theme Studio's own window reads these (`studio_action`); the sheet lists them.
        row(SaveTheme, "Save Theme", &[(CMD, "s")]).scope(Scope::Studio).not_in_palette(),
        row(SaveThemeCopy, "Save Copy", &[(SHIFT_CMD, "s")]).scope(Scope::Studio).not_in_palette(),
    ]);
    rows
}
