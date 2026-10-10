//! Agents whose hooks live in a shared JSON file as `<Event>: [entry, …]`,
//! usually under a `hooks` key (Claude Code, Codex, Gemini CLI, Qwen Code,
//! Droid). Only the location of the event map and the entry shape differ.

use std::path::Path;

use serde_json::{Map, Value, json};

use super::{AgentId, State, files, hook_command, is_legacy_command, is_our_command, status_line};

pub struct Format {
    /// Key holding the event map; `None` when events are top-level keys.
    pub root: Option<&'static str>,
    pub events: &'static [&'static str],
    /// Builds one hook entry around a shell command.
    pub entry: fn(String) -> Value,
    /// Also the file's `statusLine`, which hands over the plan usage limits (Claude Code).
    pub status_line: bool,
}

/// Claude-style entry; `timeout` in seconds (Claude, Codex, Droid).
fn claude_entry(command: String) -> Value {
    json!({ "hooks": [{ "type": "command", "command": command, "timeout": 5 }] })
}

pub const CLAUDE: Format = Format {
    root: Some("hooks"),
    events: &[
        "UserPromptSubmit",
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "Notification",
        "Stop",
        "StopFailure",
        "SessionEnd",
    ],
    entry: claude_entry,
    status_line: true,
};

pub const CODEX: Format = Format {
    root: Some("hooks"),
    events: &["UserPromptSubmit", "PermissionRequest", "PostToolUse", "Stop", "SessionEnd"],
    entry: claude_entry,
    status_line: false,
};

pub const GEMINI: Format = Format {
    root: Some("hooks"),
    events: &["BeforeAgent", "AfterTool", "Notification", "AfterAgent", "SessionEnd"],
    // Gemini's timeout is in milliseconds.
    entry: |command| {
        json!({ "hooks": [{
            "name": "wormhole-status", "type": "command", "command": command, "timeout": 5000
        }] })
    },
    status_line: false,
};

pub const QWEN: Format = Format {
    root: Some("hooks"),
    events: &["UserPromptSubmit", "PostToolUse", "Notification", "Stop", "SessionEnd"],
    entry: claude_entry,
    status_line: false,
};

pub const DROID: Format = Format {
    root: None,
    events: &["UserPromptSubmit", "PostToolUse", "Notification", "Stop", "SessionEnd"],
    entry: claude_entry,
    status_line: false,
};

/// Any string inside the entry that runs our hook command. The whole entry
/// counts as ours then: a command the user added inside our entry's `hooks`
/// list goes with it on uninstall.
pub fn is_ours(entry: &Value) -> bool {
    match entry {
        Value::String(s) => is_our_command(s),
        Value::Array(items) => items.iter().any(is_ours),
        Value::Object(map) => map.values().any(is_ours),
        _ => false,
    }
}

fn events<'a>(format: &Format, settings: &'a Value) -> Option<&'a Value> {
    match format.root {
        Some(key) => settings.get(key),
        None => Some(settings),
    }
}

fn has_ours(format: &Format, settings: &Value, event: &str) -> bool {
    events(format, settings)
        .and_then(|map| map.get(event))
        .and_then(Value::as_array)
        .is_some_and(|entries| entries.iter().any(is_ours))
}

/// Any entry or status line of the hooks an earlier version installed under its old names.
fn legacy_in(format: &Format, settings: &Value) -> bool {
    let entries =
        events(format, settings).and_then(Value::as_object).into_iter().flat_map(|map| map.values());
    entries.flat_map(|e| e.as_array().into_iter().flatten()).any(is_legacy)
        || (format.status_line && status_line::has_legacy(settings))
}

fn is_legacy(entry: &Value) -> bool {
    match entry {
        Value::String(s) => is_legacy_command(s),
        Value::Array(items) => items.iter().any(is_legacy),
        Value::Object(map) => map.values().any(is_legacy),
        _ => false,
    }
}

pub fn has_legacy(format: &Format, path: &Path) -> bool {
    path.exists() && files::read_json(path).is_ok_and(|settings| legacy_in(format, &settings))
}

