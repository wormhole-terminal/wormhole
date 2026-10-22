//! The menu bar as data: which actions each menu holds, in what groups, and what the
//! parts that follow the window in front say (the flipping labels, the checked layout, the
//! terminals 1–9, recent folders, the project's actions). The app turns this into native menus;
//! an action that is not live yet (or is the system's) is left out, and so is a separator it
//! would leave dangling.

use super::{ActionId, Stage, entry};
use crate::keys::keyspec::KeySpec;

/// What a menu item does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Action(ActionId),
    /// Open the n-th recent folder (most recent first).
    OpenRecent(usize),
    /// Run the front terminal's project action with this id.
    RunProjectAction(String),
    /// A new terminal running the n-th shell of [`MenuState::shells`].
    NewTerminalWith(usize),
    /// Wormhole goes to the n-th profile of [`MenuState::profiles`].
    SwitchProfile(usize),
    /// A new window in the n-th profile.
    OpenInProfile(usize),
    /// The window in front goes to the n-th profile.
    MoveToProfile(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Action {
        command: Command,
        title: String,
        key: Option<KeySpec>,
        checked: bool,
        enabled: bool,
    },
    Separator,
    /// The system's Services submenu.
    Services,
    Submenu {
        title: String,
        items: Vec<Item>,
        enabled: bool,
    },
    /// A greyed item that says why a submenu has nothing in it ("No Recent Folders"), as
    /// Finder's do: AppKit enables a submenu's parent whatever it holds.
    Placeholder(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub title: &'static str,
    pub items: Vec<Item>,
}

/// What the menus show about the window in front.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuState {
    /// Toggles whose thing is off: their title flips ("Show Sidebar").
    pub off: Vec<ActionId>,
    pub checked: Vec<ActionId>,
    pub disabled: Vec<ActionId>,
    /// Actions whose feature is off: not in the menus, the palette or the shortcut sheet at all.
    pub hidden: Vec<ActionId>,
    /// The sidebar's numbered terminals (⌘1–9) by name, and which one is selected.
    pub terminals: Vec<String>,
    pub current_terminal: Option<usize>,
    /// Recent folders' names, most recent first.
    pub recent: Vec<String>,
    /// The front terminal's project actions: (id, title).
    pub project_actions: Vec<(String, String)>,
    /// The shells New Terminal With offers, by name (`shells::choices`' order).
    pub shells: Vec<String>,
    /// The profiles' names, and which one Wormhole is in (an index into the names).
    pub profiles: Vec<String>,
    pub current_profile: Option<usize>,
    /// The profile of the window in front.
    pub window_profile: Option<usize>,
    /// The selected terminal is in a split: ⌘W closes its pane.
    pub split: bool,
    /// A debug build: the Developer menu is there.
    pub debug: bool,
    /// Persistent terminals are on: the items about terminals that outlive the app are there.
    pub persistent: bool,
}

impl MenuState {
    /// Whether `other` differs from this only in what the terminals are called (an agent's
    /// title changes as it works), which the menus may show a little late.
    pub fn differs_only_in_names(&self, other: &MenuState) -> bool {
        self != other
            && self.terminals.len() == other.terminals.len()
            && self.off == other.off
            && self.checked == other.checked
            && self.disabled == other.disabled
            && self.hidden == other.hidden
            && self.current_terminal == other.current_terminal
            && self.recent == other.recent
            && self.project_actions == other.project_actions
            && self.shells == other.shells
            && self.profiles == other.profiles
            && self.current_profile == other.current_profile
            && self.window_profile == other.window_profile
            && self.split == other.split
            && self.debug == other.debug
            && self.persistent == other.persistent
    }
}

/// A slot in a menu's layout.
enum Slot {
    Action(ActionId),
    Separator,
    Services,
    Submenu(&'static str, &'static [Slot]),
    RecentFolders,
    ProjectActions,
    Terminals,
    /// The shells to open a terminal with (`MenuState::shells`), as Terminal.app's "New Tab with
    /// Profile".
    NewTerminalWith,
    /// The Profile submenu: switch, open a window in, move this window to, new, manage.
    Profiles,
}

use ActionId::*;
use Slot::{Action as A, Separator as Sep};

const APP: &[Slot] = &[
    A(About),
    A(CheckForUpdates),
    Sep,
    A(Settings),
    A(ThemeStudio),
    Sep,
    Slot::Services,
    Sep,
    A(Hide),
    A(HideOthers),
    A(ShowAll),
    Sep,
    A(Quit),
    A(QuitAndEndAll),
];

const SHELL: &[Slot] = &[
    A(NewWindow),
    A(NewTerminal),
    A(NewTerminalNoGroup),
    Slot::NewTerminalWith,
    A(NewTerminalInWorktree),
    Sep,
    A(SplitRight),
    A(SplitDown),
    A(LeaveSplit),
    Sep,
    A(OpenFolder),
    Slot::RecentFolders,
    Sep,
    Slot::Profiles,
    Sep,
    A(CloseTerminal),
    A(CloseWindow),
];

