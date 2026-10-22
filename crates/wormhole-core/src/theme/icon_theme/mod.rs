//! File icon themes in Zed's icon theme format (schema v0.3.0), so existing
//! Zed icon themes can be dropped in unchanged.
//!
//! Lookup follows Zed's `FileIcons::get_icon`: the full file name and then
//! every suffix after a `.` (longest first) is tried against `file_stems`, then
//! `file_suffixes`; the first candidate whose icon type exists in
//! `file_icons` wins, else `file_icons["default"]`. Zed matches
//! case-sensitively; we try each candidate exactly first and then
//! case-insensitively before moving on to the next, shorter one, so
//! `readme.md` gets the readme icon and `photo.PNG` the image icon.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use serde::Deserialize;

use self::format::{FamilyContent, PairContent, ThemeContent};
pub use self::load::{discover, load_family};
use crate::storage::jsonc;

mod builtin;
mod format;
mod load;
#[cfg(test)]
mod tests;

/// Asset keys of the bundled theme start with this.
pub const BUILTIN_PREFIX: &str = "wormhole-icons/";

/// Where an icon's SVG comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconSource {
    /// Key into [`builtin_asset`], e.g. `wormhole-icons/dark/rust.svg`.
    Builtin(&'static str),
    /// Absolute path of an SVG inside a user theme's folder.
    File(PathBuf),
}

/// What the lookups return; the same type as [`IconSource`].
#[deprecated(note = "use `IconSource`")]
pub type IconRef = IconSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
}

/// How the app should draw the icons: `Color` through `img()` (resvg, keeps
/// the SVG's colors), `Mono` through `svg()` (alpha mask tinted with the text
/// color). Set with `"x-wormhole": { "render": "mono" }`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderMode {
    #[default]
    Color,
    Mono,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconPair {
    pub collapsed: Option<IconSource>,
    pub expanded: Option<IconSource>,
}

impl IconPair {
    fn get(&self, expanded: bool) -> Option<&IconSource> {
        if expanded { self.expanded.as_ref() } else { self.collapsed.as_ref() }
    }
}

#[derive(Debug, Clone)]
pub struct IconThemeFamily {
    pub name: String,
    pub author: String,
    pub render: RenderMode,
    /// Shared, so handing a theme to the app (or cloning the family) copies
    /// no lookup tables.
    pub themes: Vec<Arc<IconTheme>>,
}

#[derive(Debug, Clone)]
pub struct IconTheme {
    pub name: String,
    pub appearance: Appearance,
    pub render: RenderMode,
    directory_icons: IconPair,
    chevron_icons: IconPair,
    named_directory_icons: HashMap<String, IconPair>,
    file_stems: HashMap<String, String>,
    file_suffixes: HashMap<String, String>,
    file_icons: HashMap<String, IconSource>,
    // Lowercased copies for the case-insensitive second pass.
    lower_named_directories: HashMap<String, IconPair>,
    lower_stems: HashMap<String, String>,
    lower_suffixes: HashMap<String, String>,
}

// ---- lookups ----

impl IconTheme {
    /// Icon for a file. `path` may be a bare name or a `/`-separated
    /// repo-relative path (only the last component is used).
    pub fn file_icon(&self, path: &str) -> Option<&IconSource> {
        let name = path.trim_end_matches('/').rsplit('/').next().unwrap_or(path);
        if !name.is_empty() {
            for c in candidates(name) {
                let found = self
                    .icon_for(c, &self.file_stems, &self.file_suffixes)
                    .or_else(|| self.icon_for(&c.to_lowercase(), &self.lower_stems, &self.lower_suffixes));
                if found.is_some() {
                    return found;
                }
            }
        }
        self.file_icons.get("default")
    }

