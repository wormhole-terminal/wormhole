//! The menu bar for a window's state, as core builds it (`keymap::menus`): the layout, the flipping titles,
//! what is hidden or disabled, the numbered terminals. Swift hands the window's facts in and draws what
//! comes back.

use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keymap::menus::{self, Command, Item, Menu, MenuState};

use super::records::{KeySpecRecord, id_from_string, id_string};

/// What choosing an item does.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum MenuCommandRecord {
    Action { id: String },
    OpenRecent { index: u32 },
    RunProjectAction { id: String },
    NewTerminalWith { index: u32 },
    SwitchProfile { index: u32 },
    OpenInProfile { index: u32 },
    MoveToProfile { index: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MenuItemKindRecord {
    Action,
    Separator,
    /// The system's Services submenu.
    Services,
    Submenu,
    /// A greyed line saying why a submenu is empty ("No Recent Folders"); its words are the title.
    Placeholder,
}

/// One entry of a menu. A flat record (a submenu holds `items`) so Swift keeps its own value types.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MenuItemRecord {
    pub kind: MenuItemKindRecord,
    /// `Action` only.
    pub command: Option<MenuCommandRecord>,
    pub title: String,
    pub key: Option<KeySpecRecord>,
    pub checked: bool,
    pub enabled: bool,
    /// `Submenu` only.
    pub items: Vec<MenuItemRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MenuRecord {
    pub title: String,
    pub items: Vec<MenuItemRecord>,
}

/// What the menus show about the window in front (`MenuState`), actions by id text.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct MenuStateRecord {
    pub off: Vec<String>,
    pub checked: Vec<String>,
    pub disabled: Vec<String>,
    pub hidden: Vec<String>,
    pub terminals: Vec<String>,
    pub current_terminal: Option<u32>,
    pub recent: Vec<String>,
    /// The front terminal's project actions as (id, title).
    pub project_actions: Vec<ProjectActionRecord>,
    pub shells: Vec<String>,
    pub profiles: Vec<String>,
    pub current_profile: Option<u32>,
    pub window_profile: Option<u32>,
    pub split: bool,
    pub undo: Option<String>,
    pub redo: Option<String>,
    pub debug: bool,
    pub persistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProjectActionRecord {
    pub id: String,
    pub title: String,
}

impl MenuStateRecord {
    /// An id text the table does not know is dropped: a stale Swift side names no action that exists.
    fn ids(texts: &[String]) -> Vec<ActionId> {
        texts.iter().filter_map(|text| id_from_string(text)).collect()
    }

    pub(super) fn to_core(&self) -> MenuState {
        let index = |i: Option<u32>| i.map(|i| i as usize);
        MenuState {
            off: Self::ids(&self.off),
            checked: Self::ids(&self.checked),
            disabled: Self::ids(&self.disabled),
            hidden: Self::ids(&self.hidden),
            terminals: self.terminals.clone(),
            current_terminal: index(self.current_terminal),
            recent: self.recent.clone(),
            project_actions: self.project_actions.iter().map(|a| (a.id.clone(), a.title.clone())).collect(),
            shells: self.shells.clone(),
            profiles: self.profiles.clone(),
            current_profile: index(self.current_profile),
            window_profile: index(self.window_profile),
            split: self.split,
            undo: self.undo.clone(),
            redo: self.redo.clone(),
            debug: self.debug,
            persistent: self.persistent,
        }
    }
}

fn command(command: &Command) -> MenuCommandRecord {
    match command {
        Command::Action(id) => MenuCommandRecord::Action { id: id_string(*id) },
        Command::OpenRecent(i) => MenuCommandRecord::OpenRecent { index: *i as u32 },
        Command::RunProjectAction(id) => MenuCommandRecord::RunProjectAction { id: id.clone() },
        Command::NewTerminalWith(i) => MenuCommandRecord::NewTerminalWith { index: *i as u32 },
        Command::SwitchProfile(i) => MenuCommandRecord::SwitchProfile { index: *i as u32 },
        Command::OpenInProfile(i) => MenuCommandRecord::OpenInProfile { index: *i as u32 },
        Command::MoveToProfile(i) => MenuCommandRecord::MoveToProfile { index: *i as u32 },
    }
}

fn plain(kind: MenuItemKindRecord, title: &str) -> MenuItemRecord {
    MenuItemRecord {
        kind,
        command: None,
        title: title.to_string(),
        key: None,
        checked: false,
        enabled: true,
        items: Vec::new(),
    }
}

fn item_record(item: &Item) -> MenuItemRecord {
    match item {
        Item::Action { command: c, title, key, checked, enabled } => MenuItemRecord {
            command: Some(command(c)),
            key: key.as_ref().map(KeySpecRecord::from),
            checked: *checked,
            enabled: *enabled,
            ..plain(MenuItemKindRecord::Action, title)
        },
        Item::Separator => plain(MenuItemKindRecord::Separator, ""),
        Item::Services => plain(MenuItemKindRecord::Services, ""),
        Item::Submenu { title, items, enabled } => MenuItemRecord {
            enabled: *enabled,
            items: items.iter().map(item_record).collect(),
            ..plain(MenuItemKindRecord::Submenu, title)
        },
        Item::Placeholder(words) => {
            MenuItemRecord { enabled: false, ..plain(MenuItemKindRecord::Placeholder, words) }
        }
    }
}

fn menu(menu: &Menu) -> MenuRecord {
    MenuRecord { title: menu.title.to_string(), items: menu.items.iter().map(item_record).collect() }
}

/// The menu bar for `state`, left to right.
#[uniffi::export]
pub fn keymap_menus(state: MenuStateRecord) -> Vec<MenuRecord> {
    menus::menus(&state.to_core()).iter().map(menu).collect()
}

/// The same menus as core's own text dump (`menus::dump`), so a front end can be compared with it line by line.
#[uniffi::export]
pub fn keymap_menus_dump(state: MenuStateRecord) -> String {
    menus::dump(&menus::menus(&state.to_core()))
}
