//! Pure snapshot → menu mapping, with the Dock menu. No AppKit here, so the layout is
//! unit-testable; `appkit.rs` only renders what this produces.
// Only the macOS renderer consumes the model; it is portable and unit-tested everywhere.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use std::time::Instant;

use wormhole_core::config::MenuBarIcon;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::notify::{self, Counts, ItemState, State, Terminal};

use super::{ActionItem, MenuCommand, MenuSnapshot};

/// Terminals listed in the sections before the rest fold into "N more…", so the menu never
/// outgrows the screen.
pub const MAX_SESSIONS: usize = 20;

/// The glyph in front of a terminal row: a palette SF Symbol in the theme's status colour, the
/// shapes of the sidebar's lead column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// An agent or a program that reports needs you: an amber dot.
    NeedsYou,
    /// A terminal that nothing tells the state of rang its bell: an amber bell.
    Bell,
    /// An agent, or a program that reports, works.
    Working,
    /// A command runs in a terminal that nothing tells the state of: the same, neutral.
    Running,
    /// Finished: a check.
    Done,
    /// Failed (a command's exit code, or an agent or program that stopped on an error): a crossed
    /// circle in the danger colour.
    Failed,
    /// Idle: a ring.
    Idle,
    Exited,
}

impl Glyph {
    fn of(terminal: &Terminal) -> Self {
        match (terminal.state, terminal.source.told()) {
            (State::NeedsYou, true) => Glyph::NeedsYou,
            (State::NeedsYou, false) => Glyph::Bell,
            (State::Working, true) => Glyph::Working,
            (State::Working, false) => Glyph::Running,
            (State::Done, _) => Glyph::Done,
            (State::Failed, _) => Glyph::Failed,
            (State::Idle, _) => Glyph::Idle,
            (State::Exited, _) => Glyph::Exited,
        }
    }
}

/// Key equivalent shown right-aligned in the menu (only active while the menu is open).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyEquivalent {
    /// Lowercase character AppKit expects (`"t"`, `" "`).
    pub key: String,
    pub command: bool,
    pub option: bool,
    pub control: bool,
    pub shift: bool,
}

impl KeyEquivalent {
    /// The menu's form of a key spec; `None` for keys a menu item can't show as one character
    /// (F-keys, arrows, Enter).
    pub fn from_spec(spec: &KeySpec) -> Option<Self> {
        let key = match spec.key.as_str() {
            "space" => " ".to_string(),
            key if key.chars().count() == 1 => key.to_string(),
            _ => return None,
        };
        let m = spec.mods;
        Some(Self { key, command: m.cmd, option: m.alt, control: m.ctrl, shift: m.shift })
    }

