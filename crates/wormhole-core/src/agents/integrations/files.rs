//! Careful edits of other programs' config files: read, back up once per
//! change, write atomically, and never touch a file we can't parse.

use std::path::Path;

use serde_json::{Value, json};

use crate::storage::files::{self, LinkPolicy};

/// Suffix of the copy kept next to a config file before we change it.
pub(crate) const BACKUP_SUFFIX: &str = "wormhole-backup";

pub fn display(path: &Path) -> String {
    let home = dirs::home_dir().unwrap_or_default();
    match path.strip_prefix(&home) {
        Ok(rest) if !home.as_os_str().is_empty() => format!("~/{}", rest.display()),
        _ => path.display().to_string(),
    }
}

/// The text of a config file, or `None` if it doesn't exist yet.
pub fn read_text(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("{}: {err}", display(path))),
    }
}

/// A JSON object config; missing or empty files are `{}`. Files with
/// comments are refused: writing them back would drop the comments.
pub fn read_json(path: &Path) -> Result<Value, String> {
    let Some(text) = read_text(path)? else { return Ok(json!({})) };
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| {
        if crate::storage::jsonc::parse::<Value>(&text).is_ok() {
            format!("{} contains comments; add the hooks by hand to keep them", display(path))
        } else {
            format!("{} is not valid JSON: {err}", display(path))
        }
    })?;
    if !value.is_object() {
        return Err(format!("{} is not a JSON object", display(path)));
    }
    Ok(value)
}

pub fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    write_text(path, &(text + "\n"))
}

/// Keep a copy of the current file, then replace it atomically.
///
/// These are the user's own files: a symlinked one (a dotfiles setup) is written
/// through so the link stays, and the file keeps its mode, since a 0600
/// `settings.json` may hold API keys. `fs::copy` keeps the mode for the backup.
pub fn write_text(path: &Path, text: &str) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{}: {e}", display(path));
    if path.exists() {
        std::fs::copy(path, with_suffix(path, BACKUP_SUFFIX)).map_err(err)?;
    }
    // Creates missing folders (a first install of `~/.gemini/settings.json`).
    files::write_atomic(path, text.as_bytes(), LinkPolicy::Follow).map_err(err)
}

/// `settings.json` → `settings.json.<suffix>`.
pub fn with_suffix(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}
