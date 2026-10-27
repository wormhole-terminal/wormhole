//! The app's actions and their keys: one table, read by the key bindings, the menu bar,
//! the command palette and the shortcut sheet, so none of them can drift from the
//! others. It is the source of truth for every key: macOS's own keys first, Wormhole's
//! around them, and every app key carries ⌘ except F6 / ⇧F6.
//!
//! Keys are [`KeySpec`]s written the way people read them (`⇧⌘]`); a US keyboard reports some of
//! them differently (`⌘}`), which [`KeySpec::same_key`] and [`KeySpec::shifted_spelling`] cover.
//! Rows of planned features are in the table as [`Stage::Later`]: their keys are taken (a project or
//! the hotkey can't claim them) but nothing binds them or shows them in a menu yet; so are the
//! keys kept for what they mean in Terminal.app ([`Stage::Reserved`]: ⌘P, ⌘S). Keys of features in
//! the backlog are not here at all.

pub mod menus;
pub mod palette;
pub mod sheet;
mod table;
#[cfg(test)]
mod tests;

use std::fmt;
use std::sync::LazyLock;

use crate::keys::keyspec::{KeyMods, KeySpec};

/// Everything the app can be asked to do by a key or a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionId {
    // Windows and terminals.
    NewWindow,
    NewTerminal,
    /// A terminal in no group, where ⌘T opens one beside the selected terminal, in its group.
    NewTerminalNoGroup,
    /// A terminal in a new git worktree of the current terminal's repository, on a branch the
    /// dialog asks for.
    NewTerminalInWorktree,
    OpenFolder,
    PinTerminal,
    RevealInFinder,
    CopyPath,
    /// The selected terminal; in a split, its pane.
    CloseTerminal,
    CloseWindow,
    NextTerminal,
    PreviousTerminal,
    /// The sidebar's terminal with this number (1–9, as its ⌘ keycap says).
    Terminal(u8),
    /// The previous and the next group, in the tab bar's switcher (and the sidebar's order): ⌃⌥[ and
    /// ⌃⌥]. The terminals in no group count as the last group.
    PreviousGroup,
    NextGroup,
    /// The group with this number (1–9, as the tab bar's switcher lists them).
    Group(u8),
    // Profiles. The three that name a profile take it from a pick (a menu item, a palette row), so
    // they have no key of their own.
    /// The sheet that makes a profile.
    NewProfile,
    /// Settings, on the Profile page.
    ManageProfiles,
    /// Wormhole goes to the picked profile: every window switches.
    SwitchProfile,
    /// A new window in the picked profile, as the odd window when it isn't the shown one.
    OpenWindowInProfile,
    /// The window in front goes to the picked profile.
    MoveWindowToProfile,
    // Panes.
    SplitRight,
    SplitDown,
    ZoomPane,
    /// The focused pane leaves its split and stands alone as a terminal of its own.
    LeaveSplit,
    /// ⌥⌘← → ↑ ↓: the keyboard to the pane beside the focused one.
    FocusPaneLeft,
    FocusPaneRight,
    FocusPaneUp,
    FocusPaneDown,
    /// View → Layout: the current split as 2 columns, 2 rows, a 2×2 grid or a single pane.
    LayoutColumns,
    LayoutRows,
    LayoutGrid,
    LayoutSingle,
    GoToWaiting,
    /// The terminal whose agent or command finished last and that you haven't looked at, then the
    /// one before it on every press: going through what is ready for review.
    GoToFinished,
    Minimize,
    Zoom,
    BringAllToFront,
    // View and panels.
    ToggleSidebar,
    ToggleRightPanel,
    ToggleStatusBar,
    ToggleMinimap,
    /// View → Zen Mode: the window shows only the terminal list and the terminal, for a while and
    /// without saving it (`config::presets::Preset::Zen`, drawn by the app's settings store).
    ToggleZenMode,
    ShowNotes,
    ShowChanges,
    ShowHistory,
    ShowProject,
    ShowUsage,
    ToggleTerminalTabs,
    FocusTerminalOrPanel,
    FocusNextArea,
    FocusPreviousArea,
    Bigger,
    Smaller,
    ActualSize,
    FullScreen,
    CommandPalette,
    KeyboardShortcuts,
    Help,
    // Editing.
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    ClearBuffer,
    // Find in scrollback.
    Find,
    FindNext,
    FindPrevious,
    UseSelectionForFind,
    LineStart,
    LineEnd,
    DeleteLine,
    // Command blocks: jumping between prompts, and what acts on the current block.
    PreviousPrompt,
    NextPrompt,
    SelectOutput,
    CopyOutput,
    CopyCommand,
    /// "Put Command at Prompt": it once ran the command ("Rerun Command"), and the id stays so a
    /// key bound to it keeps working.
    RerunCommand,
    // Notes.
    NewNote,
    StashPrompt,
    MoveItemUp,
    MoveItemDown,
    /// The context menu of the row the keys are on, in a list.
    RowMenu,
    // The app.
    About,
    CheckForUpdates,
    Settings,
    ThemeStudio,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    /// Quit, and end every terminal the session hosts keep running (only with persistent terminals on).
    QuitAndEndAll,
    ShowLogs,
    /// The third-party licences (Help → Licenses).
    Licenses,
    /// The intro window again (Help → Welcome to Wormhole): its welcome page, with the animation.
    ShowIntro,
    /// The wormhole intro, one action per style while they are on trial.
    PlayIntroEmerge,
    PlayIntroVortex,
    PlayIntroCollapse,
    PlayIntroFtl,
    PlayIntroAurora,
    PlayIntroTentacles,
    PlayIntroVoyage,
    /// The terminal welcome again, with each hand-over from the start animation on trial: the ship
    /// docks on the mark's cell, no ship, the ship docks on a block-art logo.
    PlayWelcomeCursor,
    PlayWelcomePortal,
    PlayWelcomeLogo,
    Inspector,
    // Kept free, as Terminal.app uses them.
    Print,
    ExportText,
    // The Theme Studio's window.
    SaveTheme,
    SaveThemeCopy,
}

