//! Finding themes: the built-ins and the files in the user's themes dir, read
//! through one cache, so that the picker, `settings.theme`, `extends` and the
//! Theme Studio agree on what a name means.
//!
//! - A theme's display name is its JSON `name`, else its file stem (never the
//!   name of the theme it extends).
//! - A bare name selects a user theme before a built-in (so a theme saved
//!   under a built-in's name is the one shown), then a path is tried.
//! - Each file is parsed once per (length, modification time); a directory is
//!   listed once per lookup, not once per `extends` step.
//!
//! Only the legacy sources exist today (built-ins and the themes dir). The
//! extension registry adds layers behind the same seams: [`ThemeId`] for
//! identity and [`ThemeSource`] for `extends`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

use crate::fsutil::{self, LinkPolicy};
use crate::theme;
use crate::tokens::{self, ThemeFile, ThemeTokens};

/// Which theme an entry is: where it lives, since two files may share a name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThemeId {
    Builtin(&'static str),
    File(PathBuf),
}

/// Finds the theme files `extends` names (user themes; resolution falls back
/// to the built-ins itself).
pub trait ThemeSource {
    fn theme_file(&self, name: &str) -> Option<ThemeFile>;
}

/// Cheap change detector for a themes dir: file names, sizes and mtimes.
pub type Fingerprint = Vec<(PathBuf, u64, Option<SystemTime>)>;

/// One file in the themes dir.
#[derive(Clone, Debug)]
pub struct UserTheme {
    pub path: PathBuf,
    /// What the picker shows and bare names match: the JSON `name`, else the file stem.
    pub name: String,
    content: Arc<Content>,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant, reason = "one per file, behind an `Arc`")]
enum Content {
    /// A JSON token theme (or why it didn't parse).
    Json(Result<ThemeFile, String>),
    /// A Ghostty / iTerm2 / Alacritty palette.
    Palette(Result<theme::Theme, String>),
}

impl UserTheme {
    pub fn id(&self) -> ThemeId {
        ThemeId::File(self.path.clone())
    }

    /// The parsed JSON theme, if this is one that parses.
    pub fn file(&self) -> Option<&ThemeFile> {
        match &*self.content {
            Content::Json(Ok(file)) => Some(file),
            _ => None,
        }
    }

    /// Whether a bare name means this theme: its name in any case, or its
    /// file stem exactly.
    fn answers_to(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name) || self.path.file_stem().is_some_and(|s| s == name)
    }
}

/// The themes in one themes dir, as read now.
#[derive(Clone, Debug, Default)]
pub struct UserThemes {
    /// Sorted by path.
    pub themes: Vec<UserTheme>,
}

impl UserThemes {
    /// Read `dir` (a missing dir has no themes). Unchanged files come from
    /// the cache; hidden files are skipped, as everywhere else.
    pub fn read(dir: &Path) -> Self {
        let listed = fingerprint(dir);
        let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
        let old = cache.remove(dir).unwrap_or_default();
        let mut fresh = HashMap::with_capacity(listed.len());
        let mut themes = Vec::with_capacity(listed.len());
        for (path, len, modified) in listed {
            let content = match old.get(&path) {
                Some(cached) if cached.len == len && cached.modified == modified => {
                    Arc::clone(&cached.content)
                }
                _ => Arc::new(read_content(&path)),
            };
            themes.push(UserTheme {
                name: display_name(&path, &content),
                path: path.clone(),
                content: Arc::clone(&content),
            });
            fresh.insert(path, Cached { len, modified, content });
        }
        // Only what the dir holds now stays cached.
        cache.insert(dir.to_path_buf(), fresh);
        Self { themes }
    }

    /// The JSON theme `extends` reaches as `name`: the first by path that
    /// answers to it and parses.
    pub fn extends_target(&self, name: &str) -> Option<&UserTheme> {
        self.themes.iter().find(|t| t.file().is_some() && t.answers_to(name))
    }

    /// The user theme a bare `settings.theme` value selects: the first by path
    /// that answers to it, whatever its format (a broken one reports why).
    pub fn named(&self, name: &str) -> Option<&UserTheme> {
        self.themes.iter().find(|t| t.answers_to(name))
    }