    fn cmd(key: &str) -> Self {
        Self { key: key.into(), command: true, ..Self::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemModel {
    pub title: String,
    /// Right-aligned on the title's line: how long a terminal has waited or run ("2m", "4:12",
    /// "12m ago"), or a submenu's count.
    pub trailing: Option<String>,
    /// Second line under the title (macOS 14.4+); also used as tooltip.
    pub subtitle: Option<String>,
    pub glyph: Option<Glyph>,
    pub key: Option<KeyEquivalent>,
    pub indent: isize,
    /// `None` renders a disabled row.
    pub command: Option<MenuCommand>,
}

impl ItemModel {
    fn new(title: impl Into<String>, command: Option<MenuCommand>) -> Self {
        Self {
            title: title.into(),
            trailing: None,
            subtitle: None,
            glyph: None,
            key: None,
            indent: 0,
            command,
        }
    }

    fn key(self, key: Option<KeyEquivalent>) -> Self {
        Self { key, ..self }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A native section header (`NSMenuItem.sectionHeader`).
    Header(String),
    Separator,
    Item(ItemModel),
    Submenu {
        title: String,
        trailing: Option<String>,
        entries: Vec<Entry>,
    },
}

/// What the dot and the count on the status item say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    None,
    /// The `agent.attention` dot.
    NeedsYou,
    /// The `agent.done` check.
    Finished,
    /// The `status.danger` dot.
    Failed,
}

/// What the status bar button shows. The glyph is a template (it follows the bar's light and
/// dark), filled while something works; only the mark and the count carry colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusIcon {
    pub state: ItemState,
    pub mark: Mark,
    /// How many need you ("9+" past nine), beside the glyph.
    pub count: Option<String>,
    pub tooltip: String,
    /// The working glyph types its prompt out (`menu_bar_animate`; only while working is the
    /// state).
    pub animate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuModel {
    pub icon: StatusIcon,
    pub entries: Vec<Entry>,
}

/// The status item for `terminals`, as `mode` (`menu_bar_icon`) wants it shown. Most urgent
/// wins: needs you › failed › finished › working › idle.
pub fn status_icon(terminals: &[Terminal], mode: MenuBarIcon, animate: bool) -> StatusIcon {
    let counts = Counts::of(terminals);
    let state = notify::item_state(counts);
    let mark = match (mode, state) {
        (MenuBarIcon::Plain, _) => Mark::None,
        (_, ItemState::NeedsYou) => Mark::NeedsYou,
        (_, ItemState::Failed) => Mark::Failed,
        (_, ItemState::Finished) => Mark::Finished,
        _ => Mark::None,
    };
    let count = match mode {
        MenuBarIcon::StatusCount => notify::badge(counts.needs_you),
        MenuBarIcon::Status | MenuBarIcon::Plain => None,
    };
    StatusIcon {
        state,
        mark,
        count,
        tooltip: notify::tooltip(counts),
        animate: animate && state == ItemState::Working,
    }
}

fn terminal_row(terminal: &Terminal, now: Instant) -> Entry {
    let title = if terminal.name.trim().is_empty() { "Untitled".to_string() } else { terminal.name.clone() };
    Entry::Item(ItemModel {
        trailing: notify::elapsed(terminal.state, now.saturating_duration_since(terminal.since)),
        subtitle: terminal.detail(true),
        glyph: Some(Glyph::of(terminal)),
        ..ItemModel::new(title, Some(MenuCommand::OpenTab(terminal.tab)))
    })
}

/// "Actions · api-server" and its actions, per project, in first-appearance order. Two projects
/// with the same folder name are told apart by their path.
fn actions_entries(actions: &[ActionItem]) -> Vec<Entry> {
    let mut projects: Vec<(&ActionItem, Vec<&ActionItem>)> = Vec::new();
    for action in actions {
        match projects.iter_mut().find(|(first, _)| first.root == action.root) {
            Some((_, list)) => list.push(action),
            None => projects.push((action, vec![action])),
        }
    }
    let mut entries = Vec::new();
    for (first, list) in &projects {
        let name_repeats = projects.iter().filter(|(other, _)| other.project == first.project).count() > 1;
        entries.push(Entry::Header(match name_repeats {
            true => format!("Actions · {} — {}", first.project, first.root.display()),
            false => format!("Actions · {}", first.project),
        }));
        for action in list {
            let command = MenuCommand::RunAction { root: action.root.clone(), id: action.id.clone() };
            entries.push(Entry::Item(ItemModel::new(action.title.clone(), Some(command))));
        }
    }
    entries
}

/// The Projects submenu: recent folders, each open project's actions, then Open Folder….
fn projects_entries(snapshot: &MenuSnapshot) -> Vec<Entry> {
    let mut entries = Vec::new();
    if !snapshot.recent_projects.is_empty() {
        entries.push(Entry::Header("Recent".into()));
        for (name, path) in &snapshot.recent_projects {
            let mut item = ItemModel::new(name.clone(), Some(MenuCommand::OpenProject(path.clone())));
            item.subtitle = Some(display_path(path));
            entries.push(Entry::Item(item));
        }
    }
    entries.extend(actions_entries(&snapshot.actions));
    entries.push(Entry::Separator);
    entries.push(Entry::Item(open_folder()));
    entries
}

fn open_folder() -> ItemModel {
    ItemModel::new("Open Folder…", Some(MenuCommand::OpenFolder)).key(Some(KeyEquivalent::cmd("o")))
}

/// A path with the home folder as `~`.
fn display_path(path: &std::path::Path) -> String {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    wormhole_core::view::sidebar::display_path(path, home.as_deref())
}

/// The menu, with each row's time as of `now` (the menu is built as it opens).
pub fn build(snapshot: &MenuSnapshot, now: Instant) -> MenuModel {
    let terminals = &snapshot.terminals;
    let sections = notify::sections(terminals);
    let mut entries = Vec::new();
    let mut listed = 0;
    let mut hidden = 0;
    for (title, list) in
        [("Needs you", &sections.needs_you), ("Working", &sections.working), ("Finished", &sections.finished)]
    {
        if list.is_empty() {
            continue;
        }
        entries.push(Entry::Header(title.into()));
        for &ix in list {
            if listed < MAX_SESSIONS {
                entries.push(terminal_row(&terminals[ix], now));
                listed += 1;
            } else {
                hidden += 1;
            }
        }
    }
    if hidden > 0 {
        entries.push(Entry::Item(ItemModel::new(format!("{hidden} more…"), Some(MenuCommand::ShowApp))));
    }
    if terminals.is_empty() {
        entries.push(Entry::Item(ItemModel::new("No terminals open", None)));
    }
    if !sections.other.is_empty() {
        if !entries.is_empty() {
            entries.push(Entry::Separator);
        }
        let mut others: Vec<Entry> =
            sections.other.iter().take(MAX_SESSIONS).map(|&ix| terminal_row(&terminals[ix], now)).collect();
        if let Some(more) = sections.other.len().checked_sub(MAX_SESSIONS).filter(|&n| n > 0) {
            others.push(Entry::Item(ItemModel::new(format!("{more} more…"), Some(MenuCommand::ShowApp))));
        }
        entries.push(Entry::Submenu {
            title: "Other Terminals".into(),
            trailing: Some(sections.other.len().to_string()),
            entries: others,
        });
    }

    entries.push(Entry::Separator);
    entries.push(Entry::Item(
        ItemModel::new("New Terminal", Some(MenuCommand::NewSession)).key(Some(KeyEquivalent::cmd("t"))),
    ));
    entries.push(Entry::Item(open_folder()));
    if !snapshot.recent_projects.is_empty() || !snapshot.actions.is_empty() {
        entries.push(Entry::Submenu {
            title: "Projects".into(),
            trailing: None,
            entries: projects_entries(snapshot),
        });
    }

    entries.push(Entry::Separator);
    entries.push(Entry::Item(
        ItemModel::new("Show Wormhole", Some(MenuCommand::ShowApp)).key(snapshot.hotkey.clone()),
    ));
    let waiting = !sections.needs_you.is_empty();
    entries.push(Entry::Item(
        ItemModel::new("Jump to Next Waiting", waiting.then_some(MenuCommand::JumpToWaiting))
            .key(snapshot.jump_key.clone()),
    ));
    entries.push(Entry::Item(
        ItemModel::new("Settings…", Some(MenuCommand::OpenSettings)).key(Some(KeyEquivalent::cmd(","))),
    ));
    entries.push(Entry::Item(ItemModel::new("Check for Updates…", Some(MenuCommand::CheckForUpdates))));
    entries.push(Entry::Separator);
    entries.push(Entry::Item(
        ItemModel::new("Quit Wormhole", Some(MenuCommand::Quit)).key(Some(KeyEquivalent::cmd("q"))),
    ));

    MenuModel { icon: status_icon(terminals, snapshot.icon, snapshot.animate), entries }
}

/// One row of the Dock's menu, which GPUI renders: waiting and running terminals under
/// their headers, then New Terminal and New Window (macOS adds its own items below).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockEntry {
    Header(&'static str),
    Terminal { label: String, tab: i64 },
    Separator,
    NewTerminal,
    NewWindow,
}

/// The Dock menu for `terminals`. Times are to the minute ("2m", "4m"): GPUI keeps every menu
/// action it was ever given, so the menu is only rebuilt when its words change.
pub fn dock_menu(terminals: &[Terminal], now: Instant) -> Vec<DockEntry> {
    let sections = notify::sections(terminals);
    let mut entries = Vec::new();
    for (title, list) in [("Needs you", &sections.needs_you), ("Working", &sections.working)] {
        if list.is_empty() {
            continue;
        }
        entries.push(DockEntry::Header(title));
        for &ix in list.iter().take(MAX_SESSIONS / 2) {
            let terminal = &terminals[ix];
            let waited = now.saturating_duration_since(terminal.since);
            let time = wormhole_core::view::sidebar::wording::took(waited).unwrap_or_else(|| "now".into());
            entries.push(DockEntry::Terminal {
                label: format!("{}   {time}", terminal.name),
                tab: terminal.tab,
            });
        }
    }
    if !entries.is_empty() {
        entries.push(DockEntry::Separator);
    }
    entries.push(DockEntry::NewTerminal);
    entries.push(DockEntry::NewWindow);
    entries
}

/// The menu as text, one entry per line (devctl `menubar`, compared with the status item).
#[cfg(any(test, feature = "devctl"))]
pub fn dump(model: &MenuModel) -> String {
    let icon = &model.icon;
    let mut out = format!(
        "icon state={:?} mark={:?} count={:?} animate={} tooltip={:?}\n",
        icon.state, icon.mark, icon.count, icon.animate, icon.tooltip
    );
    dump_entries(&model.entries, 0, &mut out);
    out
}

#[cfg(any(test, feature = "devctl"))]
fn dump_entries(entries: &[Entry], depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    for entry in entries {
        match entry {
            Entry::Header(title) => out.push_str(&format!("{pad}== {title}\n")),
            Entry::Separator => out.push_str(&format!("{pad}----\n")),
            Entry::Item(item) => {
                let glyph = item.glyph.map(|g| format!("[{g:?}] ")).unwrap_or_default();
                let trailing = item.trailing.as_ref().map(|t| format!("  ⇥ {t}")).unwrap_or_default();
                let key = item.key.as_ref().map(|k| format!("  ⌨ {}", key_text(k))).unwrap_or_default();
                let disabled = if item.command.is_none() { "  (disabled)" } else { "" };
                out.push_str(&format!("{pad}{glyph}{}{trailing}{key}{disabled}\n", item.title));
                if let Some(subtitle) = &item.subtitle {
                    out.push_str(&format!("{pad}    {subtitle}\n"));
                }
            }
            Entry::Submenu { title, trailing, entries } => {
                let trailing = trailing.as_ref().map(|t| format!("  ⇥ {t}")).unwrap_or_default();
                out.push_str(&format!("{pad}{title}{trailing}  ▸\n"));
                dump_entries(entries, depth + 1, out);
            }
        }
    }
}

#[cfg(any(test, feature = "devctl"))]
fn key_text(key: &KeyEquivalent) -> String {
    let mut out = String::new();
    for (on, glyph) in [(key.control, '⌃'), (key.option, '⌥'), (key.shift, '⇧'), (key.command, '⌘')] {
        if on {
            out.push(glyph);
        }
    }
    out.push_str(&if key.key == " " { "Space".to_string() } else { key.key.to_uppercase() });
    out
}

#[cfg(test)]
mod tests;