/// Where a key works. App keys work everywhere in a window, so they collide with every other
/// scope; the terminal answers the editing keys too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Anywhere in a window, the terminal included.
    App,
    /// Text fields (and the terminal: copy, paste, select all).
    Edit,
    /// Only while a terminal has the keyboard.
    Terminal,
    /// Only while a list has the keyboard, or a note or next step being written in one.
    List,
    /// Only in the Theme Studio's window (its fields answer the editing keys).
    Studio,
}

impl Scope {
    /// Whether one key press can be meant for both scopes.
    pub fn overlaps(self, other: Scope) -> bool {
        use Scope::*;
        self == other
            || self == App
            || other == App
            || matches!((self, other), (Edit, Terminal | List | Studio) | (Terminal | List | Studio, Edit))
    }
}

/// Whether an action exists yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Bound and in the menus.
    Now,
    /// Comes later: its key is taken, but nothing binds it yet.
    Later(u8),
    /// macOS does it (⌘? opens the Help menu's search): never bound by the app, but taken.
    System,
    /// Kept for what the key means in Terminal.app (⌘P prints, ⌘S exports the text):
    /// nothing binds it yet, and no project or hotkey may take it meanwhile.
    Reserved,
}

/// One action: what menus call it, its keys, where they work.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: ActionId,
    /// The menu item's title (macOS title case). For a toggle, the title while its thing is on
    /// ("Hide Sidebar").
    pub title: &'static str,
    /// A toggle's title while its thing is off ("Show Sidebar").
    pub title_off: Option<&'static str>,
    /// What the palette, the sheet's search and a conflict message call it, when not the title.
    name: Option<&'static str>,
    /// The first key is the one menus and the sheet show; the others work too.
    pub keys: Vec<KeySpec>,
    pub scope: Scope,
    pub stage: Stage,
    /// Offered by the command palette.
    pub palette: bool,
    /// Only in debug builds (or with the inspector feature).
    pub debug_only: bool,
    /// Only while persistent terminals are on (`FeatureFlag::PersistentTerminals`): the action is
    /// about terminals that outlive the app.
    pub persistent_only: bool,
    /// The feature it belongs to, by its key in `config::FEATURES`: while that feature is off the
    /// action is hidden from the menus, the palette and the sheet (`config::FeatureMask::hidden_actions`).
    /// A key, not a `FeatureId`, because `config` is above this layer.
    pub feature: Option<&'static str>,
}

impl Entry {
    /// What to call the action outside a menu: "Show / Hide Sidebar", "New Terminal".
    pub fn name(&self) -> &'static str {
        self.name.unwrap_or(self.title)
    }

    /// The menu title for a toggle that is `on` (or any other action).
    pub fn title_for(&self, on: bool) -> &'static str {
        match (on, self.title_off) {
            (false, Some(off)) => off,
            _ => self.title,
        }
    }

    /// The key menus and the sheet show.
    pub fn key(&self) -> Option<&KeySpec> {
        self.keys.first()
    }

    /// Whether it is bound and shown now.
    pub fn is_live(&self) -> bool {
        self.stage == Stage::Now
    }

    fn uses(&self, key: &KeySpec) -> bool {
        self.keys.iter().any(|k| k.same_key(key))
    }
}

static ENTRIES: LazyLock<Vec<Entry>> = LazyLock::new(table::entries);

/// Every action, in the table's order.
pub fn entries() -> &'static [Entry] {
    &ENTRIES
}

pub fn entry(id: ActionId) -> Option<&'static Entry> {
    entries().iter().find(|e| e.id == id)
}

/// The key menus and the sheet show for `id`.
pub fn key(id: ActionId) -> Option<&'static KeySpec> {
    entry(id)?.key()
}

/// `id`'s key in Apple's notation (`⌃⌘S`), or "" when it has none.
pub fn glyphs(id: ActionId) -> String {
    key(id).map(KeySpec::glyphs).unwrap_or_default()
}

