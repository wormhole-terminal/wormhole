//! The theme file format and its history.
//!
//! A file says which version of the format it is in `version`; none means version 0, every
//! file written before the field existed. Reading is two stages: find the version, then read
//! the file as that version's shape ([`read_theme_file`]):
//!
//! - **0** is the shape of [`ThemeFile`] itself (`extends`, flat `ansi`, the old key names).
//!   Read strictly, as it always was, then [`migrate`]d in memory.
//! - **1** (`v1`) has a `base` of copied values under the person's own blocks, an `id`, and
//!   the one naming rule for keys ([`keys`]). Read strictly: a typo is an error.
//! - **A newer version** is read as the version-1 shape, leniently: entries this reader does
//!   not know are dropped and counted ([`ReadReport::skipped`]) so the interface can say so.
//!   Caps and ranges still apply. A `version` that is not a whole number of 0 or more is an
//!   error: it says nothing about how to read the file.
//!
//! Every version ends up as one model, [`ThemeFile`], and one resolver. That a version 1 file
//! has a `base` (and so no `extends` and no implicit Tokyo Night under it) is the only
//! difference the resolver sees.
//!
//! - [`appearance`]: light and dark in one file; [`slug`]: theme names for settings and `base.from`; [`keys`]: the key table;
//!   [`copy`]: building a `base`, converting a version 0 theme, diffing a base against its
//!   source; [`export`]: writing a file older releases read; [`migrate`]: the steps.

pub mod appearance;
pub mod copy;
pub mod export;
mod keys;
pub mod migrate;
mod refs;
pub mod slug;
#[cfg(test)]
mod tests;
mod unique;
mod v1;

use serde_json::Value;

pub use self::keys::{ANSI_RULE, KEY_TABLE, KeyRename, new_key, old_key};
pub use self::v1::write_v1;
use super::file::{MAX_THEME_BYTES, ThemeFile};
use crate::storage::jsonc;

/// The version this release writes.
pub const CURRENT_VERSION: u32 = 1;

/// The values a version 1 theme was copied from another theme with: what that theme sets,
/// as it wrote it (expressions stay expressions), and a complete terminal palette. The
/// theme's own blocks go on top; nothing goes underneath.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BaseLayer {
    /// The slug of the theme it was copied from.
    pub from: String,
    /// The day it was copied (`2026-10-29`), for "copied 29 Oct 2026".
    pub copied: Option<String>,
    /// The copied layers, as a file (its own `base`, `extends` and `variants` are empty).
    pub file: ThemeFile,
}

/// What reading a file had to do, for the interface to say.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadReport {
    /// The version the file said it was.
    pub source_version: u32,
    /// Made with a newer Wormhole than this one.
    pub newer: bool,
    /// Entries of a newer file this version does not know, dropped.
    pub skipped: usize,
    /// Values a migration could not carry over (a token that no longer exists).
    pub notices: Vec<String>,
}

/// Read a theme file's text into the resolver's model, whatever its version. Errors read
/// `line L, column C: message` where the text is not JSON or a version 0 file is wrong.
pub fn read_theme_file(text: &str) -> Result<ThemeFile, String> {
    if text.len() > MAX_THEME_BYTES {
        return Err(format!("theme file is larger than {} KiB", MAX_THEME_BYTES / 1024));
    }
    match version_of(&jsonc::parse::<Value>(text)?)? {
        0 => {
            let mut file: ThemeFile = jsonc::parse(text)?;
            file.version = None;
            let notices = migrate::run(&mut file, 0);
            file.read = ReadReport { source_version: 0, notices, ..ReadReport::default() };
            Ok(file)
        }
        version => v1::read(&unique::parse_unique(text)?, version),
    }
}

/// The version a parsed file says: 0 when it says none.
fn version_of(root: &Value) -> Result<u32, String> {
    let Some(version) = root.get("version") else { return Ok(0) };
    match version.as_u64() {
        Some(n) => Ok(u32::try_from(n).unwrap_or(u32::MAX)),
        None => Err("version: expected a whole number, 0 or more".into()),
    }
}

/// The terminal colors a version 1 theme must set for itself, as the resolver names them.
const REQUIRED_TERMINAL: &[&str] = &["foreground", "background", "cursor", "selection_background"];

/// The terminal colors a version 1 theme does not set (names as the file writes them): it has
/// no theme under it to take them from, so the resolver refuses a theme with a gap.
pub(super) fn missing_terminal(set: &dyn Fn(&str) -> bool) -> Vec<String> {
    let plain = REQUIRED_TERMINAL
        .iter()
        .filter(|k| !set(k))
        .map(|k| keys::new_key("terminal", k).unwrap_or(k).to_string());
    let ansi = (0..16).filter(|i| !set(&format!("ansi.{i}"))).map(|i| format!("ansi.{i}"));
    plain.chain(ansi).collect()
}

/// A version 1 file as the resolver reads it: its own layers over its base, as one theme with
/// nothing to extend. Other files are returned as they are.
pub(super) fn effective(file: &ThemeFile) -> std::borrow::Cow<'_, ThemeFile> {
    use std::borrow::Cow;
    if file.version != Some(CURRENT_VERSION) || file.base.is_none() {
        return Cow::Borrowed(file);
    }
    let own = ThemeFile { base: None, ..file.clone() };
    let under = file.base.as_ref().map(|b| ThemeFile {
        extends: None,
        base: None,
        variants: Vec::new(),
        ..b.file.clone()
    });
    Cow::Owned(match under {
        Some(under) => own.layered_over(&under),
        None => own,
    })
}