    /// Which theme a bare name selects: a user theme first, then a built-in.
    pub fn select(&self, name: &str) -> Option<ThemeId> {
        self.named(name).map(UserTheme::id).or_else(|| builtin_named(name).map(|t| ThemeId::Builtin(&t.name)))
    }

    /// Resolve a theme file with these themes available to `extends`.
    pub fn resolve_file(&self, file: &ThemeFile) -> Result<ThemeTokens, String> {
        tokens::resolve(file, &|name| self.theme_file(name))
    }

    /// One of these themes, resolved. An unnamed JSON theme is named after
    /// its file, not after the theme it extends.
    pub fn resolve(&self, theme: &UserTheme) -> Result<ThemeTokens, String> {
        match &*theme.content {
            Content::Json(Ok(file)) => {
                let mut tokens = self.resolve_file(file)?;
                if file.name.as_deref().is_none_or(str::is_empty) {
                    tokens.name = theme.name.clone();
                }
                Ok(tokens)
            }
            Content::Json(Err(err)) | Content::Palette(Err(err)) => Err(err.clone()),
            Content::Palette(Ok(palette)) => Ok(tokens::derive(palette)),
        }
    }
}

impl ThemeSource for UserThemes {
    fn theme_file(&self, name: &str) -> Option<ThemeFile> {
        self.extends_target(name).and_then(UserTheme::file).cloned()
    }
}

/// The built-in theme called `name`: exactly, else in any case.
pub fn builtin_named(name: &str) -> Option<&'static ThemeTokens> {
    let all = tokens::builtins();
    all.iter().find(|t| t.name == name).or_else(|| all.iter().find(|t| t.name.eq_ignore_ascii_case(name)))
}

/// The files in `dir` that can be themes (not hidden), with their sizes and
/// mtimes, sorted. Lists and stats only; nothing is read.
pub fn fingerprint(dir: &Path) -> Fingerprint {
    let mut files: Fingerprint = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|entry| {
            let path = entry.path();
            // Follows symlinks: a theme linked in from elsewhere is still a theme.
            let meta = std::fs::metadata(&path).ok().filter(|m| m.is_file())?;
            Some((path, meta.len(), meta.modified().ok()))
        })
        .collect();
    files.sort();
    files
}

struct Cached {
    len: u64,
    modified: Option<SystemTime>,
    content: Arc<Content>,
}

/// Parsed theme files by dir, then path.
static CACHE: LazyLock<Mutex<HashMap<PathBuf, HashMap<PathBuf, Cached>>>> = LazyLock::new(Default::default);

fn read_content(path: &Path) -> Content {
    #[cfg(test)]
    PARSES.with(|n| n.set(n.get() + 1));
    let text = fsutil::read_to_string_capped(path, tokens::MAX_THEME_BYTES as u64, LinkPolicy::Follow)
        .map_err(|e| e.to_string());
    if path.extension().is_some_and(|e| e == "json") {
        Content::Json(text.and_then(|text| tokens::parse_theme_file(&text)))
    } else {
        Content::Palette(text.and_then(|text| theme::parse_file(path, &text)))
    }
}

fn display_name(path: &Path, content: &Content) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    match content {
        Content::Json(Ok(file)) => file.name.clone().filter(|n| !n.is_empty()).unwrap_or(stem),
        _ => stem,
    }
}

