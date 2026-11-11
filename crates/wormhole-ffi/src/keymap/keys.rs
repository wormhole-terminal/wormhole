//! Keys as Swift asks about them: parsing and writing a spec, who has a key, what a key does where, and
//! what recording a shortcut makes of a press. All of it is core's (`keyspec`, `keymap`, `shortcut_record`).

use wormhole_core::keys::keymap::{self, ActionId, KeyUse};
use wormhole_core::keys::keyspec::{KeyMods, KeySpec, KeySpecError};
use wormhole_core::keys::shortcut_record::{self, Recorded};

use super::records::{KeyScopeRecord, KeySpecRecord, id_from_string, id_string};

/// Why a key spec's text could not be read (`KeySpecError`).
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum KeySpecParseError {
    #[error("empty key")]
    Empty,
    #[error("missing key in {spec:?}")]
    MissingKey { spec: String },
    #[error("unknown modifier {token:?}")]
    UnknownModifier { token: String },
    #[error("duplicate modifier {token:?}")]
    DuplicateModifier { token: String },
}

impl From<KeySpecError> for KeySpecParseError {
    fn from(error: KeySpecError) -> Self {
        match error {
            KeySpecError::Empty => Self::Empty,
            KeySpecError::MissingKey(spec) => Self::MissingKey { spec },
            KeySpecError::UnknownModifier(token) => Self::UnknownModifier { token },
            KeySpecError::DuplicateModifier(token) => Self::DuplicateModifier { token },
        }
    }
}

/// Who already has a key (`keymap::KeyUse`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum KeyUseRecord {
    Action { id: String, name: String, key: KeySpecRecord },
    System { what: String },
    Shell { what: String },
}

/// What a key press does to a shortcut field (`shortcut_record::Recorded`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum RecordedRecord {
    Cancel,
    Clear,
    NeedsModifier,
    Shortcut { spec: KeySpecRecord },
}

/// A macOS-reserved key and what macOS uses it for.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SystemKeyRecord {
    pub key: KeySpecRecord,
    pub what: String,
}

fn spec(record: &KeySpecRecord) -> KeySpec {
    KeySpec::from(record)
}

/// Apple's notation (`⇧⌘R`, `⌥Space`) read back; nil for anything else.
#[uniffi::export]
pub fn key_spec_from_glyphs(text: String) -> Option<KeySpecRecord> {
    KeySpec::from_glyphs(&text).as_ref().map(KeySpecRecord::from)
}

/// The config grammar (`cmd-shift-r`, `ctrl+alt+t`) read.
#[uniffi::export]
pub fn key_spec_parse(text: String) -> Result<KeySpecRecord, KeySpecParseError> {
    let parsed: KeySpec = text.parse()?;
    Ok(KeySpecRecord::from(&parsed))
}

/// Apple's notation of a spec, modifiers in ⌃⌥⇧⌘ order.
#[uniffi::export]
pub fn key_spec_glyphs(key: KeySpecRecord) -> String {
    spec(&key).glyphs()
}

/// The config grammar's canonical text, which `key_spec_parse` reads back.
#[uniffi::export]
pub fn key_spec_text(key: KeySpecRecord) -> String {
    spec(&key).to_string()
}

/// Whether two specs name the same key press, written either way (`⌘}` and `⇧⌘]`).
#[uniffi::export]
pub fn key_spec_same_key(a: KeySpecRecord, b: KeySpecRecord) -> bool {
    spec(&a).same_key(&spec(&b))
}

/// The keys macOS keeps for itself.
#[uniffi::export]
pub fn keymap_system_keys() -> Vec<SystemKeyRecord> {
    keymap::system_keys()
        .map(|(key, what)| SystemKeyRecord { key: KeySpecRecord::from(key), what: what.to_string() })
        .collect()
}

/// Who has `key`, if anyone: the app, macOS or the shell.
#[uniffi::export]
pub fn key_uses(key: KeySpecRecord) -> Option<KeyUseRecord> {
    Some(match keymap::uses(&spec(&key))? {
        KeyUse::Action(entry, own) => KeyUseRecord::Action {
            id: id_string(entry.id),
            name: entry.name().to_string(),
            key: KeySpecRecord::from(own),
        },
        KeyUse::System(what) => KeyUseRecord::System { what: what.to_string() },
        KeyUse::Shell(what) => KeyUseRecord::Shell { what: what.to_string() },
    })
}

/// The key `key` (lowercase) pressed with `mods` (the mask of `KeySpecRecord`) while recording a shortcut.
#[uniffi::export]
pub fn record_shortcut(mods: u8, key: String) -> RecordedRecord {
    let mods = KeyMods { ctrl: mods & 1 != 0, alt: mods & 2 != 0, shift: mods & 4 != 0, cmd: mods & 8 != 0 };
    match shortcut_record::record(mods, &key) {
        Recorded::Cancel => RecordedRecord::Cancel,
        Recorded::Clear => RecordedRecord::Clear,
        Recorded::NeedsModifier => RecordedRecord::NeedsModifier,
        Recorded::Shortcut(spec) => RecordedRecord::Shortcut { spec: KeySpecRecord::from(&spec) },
    }
}

/// What already has `key`, as the shortcut field says it; `own` is the action the field sets the key of.
#[uniffi::export]
pub fn shortcut_conflict(key: KeySpecRecord, own: Option<String>) -> Option<String> {
    let own: Option<ActionId> = own.as_deref().and_then(id_from_string);
    shortcut_record::conflict(&spec(&key), own)
}

/// The action `key` runs where `scope` applies (an app key runs everywhere), live or not.
#[uniffi::export]
pub fn key_action_for(key: KeySpecRecord, scope: KeyScopeRecord) -> Option<String> {
    keymap::action_for(&spec(&key), scope.into()).map(|entry| id_string(entry.id))
}

/// Whether the app takes `key` even from a focused terminal.
#[uniffi::export]
pub fn key_terminal_yields(key: KeySpecRecord) -> bool {
    keymap::terminal_yields(&spec(&key))
}

/// The action a focused terminal runs itself for `key` (copy, paste, clear, ⌘←).
#[uniffi::export]
pub fn key_terminal_action(key: KeySpecRecord) -> Option<String> {
    keymap::terminal_action(&spec(&key)).map(id_string)
}

/// The action `key` runs in a list or its item editor.
#[uniffi::export]
pub fn key_list_action(key: KeySpecRecord) -> Option<String> {
    keymap::list_action(&spec(&key)).map(id_string)
}

/// The action `key` runs in the Theme Studio's window.
#[uniffi::export]
pub fn key_studio_action(key: KeySpecRecord) -> Option<String> {
    keymap::studio_action(&spec(&key)).map(id_string)
}