    /// Icon for a directory: a named icon (exact, then case-insensitive),
    /// else the generic folder icon.
    pub fn directory_icon(&self, name: &str, expanded: bool) -> Option<&IconSource> {
        let name = name.trim().trim_end_matches('/');
        let name = name.rsplit('/').next().unwrap_or(name);
        let named = self
            .named_directory_icons
            .get(name)
            .or_else(|| self.lower_named_directories.get(&name.to_lowercase()))
            .and_then(|pair| pair.get(expanded));
        named.or_else(|| self.directory_icons.get(expanded))
    }

    pub fn chevron(&self, expanded: bool) -> Option<&IconSource> {
        self.chevron_icons.get(expanded)
    }

    /// The icon registered for a type key (`"rust"`, `"default"`, …).
    pub fn icon_for_type(&self, key: &str) -> Option<&IconSource> {
        self.file_icons.get(key)
    }

    /// Every icon the theme references (for preloading).
    pub fn all_icons(&self) -> impl Iterator<Item = &IconSource> {
        let pairs = std::iter::once(&self.directory_icons)
            .chain(std::iter::once(&self.chevron_icons))
            .chain(self.named_directory_icons.values());
        pairs.flat_map(|p| p.collapsed.iter().chain(p.expanded.iter())).chain(self.file_icons.values())
    }

    fn icon_for(
        &self,
        candidate: &str,
        stems: &HashMap<String, String>,
        suffixes: &HashMap<String, String>,
    ) -> Option<&IconSource> {
        // Like Zed, a candidate only counts if its type has an icon;
        // otherwise the next (shorter) suffix gets a chance.
        let key = stems.get(candidate).or_else(|| suffixes.get(candidate))?;
        self.file_icons.get(key)
    }
}

/// The full name, then everything after each `.`, longest first:
/// `foo.d.ts` → `foo.d.ts`, `d.ts`, `ts`; `.gitignore` → `.gitignore`, `gitignore`.
/// This covers every candidate Zed tries (name, dotted suffixes, multi-part
/// extension, extension or hidden name) in the same order of priority.
fn candidates(name: &str) -> Vec<&str> {
    let mut out = vec![name];
    out.extend(name.match_indices('.').map(|(i, _)| &name[i + 1..]).filter(|s| !s.is_empty()));
    out
}

/// First theme with the preferred appearance, else the family's first theme.
pub fn pick_theme(family: &IconThemeFamily, prefer_light: bool) -> Option<&IconTheme> {
    pick(family, prefer_light).map(|t| &**t)
}

/// [`pick_theme`], as a shared handle (no copy of the theme).
pub fn pick_shared(family: &IconThemeFamily, prefer_light: bool) -> Option<Arc<IconTheme>> {
    pick(family, prefer_light).cloned()
}

fn pick(family: &IconThemeFamily, prefer_light: bool) -> Option<&Arc<IconTheme>> {
    let want = if prefer_light { Appearance::Light } else { Appearance::Dark };
    family.themes.iter().find(|t| t.appearance == want).or_else(|| family.themes.first())
}

/// The icon theme called `name` in its light or dark variant: the bundled
/// one, or the user's family of that name in `user_dir` (the data dir's
/// `icon-themes`). A missing or broken user theme falls back to the bundled
/// one. Discovery and parsing are cached, so this is cheap to call on every
/// settings change.
pub fn theme_named(user_dir: &Path, name: &str, prefer_light: bool) -> Option<Arc<IconTheme>> {
    let builtin = builtin_family();
    let loaded = (name != builtin.name)
        .then(|| discover(user_dir).into_iter().find(|(n, _)| n == name))
        .flatten()
        .and_then(|(_, path)| {
            load_family(&path).inspect_err(|err| tracing::warn!("icon theme {name:?}: {err}")).ok()
        });
    pick_shared(loaded.as_ref().unwrap_or(builtin), prefer_light)
}

// ---- building themes ----

fn lowercase_keys<V: Clone>(map: &HashMap<String, V>) -> HashMap<String, V> {
    // Sorted so that keys colliding after lowercasing resolve the same way
    // on every run (the smallest original key wins).
    let sorted: BTreeMap<&String, &V> = map.iter().collect();
    let mut out = HashMap::with_capacity(map.len());
    for (k, v) in sorted {
        out.entry(k.to_lowercase()).or_insert_with(|| v.clone());
    }
    out
}

