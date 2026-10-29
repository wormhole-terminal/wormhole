//! What the Studio asks of the themes folder, as functions, so a test can stand in and never read
//! or write the user's: finding the theme a pick names, reading a theme file with the stamp it
//! had, writing one (keeping the file it replaces as a saved version first) and picking an id.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use wormhole_core::config::Config;
use wormhole_core::storage::files::{self, LinkPolicy};
use wormhole_core::theme::catalog::{ThemeEntry, catalog_in};
use wormhole_core::theme::registry::ThemeSource as _;
use wormhole_core::theme::registry::{ThemeId, UserThemes};
use wormhole_core::theme::saved_versions::keep_version_now;
use wormhole_core::theme::tokens::{MAX_THEME_BYTES, ThemeFile, ThemeTokens, copy, read_theme_file, slug};

use super::names::today;

/// Turns a draft into tokens. The Studio resolves with the user's themes available for
/// `extends`; a test passes one that reads nothing from disk.
pub(in crate::studio) type Resolve = fn(&ThemeFile) -> Result<ThemeTokens, String>;

/// A file as it was when the Studio read or wrote it: if it is different at the next write,
/// something else changed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::studio) struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}

impl Stamp {
    pub(in crate::studio) fn of(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(Self { len: meta.len(), modified: meta.modified().ok() })
    }
}

/// What a pick selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::studio) enum Found {
    /// A theme file of the themes folder.
    File(PathBuf),
    /// One version of a file that ships several (a light and a dark one).
    Variant(PathBuf),
    /// A built-in theme, by name.
    Builtin(String),
    /// Nothing: the app shows its default.
    Missing,
}

/// A theme file read.
pub(in crate::studio) struct Loaded {
    pub(in crate::studio) file: ThemeFile,
    pub(in crate::studio) stamp: Option<Stamp>,
}

/// A theme file written.
pub(in crate::studio) struct Saved {
    pub(in crate::studio) path: PathBuf,
    pub(in crate::studio) stamp: Option<Stamp>,
    /// The file it replaced had comments, which the new one does not keep.
    pub(in crate::studio) lost_comments: bool,
}

pub(in crate::studio) enum SaveError {
    /// The file is not what it was when it was read: another program wrote it.
    ChangedOnDisk,
    Other(String),
}

#[derive(Clone, Copy)]
pub(in crate::studio) struct Themes {
    pub(in crate::studio) resolve: Resolve,
    /// The file a theme saved under this name would replace, if the folder has one.
    pub(in crate::studio) saved_path: fn(&str) -> Option<PathBuf>,
    /// Every theme there is to open, built-ins first.
    pub(in crate::studio) catalog: fn() -> Vec<ThemeEntry>,
    /// What a pick's value selects.
    pub(in crate::studio) locate: fn(&str) -> Found,
    pub(in crate::studio) read: fn(&Path) -> Result<Loaded, String>,
    /// A theme with no file yet, into the folder under a file name of its own.
    pub(in crate::studio) save_new: fn(&ThemeFile) -> Result<Saved, String>,
    /// A theme over its own file, when that is still as it was read (`expected`; `None` writes
    /// whatever is there).
    pub(in crate::studio) overwrite: fn(&Path, &ThemeFile, Option<Stamp>) -> Result<Saved, SaveError>,
    /// The id a theme named like this gets: nobody else's, no built-in's. The file at the path (if
    /// any) is the theme itself, whose own id does not count against it.
    pub(in crate::studio) new_id: fn(&str, Option<&Path>) -> String,
    /// A theme in the older format as one in the new, with this id (see `convert`).
    pub(in crate::studio) convert: fn(&ThemeFile, &str) -> Result<ThemeFile, String>,
    /// Whether a pick's value (a name, a path or a slug) names the theme saved at this path.
    pub(in crate::studio) selects: fn(&str, &Path) -> bool,
}

impl Themes {
    /// The user's themes folder.
    pub(in crate::studio) const USER: Themes = Themes {
        resolve: resolve_in_profile,
        saved_path: saved_in_profile,
        catalog: catalog_in_profile,
        locate: locate_in_profile,
        read: read_in_profile,
        save_new: save_new_in_profile,
        overwrite: overwrite_in_profile,
        new_id: new_id_in_profile,
        convert: convert_in_profile,
        selects: selects_in_profile,
    };
}

// The Studio edits the themes of the profile Wormhole is in (RF-21): its own folder, not Default's.
fn dir() -> PathBuf {
    crate::active_profile::paths().themes_dir()
}

fn resolve_in_profile(file: &ThemeFile) -> Result<ThemeTokens, String> {
    wormhole_core::config::resolve_theme_file_in(&dir(), file)
}

fn saved_in_profile(name: &str) -> Option<PathBuf> {
    Config::saved_theme_path_in(&dir(), name)
}

fn catalog_in_profile() -> Vec<ThemeEntry> {
    catalog_in(&dir())
}