/// Find and the command blocks follow Terminal.app's Edit menu, which has its find and its
/// marks.
const EDIT: &[Slot] = &[
    A(Undo),
    A(Redo),
    Sep,
    A(Cut),
    A(Copy),
    A(Paste),
    A(SelectAll),
    Sep,
    A(ClearBuffer),
    Sep,
    A(Find),
    A(FindNext),
    A(FindPrevious),
    A(UseSelectionForFind),
    Sep,
    A(PreviousPrompt),
    A(NextPrompt),
    A(SelectOutput),
    A(CopyOutput),
    A(CopyCommand),
    A(RerunCommand),
];

const VIEW: &[Slot] = &[
    A(ToggleSidebar),
    A(ToggleRightPanel),
    A(ToggleStatusBar),
    A(ToggleMinimap),
    Sep,
    A(ZoomPane),
    Slot::Submenu("Layout", &[A(LayoutColumns), A(LayoutRows), A(LayoutGrid), A(LayoutSingle)]),
    Sep,
    A(ShowChanges),
    A(ShowHistory),
    A(ShowNotes),
    A(ShowUsage),
    A(ShowProject),
    Sep,
    A(ToggleTerminalTabs),
    Sep,
    A(Bigger),
    A(Smaller),
    A(ActualSize),
    Sep,
    A(CommandPalette),
    Sep,
    A(FullScreen),
];

const TERMINAL: &[Slot] = &[
    A(PinTerminal),
    Sep,
    A(FocusTerminalOrPanel),
    A(FocusNextArea),
    A(FocusPreviousArea),
    A(NewNote),
    A(StashPrompt),
    Slot::ProjectActions,
    Sep,
    A(RevealInFinder),
    A(CopyPath),
];

/// The system adds the list of open windows at the end.
const WINDOW: &[Slot] = &[
    A(Minimize),
    A(Zoom),
    Sep,
    A(NextTerminal),
    A(PreviousTerminal),
    A(NextGroup),
    A(PreviousGroup),
    A(GoToWaiting),
    A(GoToFinished),
    Slot::Submenu("Select Pane", &[A(FocusPaneLeft), A(FocusPaneRight), A(FocusPaneUp), A(FocusPaneDown)]),
    Sep,
    Slot::Terminals,
    Sep,
    A(BringAllToFront),
];

/// "Wormhole Help" is the system's search (⌘?) until there is help to show.
const HELP: &[Slot] = &[A(Help), A(KeyboardShortcuts), A(ShowIntro), Sep, A(Licenses), A(ShowLogs)];

const DEVELOPER: &[Slot] = &[A(Inspector)];

/// The menu bar for `state`, left to right.
pub fn menus(state: &MenuState) -> Vec<Menu> {
    let mut layout =
        vec![("Wormhole", APP), ("Shell", SHELL), ("Edit", EDIT), ("View", VIEW), ("Terminal", TERMINAL)];
    layout.extend([("Window", WINDOW), ("Help", HELP)]);
    if state.debug {
        layout.push(("Developer", DEVELOPER));
    }
    layout.into_iter().map(|(title, slots)| Menu { title, items: tidy(fill(slots, state)) }).collect()
}

fn fill(slots: &[Slot], state: &MenuState) -> Vec<Item> {
    let mut items = Vec::new();
    for slot in slots {
        match slot {
            Slot::Action(id) => items.extend(action_item(*id, state)),
            Slot::Separator => items.push(Item::Separator),
            Slot::Services => items.push(Item::Services),
            Slot::Submenu(title, slots) => {
                let inner = tidy(fill(slots, state));
                if !inner.is_empty() {
                    items.push(Item::Submenu { title: title.to_string(), items: inner, enabled: true });
                }
            }
            Slot::RecentFolders => {
                let recent = state.recent.iter().enumerate().map(|(ix, name)| Item::Action {
                    command: Command::OpenRecent(ix),
                    title: name.clone(),
                    key: None,
                    checked: false,
                    enabled: true,
                });
                items.push(submenu("Open Recent", recent.collect(), "No Recent Folders"));
            }
            Slot::ProjectActions => {
                let actions = state.project_actions.iter().map(|(id, title)| Item::Action {
                    command: Command::RunProjectAction(id.clone()),
                    title: title.clone(),
                    key: None,
                    checked: false,
                    enabled: true,
                });
                items.push(submenu("Project Actions", actions.collect(), "No Project Actions"));
            }
            Slot::NewTerminalWith => {
                let shells = state.shells.iter().enumerate().map(|(ix, name)| Item::Action {
                    command: Command::NewTerminalWith(ix),
                    title: name.clone(),
                    key: None,
                    checked: false,
                    enabled: true,
                });
                items.push(submenu("New Terminal With", shells.collect(), "No Shells"));
            }
            Slot::Profiles => items.push(profile_submenu(state)),
            Slot::Terminals => {
                for (ix, name) in state.terminals.iter().take(9).enumerate() {
                    let id = Terminal(ix as u8 + 1);
                    items.push(Item::Action {
                        command: Command::Action(id),
                        title: name.clone(),
                        key: super::key(id).cloned(),
                        checked: state.current_terminal == Some(ix),
                        enabled: true,
                    });
                }
            }
        }
    }
    items
}