fn build_family(
    content: FamilyContent,
    resolve: &dyn Fn(&str) -> Result<IconSource, String>,
    base: Option<&IconTheme>,
) -> Result<IconThemeFamily, String> {
    let render = content.wormhole.render;
    let themes = content
        .themes
        .into_iter()
        .map(|t| build_theme(t, render, resolve, base).map(Arc::new))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IconThemeFamily { name: content.name, author: content.author, render, themes })
}

fn build_theme(
    t: ThemeContent,
    render: RenderMode,
    resolve: &dyn Fn(&str) -> Result<IconSource, String>,
    base: Option<&IconTheme>,
) -> Result<IconTheme, String> {
    let pair = |p: PairContent| -> Result<IconPair, String> {
        Ok(IconPair {
            collapsed: p.collapsed.as_deref().map(resolve).transpose()?,
            expanded: p.expanded.as_deref().map(resolve).transpose()?,
        })
    };
    let named_directory_icons = t
        .named_directory_icons
        .into_iter()
        .map(|(k, p)| Ok((k, pair(p)?)))
        .collect::<Result<HashMap<_, _>, String>>()?;
    let file_icons = t
        .file_icons
        .into_iter()
        .map(|(k, d)| resolve(&d.path).map(|r| (k, r)))
        .collect::<Result<HashMap<_, _>, String>>()?;
    // Zed layers every theme's associations over the default theme's, so a
    // theme that only defines `file_icons` for known type keys still works.
    let (mut file_stems, mut file_suffixes) =
        base.map(|b| (b.file_stems.clone(), b.file_suffixes.clone())).unwrap_or_default();
    file_stems.extend(t.file_stems);
    file_suffixes.extend(t.file_suffixes);
    Ok(IconTheme {
        name: t.name,
        appearance: t.appearance,
        render,
        directory_icons: pair(t.directory_icons)?,
        chevron_icons: pair(t.chevron_icons)?,
        lower_named_directories: lowercase_keys(&named_directory_icons),
        lower_stems: lowercase_keys(&file_stems),
        lower_suffixes: lowercase_keys(&file_suffixes),
        named_directory_icons,
        file_stems,
        file_suffixes,
        file_icons,
    })
}

// ---- bundled theme ----

/// Every bundled SVG as `(asset key, bytes)`, for registering an in-memory
/// GPUI asset source.
pub fn builtin_icons() -> &'static [(&'static str, &'static [u8])] {
    builtin::ICONS
}

pub fn builtin_asset(key: &str) -> Option<&'static [u8]> {
    builtin::ICONS.iter().find(|(k, _)| *k == key).map(|(_, bytes)| *bytes)
}

fn builtin_key(rel: &str) -> Option<&'static str> {
    let rel = rel.trim_start_matches("./").strip_prefix("icons/")?;
    let key = format!("{BUILTIN_PREFIX}{rel}");
    builtin::ICONS.iter().map(|(k, _)| *k).find(|k| *k == key)
}

fn parse_builtin() -> Result<IconThemeFamily, String> {
    let content: FamilyContent = jsonc::parse(builtin::THEME_JSON)?;
    let resolve = |p: &str| {
        builtin_key(p).map(IconSource::Builtin).ok_or_else(|| format!("bundled icon {p:?} is missing"))
    };
    build_family(content, &resolve, None)
}

/// "Wormhole Icons", the bundled default (dark and light). Tests guarantee
/// it parses; were it ever broken, this is an empty family rather than a panic.
pub fn builtin_family() -> &'static IconThemeFamily {
    static FAMILY: LazyLock<IconThemeFamily> = LazyLock::new(|| {
        parse_builtin().unwrap_or_else(|_| IconThemeFamily {
            name: "Wormhole Icons".into(),
            author: String::new(),
            render: RenderMode::Color,
            themes: Vec::new(),
        })
    });
    &FAMILY
}