/// A tooltip that names its key: "Hide sidebar (⌃⌘S)".
pub fn with_key(text: &str, id: ActionId) -> String {
    match key(id) {
        Some(key) => format!("{text} ({})", key.glyphs()),
        None => text.to_string(),
    }
}

/// The action `key` runs where `scope` applies (an app key runs everywhere), live or not.
pub fn action_for(key: &KeySpec, scope: Scope) -> Option<&'static Entry> {
    entries().iter().find(|e| e.scope.overlaps(scope) && e.uses(key))
}

/// Keys macOS keeps for itself: never bound by the app, a project or the hotkey.
const SYSTEM_KEYS: &[(&str, &str)] = &[
    ("⌘Space", "Spotlight"),
    ("⌥⌘Space", "Finder search"),
    ("⌘⇥", "switching apps"),
    ("⇧⌘⇥", "switching apps"),
    ("⌘`", "cycling windows"),
    ("⌃↑", "Mission Control"),
    ("⌃↓", "App Exposé"),
    ("⌃←", "moving between Spaces"),
    ("⌃→", "moving between Spaces"),
    ("⇧⌘3", "screenshots"),
    ("⇧⌘4", "screenshots"),
    ("⇧⌘5", "screenshots"),
    ("⌃⌘Q", "Lock Screen"),
    ("⌃⌘Space", "the Character Viewer"),
    ("⌃⌘D", "Look Up"),
    ("⌃Space", "switching input sources"),
    ("⌃⌥Space", "switching input sources"),
    ("⇧⌘Q", "Log Out"),
    ("⌥⌘⎋", "Force Quit"),
];

static SYSTEM: LazyLock<Vec<(KeySpec, &'static str)>> = LazyLock::new(|| {
    SYSTEM_KEYS.iter().filter_map(|(text, what)| Some((KeySpec::from_glyphs(text)?, *what))).collect()
});

/// Who has a key already, for a conflict message ("⌘T is New Terminal") or a recorder's warning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyUse {
    /// One of the app's actions (live, of a later release, or the system's Help search), and which of
    /// its keys it is (an action may have several).
    Action(&'static Entry, &'static KeySpec),
    /// macOS keeps it (Spotlight, screenshots, Spaces).
    System(&'static str),
    /// The shell and the programs in it need it: typing, control characters, Meta.
    Shell(&'static str),
}

impl fmt::Display for KeyUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action(entry, key) => write!(f, "{} is {}", key.glyphs(), entry.name()),
            Self::System(what) => write!(f, "macOS uses it for {what}"),
            Self::Shell(what) => write!(f, "the shell needs it ({what})"),
        }
    }
}

/// Who already has `key`, if anyone: the app, macOS or the shell. F-keys are free but F6 / ⇧F6.
pub fn uses(key: &KeySpec) -> Option<KeyUse> {
    if let Some(entry) = entries().iter().find(|e| e.uses(key))
        && let Some(own) = entry.keys.iter().find(|k| k.same_key(key))
    {
        return Some(KeyUse::Action(entry, own));
    }
    if let Some((_, what)) = SYSTEM.iter().find(|(k, _)| k.same_key(key)) {
        return Some(KeyUse::System(what));
    }
    let KeyMods { cmd, ctrl, alt, .. } = key.mods;
    if cmd || key.is_function_key() {
        return None;
    }
    Some(KeyUse::Shell(if ctrl {
        "a control character"
    } else if alt {
        "Meta, word jumps and accents"
    } else {
        "typing"
    }))
}

/// Whether the app takes `key` even from a focused terminal: the live app keys that a terminal
/// would otherwise send to its program (F6 / ⇧F6; the ⌘ keys never reach the shell anyway).
pub fn terminal_yields(key: &KeySpec) -> bool {
    entries().iter().any(|e| e.scope == Scope::App && e.is_live() && e.uses(key))
}

/// The action a focused terminal runs itself for `key` (copy, paste, clear, ⌘← …).
pub fn terminal_action(key: &KeySpec) -> Option<ActionId> {
    entries()
        .iter()
        .find(|e| matches!(e.scope, Scope::Terminal | Scope::Edit) && e.is_live() && e.uses(key))
        .map(|e| e.id)
}

/// The action `key` runs in a list or its item editor (⌥⌘↑ / ⌥⌘↓ move the item, ⌃↩ opens the
/// row's menu).
pub fn list_action(key: &KeySpec) -> Option<ActionId> {
    scoped_action(Scope::List, key)
}

/// The action `key` runs in the Theme Studio's window (⌘S saves, ⇧⌘S saves a copy).
pub fn studio_action(key: &KeySpec) -> Option<ActionId> {
    scoped_action(Scope::Studio, key)
}

fn scoped_action(scope: Scope, key: &KeySpec) -> Option<ActionId> {
    entries().iter().find(|e| e.scope == scope && e.is_live() && e.uses(key)).map(|e| e.id)
}