#[cfg(test)]
thread_local! {
    /// Theme files read and parsed on this thread (cache misses).
    static PARSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{PARSES, ThemeId, ThemeSource, UserThemes, fingerprint};

    fn write(dir: &Path, file: &str, text: &str) {
        std::fs::write(dir.join(file), text).expect("write theme");
    }

    fn parses() -> usize {
        PARSES.with(|n| n.get())
    }

    #[test]
    fn names_come_from_the_file_never_from_the_base_theme() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "a.json", r#"{ "name": "Aurora", "extends": "Nord" }"#);
        write(dir.path(), "b.json", r#"{ "extends": "Nord" }"#);
        write(dir.path(), "c.json", r#"{ "name": "", "extends": "Nord" }"#);
        write(dir.path(), "Paper", "background = #fafafa\n");
        write(dir.path(), ".hidden.json", r#"{ "name": "Hidden" }"#);
        let user = UserThemes::read(dir.path());
        let names: Vec<&str> = user.themes.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["Paper", "Aurora", "b", "c"]);
        let b = user.named("b").expect("b");
        assert_eq!(user.resolve(b).expect("resolves").name, "b");
        assert_eq!(user.resolve(user.named("Paper").expect("paper")).expect("resolves").name, "Paper");
        assert!(user.named("Hidden").is_none());
    }

    #[test]
    fn user_themes_come_before_built_ins() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(
            dir.path(),
            "mine.json",
            r##"{ "name": "Nord", "extends": "Nord", "ui": { "primary": "#ff0000" } }"##,
        );
        let user = UserThemes::read(dir.path());
        let mine = ThemeId::File(dir.path().join("mine.json"));
        assert_eq!(user.select("Nord"), Some(mine.clone()));
        assert_eq!(user.select("nord"), Some(mine.clone()));
        assert_eq!(user.select("mine"), Some(mine));
        assert_eq!(user.select("Dracula"), Some(ThemeId::Builtin("Dracula")));
        assert_eq!(user.select("dracula"), Some(ThemeId::Builtin("Dracula")));
        assert_eq!(user.select("nope"), None);
        // It may still extend the built-in it shadows.
        let tokens = user.resolve(user.named("Nord").expect("mine")).expect("resolves");
        assert_eq!(tokens.get("primary").map(|c| c.to_hex()).as_deref(), Some("#ff0000"));
    }

    #[test]
    fn extends_finds_parsed_json_themes_by_name_or_stem() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "base.json", r#"{ "name": "Base", "extends": "Dracula" }"#);
        write(dir.path(), "broken.json", "{ nope");
        write(dir.path(), "Paper", "background = #fafafa\n");
        let user = UserThemes::read(dir.path());
        assert!(user.theme_file("base").is_some());
        assert!(user.theme_file("BASE").is_some());
        assert!(user.theme_file("broken").is_none());
        assert!(user.theme_file("Paper").is_none());
        // A bare name still finds the broken file, so selecting it says what is wrong.
        let broken = user.named("broken").expect("listed");
        assert!(user.resolve(broken).expect_err("broken").contains("line 1"));
    }

    #[test]
    fn files_are_parsed_once_until_they_change() {
        let dir = tempfile::tempdir().expect("temp dir");
        for i in 0..5 {
            let extends = if i == 0 { "Nord".to_string() } else { format!("T{}", i - 1) };
            write(
                dir.path(),
                &format!("t{i}.json"),
                &format!(r#"{{ "name": "T{i}", "extends": "{extends}" }}"#),
            );
        }
        let start = parses();
        let user = UserThemes::read(dir.path());
        // The whole chain resolves from the one read.
        assert!(user.resolve(user.named("T4").expect("T4")).is_ok());
        assert_eq!(parses() - start, 5);
        let again = UserThemes::read(dir.path());
        assert!(again.resolve(again.named("T4").expect("T4")).is_ok());
        assert_eq!(parses() - start, 5);
        write(dir.path(), "t2.json", r#"{ "name": "T2", "extends": "T1", "ui": {} }"#);
        let changed = UserThemes::read(dir.path());
        assert_eq!(parses() - start, 6);
        assert_eq!(changed.themes.len(), 5);
    }

    #[test]
    fn the_fingerprint_lists_theme_files_only() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(fingerprint(&dir.path().join("missing")).is_empty());
        write(dir.path(), "a.json", "{}");
        write(dir.path(), ".DS_Store", "x");
        std::fs::create_dir(dir.path().join("folder")).expect("mkdir");
        let files: Vec<_> = fingerprint(dir.path()).into_iter().map(|(p, len, _)| (p, len)).collect();
        assert_eq!(files, [(dir.path().join("a.json"), 2)]);
    }
}
