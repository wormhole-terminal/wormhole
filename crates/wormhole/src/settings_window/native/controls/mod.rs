//! What each setting the native window writes is, as data: a switch over a `bool`, a menu over a few options, a
//! shortcut recorder. A page's table is in its own file (`terminal`, `appearance`, ...); the codec reads and writes
//! the settings through them, so a setting's options and what choosing one does are written once.
//!
//! Words come as messages of the catalogs, looked up when asked: the tables are `const` and cannot read the
//! language in use.

use std::collections::HashMap;
use std::sync::OnceLock;

use gpui_kit::App;
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_text::{Key, text};

use crate::settings::Settings;

mod agents;
mod appearance;
mod fonts;
mod git;
mod menu_bar;
mod notifications;
mod sessions;
mod terminal;
mod updates;

/// The words of one option of a menu.
pub(super) enum Label {
    /// A message of the catalogs.
    Message(Key),
    /// A message with a whole number in it ("4 spaces"): how to say the number, and the number.
    Counted(fn(u32) -> String, u32),
}

impl Label {
    pub(super) fn words(&self) -> String {
        match self {
            Self::Message(key) => text(*key).to_string(),
            Self::Counted(say, n) => say(*n),
        }
    }
}

/// One option of a menu.
pub(super) struct MenuOption {
    pub label: Label,
}

impl MenuOption {
    pub(super) const fn text(label: Key) -> Self {
        Self { label: Label::Message(label) }
    }

    pub(super) const fn counted(say: fn(u32) -> String, n: u32) -> Self {
        Self { label: Label::Counted(say, n) }
    }
}

/// What a menu's setting is now, read from the settings.
pub(super) enum Current {
    /// The option at this place.
    At(usize),
    /// None of the options: a value written in `config.json` by hand. Shown as it is, and kept until another
    /// option is chosen.
    Other(String),
}

/// How a menu reads its setting.
#[derive(Clone, Copy)]
pub(super) enum Read {
    /// Every value of the setting is an option.
    Index(fn(&Settings) -> usize),
    /// The setting is a number or a mix of keys, so a value may be none of the options.
    Value(fn(&Settings) -> Current),
}

pub(super) struct Menu {
    pub options: &'static [MenuOption],
    pub read: Read,
    /// Chooses the option at a place; never called with a place past the options.
    pub set: fn(&mut Settings, usize),
}

impl Menu {
    pub(super) fn current(&self, settings: &Settings) -> Current {
        match self.read {
            Read::Index(get) => Current::At(get(settings).min(self.options.len().saturating_sub(1))),
            Read::Value(get) => get(settings),
        }
    }
}

/// What a shortcut row records into and checks.
#[derive(Clone, Copy)]
pub(super) struct RecorderRow {
    pub get: fn(&Settings) -> &str,
    pub set: fn(&mut Settings, String),
    /// Whether the row can use these keys (a modifier, a key the system can register).
    pub accepts: fn(&KeySpec) -> bool,
    /// What is wrong with the shortcut set now (macOS has it, it didn't register).
    pub problem: fn(&Settings, &App) -> Option<String>,
    /// The action whose key this is (its own key is no conflict).
    pub own: Option<ActionId>,
}

pub(super) enum Control {
    /// A switch.
    Toggle { get: fn(&Settings) -> bool, set: fn(&mut Settings, bool) },
    /// A few options in a menu that shows the current one.
    Menu(Menu),
    /// A shortcut, recorded by pressing it (empty is none).
    Recorder(RecorderRow),
}

pub(super) struct Setting {
    /// The settings index's id for the row.
    pub id: &'static str,
    pub control: Control,
}

impl Setting {
    pub(super) const fn toggle(
        id: &'static str,
        get: fn(&Settings) -> bool,
        set: fn(&mut Settings, bool),
    ) -> Self {
        Self { id, control: Control::Toggle { get, set } }
    }

    /// A menu over a setting of which every value is one of the options.
    pub(super) const fn menu(
        id: &'static str,
        options: &'static [MenuOption],
        get: fn(&Settings) -> usize,
        set: fn(&mut Settings, usize),
    ) -> Self {
        Self { id, control: Control::Menu(Menu { options, read: Read::Index(get), set }) }
    }

    /// A menu over a setting that may hold a value none of the options is.
    pub(super) const fn menu_of_values(
        id: &'static str,
        options: &'static [MenuOption],
        get: fn(&Settings) -> Current,
        set: fn(&mut Settings, usize),
    ) -> Self {
        Self { id, control: Control::Menu(Menu { options, read: Read::Value(get), set }) }
    }

    pub(super) const fn recorder(id: &'static str, row: RecorderRow) -> Self {
        Self { id, control: Control::Recorder(row) }
    }
}

/// A switch for a `bool` field of the settings.
macro_rules! toggle {
    ($id:literal, $field:ident) => {
        Setting::toggle($id, |s| s.$field, |s, on| s.$field = on)
    };
}
use toggle;

/// The row whose recorder sets the global hotkey.
pub(super) const HOTKEY_ROW: &str = "hotkey";

/// Every page's controls.
const PAGES: [&[Setting]; 9] = [
    terminal::SETTINGS,
    appearance::SETTINGS,
    fonts::SETTINGS,
    sessions::SETTINGS,
    git::SETTINGS,
    agents::SETTINGS,
    notifications::SETTINGS,
    menu_bar::SETTINGS,
    updates::SETTINGS,
];

/// The control of the row `id`.
pub(super) fn control(id: &str) -> Option<&'static Setting> {
    static CONTROLS: OnceLock<HashMap<&'static str, &'static Setting>> = OnceLock::new();
    CONTROLS
        .get_or_init(|| PAGES.into_iter().flatten().map(|setting| (setting.id, setting)).collect())
        .get(id)
        .copied()
}

/// What the recorder of row `id` records into and checks.
pub(super) fn recorder_row(id: &str) -> Option<RecorderRow> {
    match control(id)?.control {
        Control::Recorder(row) => Some(row),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_is_in_one_page_only() {
        let mut ids: Vec<_> = PAGES.into_iter().flatten().map(|s| s.id).collect();
        let all = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all);
    }

    #[test]
    fn every_menu_has_options_and_never_reads_past_them() {
        for setting in PAGES.into_iter().flatten() {
            if let Control::Menu(menu) = &setting.control {
                assert!(!menu.options.is_empty(), "{}", setting.id);
                if let Current::At(at) = menu.current(&Settings::default()) {
                    assert!(at < menu.options.len(), "{}", setting.id);
                }
            }
        }
    }
}
