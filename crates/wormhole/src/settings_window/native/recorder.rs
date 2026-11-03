//! The shortcut recorders of the native window (the global hotkey, the jump to the next waiting terminal). The
//! window captures the key press (AppKit's event monitor) and sends it as an event; what it means is decided here
//! by the code the GPUI recorder uses (`shortcut_record`, the row's `accepts`, the key table's conflicts), so both
//! windows accept and refuse the same keys. The state lives here, not in the window: the snapshot says which row
//! records (the window then listens for keys) and carries the warning of a row (keys taken, a hotkey that did not
//! register).

use gpui_kit::App;
use serde_json::{Map, Value};
use wormhole_core::keys::keyspec::{KeyMods, KeySpec};
use wormhole_core::keys::shortcut_record::{self, Recorded};
use wormhole_text::keys::settings_menu_bar_key_conflict;

use super::{Native, rows};
use crate::settings::{settings, update_settings};
use crate::settings_window::recorder::HOTKEY_ROW;
use crate::settings_window::registry;

/// A recording in progress, or the keys it refused.
pub(super) struct Recording {
    /// The window's row key.
    key: String,
    /// The settings index id of the row.
    id: &'static str,
    /// Keys somebody else has, and who: shown on the row, not taken; the row no longer records.
    taken: Option<String>,
}

fn replace(recording: Option<Recording>, cx: &mut App) {
    // The global hotkey is held off only while its own field waits for keys.
    let pause = recording.as_ref().is_some_and(|r| r.id == HOTKEY_ROW && r.taken.is_none());
    if cx.has_global::<Native>() {
        cx.global_mut::<Native>().recording = recording;
    }
    crate::hotkey::pause(pause, cx);
}

/// The row `key` starts to record.
pub(super) fn start(key: &str, cx: &mut App) {
    let Some(id) = rows::id_of(key) else { return };
    if registry::recorder_row(id).is_some() {
        replace(Some(Recording { key: key.to_string(), id, taken: None }), cx);
    }
}

/// The window stopped (Esc is a key press; this is a click away, a page change or the window closing).
pub(super) fn stop(cx: &mut App) {
    replace(None, cx);
}

/// A key pressed while a row records: `mods` lists the held modifiers (`ctrl`, `alt`, `shift`, `cmd`), `name` is
/// the key as GPUI names it.
pub(super) fn key(mods: &str, name: &str, cx: &mut App) {
    let Some(recording) = cx.try_global::<Native>().and_then(|n| n.recording.as_ref()) else { return };
    if recording.taken.is_some() {
        return;
    }
    let (key, id) = (recording.key.clone(), recording.id);
    let Some(row) = registry::recorder_row(id) else { return };
    let held = |word: &str| mods.split(',').any(|m| m == word);
    let mods = KeyMods { cmd: held("cmd"), ctrl: held("ctrl"), alt: held("alt"), shift: held("shift") };
    match shortcut_record::record(mods, name) {
        Recorded::Cancel => stop(cx),
        Recorded::Clear => {
            stop(cx);
            update_settings(cx, |s| (row.set)(s, String::new()));
        }
        Recorded::NeedsModifier => {}
        // Keys this row can't use: wait for others.
        Recorded::Shortcut(spec) if !(row.accepts)(&spec) => {}
        Recorded::Shortcut(spec) => match shortcut_record::conflict(&spec, row.own) {
            Some(conflict) => replace(Some(Recording { key, id, taken: Some(conflict) }), cx),
            None => {
                stop(cx);
                update_settings(cx, |s| (row.set)(s, spec.to_string()));
            }
        },
    }
}

/// The shortcut as the window shows it ("⌥Space"); nothing when there is none or it does not parse.
pub(super) fn glyphs(stored: &str) -> String {
    stored.trim().parse::<KeySpec>().map(|spec| spec.glyphs()).unwrap_or_default()
}

/// For the snapshot: the row that records (or ""), and each recorder row's warning.
pub(super) fn snapshot(cx: &App) -> (String, Value) {
    let recording = cx.try_global::<Native>().and_then(|n| n.recording.as_ref());
    let active = recording.filter(|r| r.taken.is_none()).map(|r| r.key.clone()).unwrap_or_default();
    let mut notes = Map::new();
    for (key, id) in rows::all() {
        let Some(row) = registry::recorder_row(id) else { continue };
        let note = match recording {
            Some(r) if r.key == *key && r.taken.is_some() => {
                r.taken.as_deref().map(settings_menu_bar_key_conflict)
            }
            _ => {
                let parsed = (row.get)(settings(cx)).trim().parse::<KeySpec>().is_ok();
                parsed.then(|| (row.problem)(settings(cx), cx)).flatten()
            }
        };
        if let Some(note) = note {
            notes.insert((*key).into(), Value::String(note));
        }
    }
    (active, Value::Object(notes))
}

#[cfg(test)]
mod tests {
    use super::glyphs;

    #[test]
    fn a_stored_shortcut_is_shown_as_key_caps() {
        assert_eq!(glyphs("alt-space"), "⌥Space");
        assert_eq!(glyphs(""), "");
    }
}