pub fn state(format: &Format, path: &Path) -> Result<State, String> {
    let settings = files::read_json(path)?;
    // Hooks under the old names still work but are due to be replaced: not "installed" yet.
    if legacy_in(format, &settings) {
        return Ok(State::Partial);
    }
    let present = format.events.iter().filter(|e| has_ours(format, &settings, e)).count()
        + usize::from(format.status_line && status_line::installed(&settings));
    Ok(match present {
        0 => State::NotInstalled,
        n if n == format.events.len() + usize::from(format.status_line) => State::Installed,
        _ => State::Partial,
    })
}

pub fn install(format: &Format, agent: AgentId, path: &Path) -> Result<(), String> {
    let mut settings = files::read_json(path)?;
    // Hooks under the old names go first, so today's take their place.
    let replaced = remove_legacy(format, &mut settings);
    let missing: Vec<&str> =
        format.events.iter().copied().filter(|e| !has_ours(format, &settings, e)).collect();
    let line_added = format.status_line && status_line::install(&mut settings)?;
    if missing.is_empty() {
        return if line_added || replaced { files::write_json(path, &settings) } else { Ok(()) };
    }
    let map = event_map(format, &mut settings)?;
    for event in missing {
        let entries = map.entry(event).or_insert_with(|| json!([]));
        let entries = entries.as_array_mut().ok_or(format!("{event} is not an array"))?;
        entries.push((format.entry)(hook_command(agent, event)));
    }
    files::write_json(path, &settings)
}

/// Take out the entries of the hooks an earlier version installed under its old names, and any
/// event list that leaves empty. Whether anything went.
fn remove_legacy(format: &Format, settings: &mut Value) -> bool {
    let Ok(map) = event_map_existing(format, settings) else { return false };
    let mut changed = false;
    let mut empty = Vec::new();
    for (event, entries) in map.iter_mut() {
        let Some(entries) = entries.as_array_mut() else { continue };
        let before = entries.len();
        entries.retain(|entry| !is_legacy(entry));
        if entries.len() != before {
            changed = true;
            if entries.is_empty() {
                empty.push(event.clone());
            }
        }
    }
    for event in empty {
        map.shift_remove(&event);
    }
    changed
}

pub fn uninstall(format: &Format, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut settings = files::read_json(path)?;
    let mut changed = format.status_line && status_line::uninstall(&mut settings);
    let emptied = {
        let Ok(map) = event_map_existing(format, &mut settings) else {
            return if changed { files::write_json(path, &settings) } else { Ok(()) };
        };
        // Every event, not just today's list: one a later version stopped
        // installing (or the user moved our entry to) would keep ours forever.
        let mut empty = Vec::new();
        for (event, entries) in map.iter_mut() {
            let Some(entries) = entries.as_array_mut() else { continue };
            let before = entries.len();
            entries.retain(|entry| !is_ours(entry));
            if entries.len() != before {
                changed = true;
                if entries.is_empty() {
                    empty.push(event.clone());
                }
            }
        }
        for event in empty {
            // `shift_remove` keeps the order of the user's other keys.
            map.shift_remove(&event);
        }
        map.is_empty()
    };
    // Drop a `hooks` object we leave empty, but never the whole file.
    if emptied && let (Some(key), Some(obj)) = (format.root, settings.as_object_mut()) {
        obj.shift_remove(key);
    }
    if changed { files::write_json(path, &settings) } else { Ok(()) }
}

fn event_map<'a>(format: &Format, settings: &'a mut Value) -> Result<&'a mut Map<String, Value>, String> {
    let obj = settings.as_object_mut().ok_or("the settings file is not a JSON object")?;
    match format.root {
        Some(key) => obj
            .entry(key)
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or(format!("\"{key}\" is not an object")),
        None => Ok(obj),
    }
}

fn event_map_existing<'a>(
    format: &Format,
    settings: &'a mut Value,
) -> Result<&'a mut Map<String, Value>, String> {
    match format.root {
        Some(key) => settings.get_mut(key).and_then(Value::as_object_mut).ok_or_else(String::new),
        None => settings.as_object_mut().ok_or_else(String::new),
    }
}