fn locate_in_profile(setting: &str) -> Found {
    match UserThemes::read(&dir()).select(setting) {
        Some(ThemeId::File(path)) => Found::File(path),
        Some(ThemeId::Variant(path, _)) => Found::Variant(path),
        Some(ThemeId::Builtin(name)) => Found::Builtin(name.to_string()),
        None => Found::Missing,
    }
}

fn read_in_profile(path: &Path) -> Result<Loaded, String> {
    let text = files::read_to_string_capped(path, MAX_THEME_BYTES as u64, LinkPolicy::Follow)
        .map_err(|e| e.to_string())?;
    Ok(Loaded { file: read_theme_file(&text)?, stamp: Stamp::of(path) })
}

fn selects_in_profile(setting: &str, path: &Path) -> bool {
    UserThemes::read(&dir()).select(setting) == Some(ThemeId::File(path.to_path_buf()))
}

fn new_id_in_profile(name: &str, itself: Option<&Path>) -> String {
    let taken: Vec<String> = UserThemes::read(&dir())
        .themes
        .into_iter()
        .filter(|t| Some(t.path.as_path()) != itself)
        .map(|t| t.slug)
        .collect();
    let wanted = slug::derive_id(name);
    std::iter::once(wanted.clone())
        .chain((2..).map(|n| format!("{wanted}_{n}")))
        .find(|id| !slug::is_reserved(id) && !taken.contains(id))
        .unwrap_or(wanted)
}

fn convert_in_profile(file: &ThemeFile, id: &str) -> Result<ThemeFile, String> {
    let themes = UserThemes::read(&dir());
    let lookup = |name: &str| themes.theme_file(name);
    let slug_of = |name: &str| match slug::builtin_slug(name) {
        Some(builtin) => builtin.to_string(),
        None => themes.named(name).map_or_else(|| slug::derive_id(name), |t| t.slug.clone()),
    };
    copy::convert_to_v1(file, &lookup, &slug_of, id, &today())
}

/// Whether JSON text has a `//` or `/* */` comment outside its strings.
pub(in crate::studio) fn has_comments(text: &str) -> bool {
    let (mut in_string, mut escaped) = (false, false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
        } else if c == '"' {
            in_string = true;
        } else if c == '/' && matches!(chars.peek(), Some('/' | '*')) {
            return true;
        }
    }
    false
}

/// Keep the file `path` is, as a saved version, before it is replaced. A version that can't be
/// kept is logged and does not stop the save: the person's edit matters more than the history.
/// Whether it had comments, which the file written over it will not.
fn keep_old(themes: &Path, path: &Path, file: &ThemeFile) -> bool {
    let Ok(old) = files::read_to_string_capped(path, MAX_THEME_BYTES as u64, LinkPolicy::Follow) else {
        return false;
    };
    let id = UserThemes::read(themes)
        .themes
        .iter()
        .find(|t| t.path == path && t.family.is_none())
        .map(|t| t.slug.clone())
        .or_else(|| file.id.clone())
        .unwrap_or_else(|| slug::derive_id(file.name.as_deref().unwrap_or("theme")));
    if let Err(err) = keep_version_now(themes, &id, &old) {
        tracing::warn!("keeping the earlier {}: {err}", path.display());
    }
    has_comments(&old)
}

fn save_new_in_profile(file: &ThemeFile) -> Result<Saved, String> {
    let dir = dir();
    // A theme of the same name is replaced in its own file: the old one is kept first.
    let lost_comments = file
        .name
        .as_deref()
        .and_then(|n| Config::saved_theme_path_in(&dir, n))
        .is_some_and(|existing| keep_old(&dir, &existing, file));
    let path = Config::save_theme_to(&dir, file)?;
    Ok(Saved { stamp: Stamp::of(&path), path, lost_comments })
}

fn overwrite_in_profile(path: &Path, file: &ThemeFile, expected: Option<Stamp>) -> Result<Saved, SaveError> {
    if expected.is_some() && Stamp::of(path) != expected {
        return Err(SaveError::ChangedOnDisk);
    }
    let lost_comments = keep_old(&dir(), path, file);
    files::write_atomic(path, file.to_json_pretty().as_bytes(), LinkPolicy::Follow)
        .map_err(|e| SaveError::Other(e.to_string()))?;
    Ok(Saved { path: path.to_path_buf(), stamp: Stamp::of(path), lost_comments })
}

#[cfg(test)]
mod tests {
    use super::has_comments;

    #[test]
    fn comments_are_found_outside_strings_only() {
        assert!(has_comments("{\n  // the accent\n  \"ui\": {}\n}"));
        assert!(has_comments("{ /* a */ \"name\": \"x\" }"));
        assert!(!has_comments(r#"{"name": "a // not a comment", "ui": {"p": "http://x"}}"#));
        assert!(!has_comments(r#"{"name": "quote \" // still a string"}"#));
        assert!(!has_comments("{}"));
    }
}
