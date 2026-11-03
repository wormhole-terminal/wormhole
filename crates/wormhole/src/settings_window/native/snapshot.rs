//! The snapshot the native window is given: every wired row's value, default and options, the rows that differ
//! from their defaults, the themes and the fonts.

use std::collections::HashSet;

use gpui_kit::App;
use serde_json::{Map, Value, json};
use wormhole_core::config::Settings;
use wormhole_core::config::settings_index::{self, Values};

use super::codec::{self, Ctx};
use super::{lists, open_in, preview, recorder, rows, system, themes, words};
use crate::fonts::FontCatalog;
use crate::settings::SettingsStore;

/// The row of the theme picker, which carries the import's failure.
const THEME_ROW: &str = "Appearance:0:2";

pub(super) fn build(separate: Option<bool>, cx: &mut App) -> Value {
    let entries = themes::entries(cx);
    let ctx = Ctx { themes: &entries, separate };
    let store = cx.global::<SettingsStore>();
    let s = store.user_settings();
    let defaults = Settings::default();
    let values_now = Values::of(s);

    let (mut values, mut default_values, mut options) = (Map::new(), Map::new(), Map::new());
    let (mut changed, mut known) = (Vec::new(), Vec::new());
    for (key, id) in rows::all() {
        if *id == open_in::ROW {
            let row = open_in::row(cx);
            values.insert((*key).into(), Value::String(row.value.clone()));
            default_values.insert((*key).into(), Value::String(row.default.clone()));
            options.insert((*key).into(), json!(row.options));
            known.push((*key).to_string());
            if row.value != row.default {
                changed.push(*key);
            }
            continue;
        }
        let now = codec::read(id, s, &ctx);
        if now.is_empty() {
            continue;
        }
        let default = codec::read(id, &defaults, &ctx);
        for ((suffix, value), (_, default)) in now.into_iter().zip(default) {
            let full = format!("{key}{suffix}");
            values.insert(full.clone(), Value::String(value));
            default_values.insert(full.clone(), Value::String(default));
            known.push(full);
        }
        if let Some(words) = codec::options(id, s) {
            options.insert((*key).into(), json!(words));
        }
        if settings_index::entry(id).is_some_and(|(_, _, entry)| values_now.entry_changed(entry)) {
            changed.push(*key);
        }
    }
    for (key, value) in system::fact_values() {
        values.insert(key.clone(), value.clone());
        default_values.insert(key.clone(), value);
        known.push(key);
    }
    let themes = themes::snapshot(&entries, s);
    let preview = preview::build(&entries, s, cx);
    let (lists, sheets) = lists::build(cx);
    let (recording, mut notes) = recorder::snapshot(cx);
    if let (Some(failure), Some(notes)) = (super::import_failure(cx), notes.as_object_mut()) {
        notes.insert(THEME_ROW.into(), Value::String(failure));
    }
    let counted = |count: usize| {
        let count = wormhole_text::whole(count);
        json!({ "n": count, "words": {
            "changed": wormhole_text::keys::settings_native_changed_count(count),
            "reset": wormhole_text::keys::settings_native_reset_count(count),
        } })
    };
    let counts = counted(changed.len());
    json!({
        "counts": counts, "values": values, "defaults": default_values, "options": options,
        "changed": changed, "known": known, "themes": themes, "fonts": fonts(cx), "lists": lists, "sheets": sheets, "profile": lists::profile(cx),
        "recording": recording, "notes": notes, "preview": preview, "words": words::snapshot(),
        "update": system::update_card(cx), "quiet": system::quiet_notices(cx), "banner": system::banner(cx),
    })
}

/// The installed families and whether each is monospaced; the families in use are there even before the listing is.
fn fonts(cx: &App) -> Value {
    let store = cx.global::<SettingsStore>();
    let s = store.user_settings();
    let catalog = cx.try_global::<FontCatalog>();
    let mono: Option<&HashSet<String>> = catalog.and_then(|c| c.monospace.as_deref());
    let mut names: Vec<String> = catalog.map(|c| c.names.to_vec()).unwrap_or_default();
    for family in [&s.font_family, &s.code_font_family, &s.ui_font_family] {
        let family = family.trim();
        if !family.is_empty() && !family.starts_with('.') && !names.iter().any(|n| n == family) {
            names.push(family.to_string());
        }
    }
    Value::Array(names.into_iter().map(|n| json!([n, mono.is_some_and(|m| m.contains(&n))])).collect())
}
