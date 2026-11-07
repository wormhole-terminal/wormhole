//! The settings window's schema as JSON, for the Swift UI's `SettingsModel` to read.
//!
//! What every section, group and setting is called and says, the words that find it and the `config.json`
//! keys it writes are `wormhole-core`'s `settings_index`; Swift never re-types them, it loads this dump. The
//! kind of control a row has (a switch, a segmented control, a stepper, ...) is `controls`, the one table
//! that the app crate's `settings_window/registry.rs` holds as closures over `Settings` today and that moves
//! into core with the FFI (`repo-docs/docs/research/swift-parity-core-surface.md`, part 2). A single `bool`
//! key with no entry in that table is a switch. Deterministic: same core, same bytes. `examples/dump_settings.rs`
//! writes it and `tests/settings_dump.rs` fails when the committed copy is stale.

mod controls;
mod options;

use serde_json::{Map, Value, json};
use wormhole_core::config::Settings;
use wormhole_core::config::settings_index::{Entry, Group, SectionId};

/// Bumped when the shape of the JSON changes, so the Swift loader can refuse a file it can't read.
const FORMAT: u32 = 1;

pub fn json() -> String {
    let defaults = serde_json::to_value(Settings::default()).unwrap_or(Value::Null);
    let sections: Vec<Value> = SectionId::ALL.into_iter().filter_map(|s| section(s, &defaults)).collect();
    let mut keys = Map::new();
    for key in sections.iter().flat_map(entry_keys) {
        if let Some(value) = get(&defaults, &key) {
            keys.insert(key, tidy(value));
        }
    }
    let doc = json!({ "format": FORMAT, "sections": sections, "defaults": keys });
    let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
    text.push('\n');
    text
}

fn entry_keys(section: &Value) -> Vec<String> {
    let groups = section["groups"].as_array().into_iter().flatten();
    let entries = groups.flat_map(|g| g["entries"].as_array().into_iter().flatten());
    entries
        .flat_map(|e| e["keys"].as_array().into_iter().flatten())
        .filter_map(|k| k.as_str().map(str::to_string))
        .collect()
}

/// A `config.json` key, which may be a path (`motion.insert`).
fn get<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    key.split('.').try_fold(root, |value, part| value.get(part))
}

/// Preview sections, the Changed page and the Components specimens are not part of the experiment's
/// pages: Changed is drawn from the others' rows, Components is a GPUI development tool.
fn section(id: SectionId, defaults: &Value) -> Option<Value> {
    if matches!(id, SectionId::Components) {
        return None;
    }
    let info = id.info();
    let groups: Vec<Value> = id.groups().iter().map(|g| group(g, defaults)).collect();
    Some(json!({
        "id": section_id(id),
        "title": info.title(),
        "intro": info.intro(),
        "nav": format!("{:?}", info.nav).to_lowercase(),
        "navTitle": info.nav.title(),
        "preview": id.is_preview(),
        "groups": groups,
    }))
}

fn group(g: &Group, defaults: &Value) -> Value {
    let entries: Vec<Value> = g.entries.iter().map(|e| entry(e, defaults)).collect();
    json!({ "title": g.title(), "blurb": g.blurb(), "aside": g.aside(), "entries": entries })
}

fn entry(e: &Entry, defaults: &Value) -> Value {
    let default_value = |key: &str| get(defaults, key).cloned();
    let control =
        controls::control(e.id, e.keys, &default_value).unwrap_or_else(|| json!({ "kind": "unsupported" }));
    let mut row = json!({
        "id": e.id,
        "label": e.label(),
        "description": e.description(),
        "keywords": e.keywords(),
        "keys": e.keys,
        "visibleHint": e.visible_hint,
        "control": control,
    });
    if let Some(feature) = e.feature {
        row["feature"] = json!(format!("{feature:?}").to_lowercase());
    }
    if let Some(when) = controls::visible_when(e.id) {
        row["visibleWhen"] = when;
    }
    row
}

/// Swift's spelling of a section: lower camel, as the nav and the tests name them.
fn section_id(id: SectionId) -> String {
    let name = format!("{id:?}");
    let mut chars = name.chars();
    chars.next().map(|c| c.to_lowercase().chain(chars).collect()).unwrap_or_default()
}

/// The settings hold `f32`s that serde widens (0.8 becomes 0.800000011920929): write the shortest text that
/// reads back as the same `f32`, as the app shows it.
fn tidy(value: &Value) -> Value {
    match value {
        Value::Number(n) if !n.is_i64() && !n.is_u64() => n
            .as_f64()
            .and_then(|v| (v as f32).to_string().parse::<f64>().ok())
            .and_then(serde_json::Number::from_f64)
            .map_or_else(|| value.clone(), Value::Number),
        Value::Array(items) => Value::Array(items.iter().map(tidy).collect()),
        other => other.clone(),
    }
}