/// Profile ▸ the profiles (the shown one checked), what opens a window in one or moves this window
/// to one (only with another profile to pick), then New Profile… and Manage Profiles….
fn profile_submenu(state: &MenuState) -> Item {
    let pick = |command: fn(usize) -> Command, skip: Option<usize>| -> Vec<Item> {
        state
            .profiles
            .iter()
            .enumerate()
            .filter(|(ix, _)| Some(*ix) != skip)
            .map(|(ix, name)| Item::Action {
                command: command(ix),
                title: name.clone(),
                key: None,
                checked: false,
                enabled: true,
            })
            .collect()
    };
    let mut items: Vec<Item> = state
        .profiles
        .iter()
        .enumerate()
        .map(|(ix, name)| Item::Action {
            command: Command::SwitchProfile(ix),
            title: name.clone(),
            key: None,
            checked: state.current_profile == Some(ix),
            enabled: true,
        })
        .collect();
    if state.profiles.len() > 1 {
        items.push(Item::Separator);
        items.push(Item::Submenu {
            title: "Open Window in".to_string(),
            items: pick(Command::OpenInProfile, None),
            enabled: true,
        });
        items.push(Item::Submenu {
            title: "Move Window to".to_string(),
            items: pick(Command::MoveToProfile, state.window_profile),
            enabled: state.window_profile.is_some(),
        });
    }
    items.push(Item::Separator);
    items.extend(action_item(NewProfile, state));
    items.extend(action_item(ManageProfiles, state));
    Item::Submenu { title: "Profile".to_string(), items: tidy(items), enabled: true }
}

/// A submenu of what the window has; with nothing, the greyed `empty` in it.
fn submenu(title: &str, mut items: Vec<Item>, empty: &'static str) -> Item {
    if items.is_empty() {
        items.push(Item::Placeholder(empty));
    }
    Item::Submenu { title: title.to_string(), enabled: true, items }
}

fn action_item(id: ActionId, state: &MenuState) -> Option<Item> {
    let entry = entry(id)?;
    if entry.stage != Stage::Now
        || (entry.debug_only && !state.debug)
        || (entry.persistent_only && !state.persistent)
        || state.hidden.contains(&id)
    {
        return None;
    }
    // In a split ⌘W closes the focused pane (the pane menu says so), so its item does too.
    let title = if id == CloseTerminal && state.split {
        "Close Pane"
    } else {
        entry.title_for(!state.off.contains(&id))
    };
    Some(Item::Action {
        command: Command::Action(id),
        title: title.to_string(),
        key: entry.key().cloned(),
        checked: state.checked.contains(&id),
        enabled: !state.disabled.contains(&id),
    })
}

/// No separator at either end of a menu or next to another one.
fn tidy(items: Vec<Item>) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::with_capacity(items.len());
    for item in items {
        let separator = item == Item::Separator;
        if separator && out.last().is_none_or(|last| *last == Item::Separator) {
            continue;
        }
        out.push(item);
    }
    if out.last() == Some(&Item::Separator) {
        out.pop();
    }
    out
}

/// The menus as text, one item per line with its key and marks, for tests and `devctl menus`.
pub fn dump(menus: &[Menu]) -> String {
    fn write_items(out: &mut String, items: &[Item], depth: usize) {
        let indent = "  ".repeat(depth);
        for item in items {
            match item {
                Item::Separator => out.push_str(&format!("{indent}---\n")),
                Item::Services => out.push_str(&format!("{indent}Services ▸\n")),
                Item::Placeholder(title) => out.push_str(&format!("{indent}{title}  (disabled)\n")),
                Item::Submenu { title, items: inner, enabled } => {
                    let disabled = if *enabled { "" } else { "  (disabled)" };
                    out.push_str(&format!("{indent}{title} ▸{disabled}\n"));
                    write_items(out, inner, depth + 1);
                }
                Item::Action { title, key, checked, enabled, .. } => {
                    let check = if *checked { "✓ " } else { "" };
                    let key = key.as_ref().map(|k| format!("  {}", k.glyphs())).unwrap_or_default();
                    let disabled = if *enabled { "" } else { "  (disabled)" };
                    out.push_str(&format!("{indent}{check}{title}{key}{disabled}\n"));
                }
            }
        }
    }
    let mut out = String::new();
    for menu in menus {
        out.push_str(&format!("{}\n", menu.title));
        write_items(&mut out, &menu.items, 1);
    }
    out
}
