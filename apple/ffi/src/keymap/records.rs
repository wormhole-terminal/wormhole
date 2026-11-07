//! The keymap's values as Swift reads them: plain records, the action named by its string id
//! (`newTerminal`, `terminal.3`), keys as a modifier mask and the lowercase key.

use wormhole_core::keys::keymap::{self, ActionId, Entry, Scope, Stage};
use wormhole_core::keys::keyspec::{KeyMods, KeySpec};

/// A key and the modifiers held with it. `mods` is a mask: control 1, option 2, shift 4, command 8 (the
/// order of Apple's glyphs, and `KeyMods`' raw values on the Swift side).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct KeySpecRecord {
    pub mods: u8,
    pub key: String,
}

impl From<&KeySpec> for KeySpecRecord {
    fn from(spec: &KeySpec) -> Self {
        let KeyMods { cmd, ctrl, alt, shift } = spec.mods;
        let mods = u8::from(ctrl) | u8::from(alt) << 1 | u8::from(shift) << 2 | u8::from(cmd) << 3;
        Self { mods, key: spec.key.clone() }
    }
}

impl From<&KeySpecRecord> for KeySpec {
    fn from(record: &KeySpecRecord) -> Self {
        let mods = KeyMods {
            ctrl: record.mods & 1 != 0,
            alt: record.mods & 2 != 0,
            shift: record.mods & 4 != 0,
            cmd: record.mods & 8 != 0,
        };
        KeySpec::new(mods, &record.key)
    }
}

/// Where a key works (`keymap::Scope`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum KeyScopeRecord {
    App,
    Edit,
    Terminal,
    List,
    Studio,
}

impl From<Scope> for KeyScopeRecord {
    fn from(scope: Scope) -> Self {
        match scope {
            Scope::App => Self::App,
            Scope::Edit => Self::Edit,
            Scope::Terminal => Self::Terminal,
            Scope::List => Self::List,
            Scope::Studio => Self::Studio,
        }
    }
}

impl From<KeyScopeRecord> for Scope {
    fn from(scope: KeyScopeRecord) -> Self {
        match scope {
            KeyScopeRecord::App => Self::App,
            KeyScopeRecord::Edit => Self::Edit,
            KeyScopeRecord::Terminal => Self::Terminal,
            KeyScopeRecord::List => Self::List,
            KeyScopeRecord::Studio => Self::Studio,
        }
    }
}

/// Whether an action exists yet (`keymap::Stage`; the release a `Later` one comes in is not needed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum KeyStageRecord {
    Now,
    Later,
    System,
    Reserved,
}

impl From<Stage> for KeyStageRecord {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::Now => Self::Now,
            Stage::Later(_) => Self::Later,
            Stage::System => Self::System,
            Stage::Reserved => Self::Reserved,
        }
    }
}

/// One action of the table (`keymap::Entry`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct KeymapEntryRecord {
    pub id: String,
    pub title: String,
    pub title_off: Option<String>,
    /// What the palette and a conflict message call it, when not the title.
    pub name: Option<String>,
    pub keys: Vec<KeySpecRecord>,
    pub scope: KeyScopeRecord,
    pub stage: KeyStageRecord,
    pub palette: bool,
    pub debug_only: bool,
    pub persistent_only: bool,
    pub feature: Option<String>,
    pub flag: Option<String>,
}

impl From<&Entry> for KeymapEntryRecord {
    fn from(entry: &Entry) -> Self {
        // `name()` falls back to the title; the record keeps only what the table itself says.
        let name = (entry.name() != entry.title).then(|| entry.name().to_string());
        Self {
            id: id_string(entry.id),
            title: entry.title.to_string(),
            title_off: entry.title_off.map(str::to_string),
            name,
            keys: entry.keys.iter().map(KeySpecRecord::from).collect(),
            scope: entry.scope.into(),
            stage: entry.stage.into(),
            palette: entry.palette,
            debug_only: entry.debug_only,
            persistent_only: entry.persistent_only,
            feature: entry.feature.map(str::to_string),
            flag: entry.flag.map(str::to_string),
        }
    }
}

/// An action's id as text: the variant's name with a lowercase first letter, and a number after a dot for
/// the numbered ones (`Terminal(3)` is `terminal.3`). Derived from `Debug`, so a new action needs no code here.
pub(crate) fn id_string(id: ActionId) -> String {
    let debug = format!("{id:?}");
    let (name, number) = match debug.split_once('(') {
        Some((name, rest)) => (name, Some(rest.trim_end_matches(')'))),
        None => (debug.as_str(), None),
    };
    let mut chars = name.chars();
    let head: String = chars.next().into_iter().flat_map(char::to_lowercase).collect();
    match number {
        Some(n) => format!("{head}{}.{n}", chars.as_str()),
        None => format!("{head}{}", chars.as_str()),
    }
}

/// The action with this id text, if the table has one.
pub(crate) fn id_from_string(text: &str) -> Option<ActionId> {
    keymap::entries().iter().map(|entry| entry.id).find(|id| id_string(*id) == text)
}
