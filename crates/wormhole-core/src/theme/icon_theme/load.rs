//! User icon themes on disk: loading a family from its JSON (paths contained
//! in the theme's folder) and discovering the families in the icon themes dir.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

use serde::Deserialize;

use super::format::FamilyContent;
use super::{IconSource, IconThemeFamily, build_family, builtin_family, pick_theme};
use crate::storage::files::{self, LinkPolicy};
use crate::storage::jsonc;

/// Load a theme family from its JSON file, or from a folder containing one
/// (`<folder>/*.json`, or a Zed extension's `<folder>/icon_themes/*.json`).
///
/// Icon paths are relative to the JSON's folder, except for a JSON inside an
/// `icon_themes/` folder, whose paths are relative to the extension root one
/// level up (Zed's layout). Paths may not leave that root.
///
/// Loaded families are cached by the JSON's size and modification time (a
/// copy is cheap: the themes are shared), since Zed-style families run to
/// hundreds of KiB and one canonicalized path per icon.
pub fn load_family(dir_or_json_path: &Path) -> Result<IconThemeFamily, String> {
    let json = if dir_or_json_path.is_dir() {
        theme_jsons(dir_or_json_path)
            .into_iter()
            .next()
            .ok_or_else(|| format!("{}: no icon theme JSON found", dir_or_json_path.display()))?
    } else {
        dir_or_json_path.to_path_buf()
    };
    let stamp = stamp(&json);
    let mut families = FAMILIES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((cached, family)) = families.get(&json)
        && stamp.is_some()
        && *cached == stamp
    {
        return Ok(family.clone());
    }
    let family = read_family(&json)?;
    if families.len() >= MAX_CACHED_FAMILIES {
        families.clear();
    }
    families.insert(json, (stamp, family.clone()));
    Ok(family)
}

fn read_family(json: &Path) -> Result<IconThemeFamily, String> {
    #[cfg(test)]
    PARSES.with(|n| n.set(n.get() + 1));
    let text = files::read_to_string_capped(json, MAX_FAMILY_BYTES, LinkPolicy::Follow)
        .map_err(|e| format!("{}: {e}", json.display()))?;
    let content: FamilyContent = jsonc::parse(&text).map_err(|e| format!("{}: {e}", json.display()))?;
    let json_dir = json.parent().unwrap_or(Path::new("."));
    let root = if json_dir.file_name().is_some_and(|n| n == "icon_themes") {
        json_dir.parent().unwrap_or(json_dir)
    } else {
        json_dir
    };
    let root = root.canonicalize().map_err(|e| format!("{}: {e}", root.display()))?;
    let base = pick_theme(builtin_family(), false);
    build_family(content, &|p| resolve_in(&root, p), base).map_err(|e| format!("{}: {e}", json.display()))
}

/// Largest icon theme JSON read; big Zed families are a few hundred KiB.
const MAX_FAMILY_BYTES: u64 = 4 * 1024 * 1024;

/// Families kept loaded; the app shows one at a time.
const MAX_CACHED_FAMILIES: usize = 8;

/// When a file was last changed, as far as caching goes: size and mtime.
type Stamp = Option<(u64, Option<SystemTime>)>;

fn stamp(path: &Path) -> Stamp {
    std::fs::metadata(path).ok().map(|m| (m.len(), m.modified().ok()))
}

static FAMILIES: LazyLock<Mutex<HashMap<PathBuf, (Stamp, IconThemeFamily)>>> =
    LazyLock::new(Default::default);

/// Family names (with the stamp they were read at) by JSON path.
type Names = HashMap<PathBuf, (Stamp, String)>;

/// [`Names`] by user dir.
static NAMES: LazyLock<Mutex<HashMap<PathBuf, Names>>> = LazyLock::new(Default::default);

#[cfg(test)]
thread_local! {
    /// Icon theme JSON files read and parsed on this thread (cache misses).
    pub(super) static PARSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Resolve `rel` against `root`, refusing anything that ends up outside it
/// (absolute paths, `..`, or symlinks pointing elsewhere).
fn resolve_in(root: &Path, rel: &str) -> Result<IconSource, String> {
    let rel_path = Path::new(rel);
    let mut out = root.to_path_buf();
    for c in rel_path.components() {
        match c {
            Component::CurDir => {}
            Component::Normal(part) => out.push(part),
            Component::ParentDir if out != root => {
                out.pop();
            }
            _ => return Err(format!("icon path {rel:?} escapes the theme folder")),
        }
    }
    if let Ok(real) = out.canonicalize()
        && !real.starts_with(root)
    {
        return Err(format!("icon path {rel:?} escapes the theme folder"));
    }
    Ok(IconSource::File(out))
}

/// JSON files of one theme folder, sorted: `*.json` first, then Zed's
/// `icon_themes/*.json`.
fn theme_jsons(folder: &Path) -> Vec<PathBuf> {
    let list = |dir: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json"))
            .collect();
        v.sort();
        v
    };
    let mut out = list(folder);
    out.extend(list(&folder.join("icon_themes")));
    out
}

/// Scan `<user_dir>/<theme folder>/*.json` (and Zed extensions'
/// `<theme folder>/icon_themes/*.json`). `user_dir` is the `icon-themes`
/// folder in the data dir. Returns the family name from each JSON (or the file
/// stem if it doesn't parse) and the JSON path, sorted by name.
pub fn discover(user_dir: &Path) -> Vec<(String, PathBuf)> {
    #[derive(Deserialize)]
    struct NameOnly {
        name: String,
    }
    let mut folders: Vec<PathBuf> = std::fs::read_dir(user_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    let mut names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    let known = names.remove(user_dir).unwrap_or_default();
    let mut fresh = HashMap::new();
    let mut out: Vec<(String, PathBuf)> = folders
        .iter()
        .flat_map(|f| theme_jsons(f))
        .map(|json| {
            let stamp = stamp(&json);
            let name = match known.get(&json) {
                Some((cached, name)) if stamp.is_some() && *cached == stamp => name.clone(),
                _ => {
                    #[cfg(test)]
                    PARSES.with(|n| n.set(n.get() + 1));
                    files::read_to_string_capped(&json, MAX_FAMILY_BYTES, LinkPolicy::Follow)
                        .ok()
                        .and_then(|t| jsonc::parse::<NameOnly>(&t).ok())
                        .map(|n| n.name)
                        .unwrap_or_else(|| {
                            json.file_stem().unwrap_or_default().to_string_lossy().into_owned()
                        })
                }
            };
            fresh.insert(json.clone(), (stamp, name.clone()));
            (name, json)
        })
        .collect();
    // Only what the dir holds now stays cached.
    names.insert(user_dir.to_path_buf(), fresh);
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()).then_with(|| a.1.cmp(&b.1)));
    out
}
