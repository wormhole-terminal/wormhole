//! What the Studio asks of the themes folder, as functions, so a test can stand in and never read
//! or write the user's: finding the theme a pick names, reading a theme file with the stamp it
//! had, writing one (keeping the file it replaces as a saved version first) and picking an id.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use wormhole_core::config::Config;
use wormhole_core::storage::files::{self, LinkPolicy};
use wormhole_core::theme::registry::{ThemeId, UserThemes};
use wormhole_core::theme::saved_versions::keep_version_now;
use wormhole_core::theme::tokens::{MAX_THEME_BYTES, ThemeFile, ThemeTokens, read_theme_file, slug};

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
    /// What a pick's value selects.
    pub(in crate::studio) locate: fn(&str) -> Found,
    pub(in crate::studio) read: fn(&Path) -> Result<Loaded, String>,
    /// A theme with no file yet, into the folder under a file name of its own.
    pub(in crate::studio) save_new: fn(&ThemeFile) -> Result<Saved, String>,
    /// A theme over its own file, when that is still as it was read (`expected`; `None` writes
    /// whatever is there).
    pub(in crate::studio) overwrite: fn(&Path, &ThemeFile, Option<Stamp>) -> Result<Saved, SaveError>,
    /// The id a theme named like this gets: nobody else's, no built-in's.
    pub(in crate::studio) new_id: fn(&str) -> String,
    /// Whether a pick's value (a name, a path or a slug) names the theme saved at this path.
    pub(in crate::studio) selects: fn(&str, &Path) -> bool,
}

impl Themes {
    /// The user's themes folder.
    pub(in crate::studio) const USER: Themes = Themes {
        resolve: resolve_in_profile,
        saved_path: saved_in_profile,
        locate: locate_in_profile,
        read: read_in_profile,
        save_new: save_new_in_profile,
        overwrite: overwrite_in_profile,
        new_id: new_id_in_profile,
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

fn new_id_in_profile(name: &str) -> String {
    let taken: Vec<String> = UserThemes::read(&dir()).themes.into_iter().map(|t| t.slug).collect();
    let wanted = slug::derive_id(name);
    std::iter::once(wanted.clone())
        .chain((2..).map(|n| format!("{wanted}_{n}")))
        .find(|id| !slug::is_reserved(id) && !taken.contains(id))
        .unwrap_or(wanted)
}

/// Keep the file `path` is, as a saved version, before it is replaced. A version that can't be
/// kept is logged and does not stop the save: the person's edit matters more than the history.
fn keep_old(themes: &Path, path: &Path, file: &ThemeFile) {
    let Ok(old) = files::read_to_string_capped(path, MAX_THEME_BYTES as u64, LinkPolicy::Follow) else {
        return;
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
}

fn save_new_in_profile(file: &ThemeFile) -> Result<Saved, String> {
    let dir = dir();
    // A theme of the same name is replaced in its own file: the old one is kept first.
    if let Some(existing) = file.name.as_deref().and_then(|n| Config::saved_theme_path_in(&dir, n)) {
        keep_old(&dir, &existing, file);
    }
    let path = Config::save_theme_to(&dir, file)?;
    Ok(Saved { stamp: Stamp::of(&path), path })
}

fn overwrite_in_profile(path: &Path, file: &ThemeFile, expected: Option<Stamp>) -> Result<Saved, SaveError> {
    if expected.is_some() && Stamp::of(path) != expected {
        return Err(SaveError::ChangedOnDisk);
    }
    keep_old(&dir(), path, file);
    files::write_atomic(path, file.to_json_pretty().as_bytes(), LinkPolicy::Follow)
        .map_err(|e| SaveError::Other(e.to_string()))?;
    Ok(Saved { path: path.to_path_buf(), stamp: Stamp::of(path) })
}
