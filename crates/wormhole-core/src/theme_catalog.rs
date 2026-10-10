//! Every theme the user can pick, resolved to tokens: the built-ins plus the
//! files in the themes dir. The settings theme picker draws its preview cards
//! from this, so it is built once and only rebuilt when the themes dir changes
//! (see [`fingerprint`]).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::config::Config;
use crate::theme_registry::{self, ThemeId, UserThemes};
use crate::tokens::{self, ThemeTokens};

#[derive(Clone, Debug)]
pub struct ThemeEntry {
    /// What the picker shows.
    pub label: String,
    /// What goes into `settings.theme` to select it (name or file path).
    pub value: String,
    pub tokens: ThemeTokens,
    pub builtin: bool,
    /// Which theme this is (labels can repeat).
    pub id: ThemeId,
    /// The themes this catalog was built from, to tell what a name selects.
    themes: Arc<UserThemes>,
}

impl ThemeEntry {
    /// Whether `settings.theme` (a name, possibly in another case, or a path)
    /// selects this entry: the one entry a name resolves to, so a user theme
    /// that shadows a built-in's name is the only one shown as active.
    pub fn is_selected_by(&self, setting: &str) -> bool {
        let by_path = !self.builtin && self.value == setting;
        by_path || self.themes.select(setting).as_ref() == Some(&self.id)
    }
}

/// Built-ins and the user's themes dir.
pub fn catalog() -> Vec<ThemeEntry> {
    catalog_in(&Config::themes_dir())
}

/// Built-ins, then the themes in `dir` sorted by name, each resolved with the
/// themes in `dir` available to `extends`. Files that don't resolve are left
/// out (and logged): a broken import shouldn't hide the rest.
pub fn catalog_in(dir: &Path) -> Vec<ThemeEntry> {
    let themes = Arc::new(UserThemes::read(dir));
    let mut entries: Vec<ThemeEntry> = tokens::builtins()
        .iter()
        .map(|tokens| ThemeEntry {
            label: tokens.name.clone(),
            value: tokens.name.clone(),
            tokens: tokens.clone(),
            builtin: true,
            id: ThemeId::Builtin(&tokens.name),
            themes: Arc::clone(&themes),
        })
        .collect();
    let mut user: Vec<ThemeEntry> = themes
        .themes
        .iter()
        .filter_map(|theme| match themes.resolve(theme) {
            Ok(tokens) => Some(ThemeEntry {
                label: theme.name.clone(),
                value: theme.path.to_string_lossy().into_owned(),
                tokens,
                builtin: false,
                id: theme.id(),
                themes: Arc::clone(&themes),
            }),
            Err(err) => {
                tracing::warn!("theme {}: {err}", theme.path.display());
                None
            }
        })
        .collect();
    user.sort_by_key(|e| e.label.to_lowercase());
    entries.extend(user);
    entries
}

/// Cheap change detector for the themes dir: file names, sizes and mtimes.
pub fn fingerprint() -> Vec<(PathBuf, u64, Option<SystemTime>)> {
    fingerprint_in(&Config::themes_dir())
}

pub fn fingerprint_in(dir: &Path) -> Vec<(PathBuf, u64, Option<SystemTime>)> {
    theme_registry::fingerprint(dir)
}

/// A theme file: JSON token theme, or a Ghostty / iTerm2 / Alacritty palette.
/// Resolved the way `config` resolves a theme given by path.
pub fn load_path(path: &Path) -> Result<ThemeTokens, String> {
    crate::config::load_theme_path(&Config::themes_dir(), path)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{catalog_in, fingerprint_in};
    use crate::tokens::{self, Appearance};

    const GHOSTTY: &str = "background = #fafafa\nforeground = #202020\npalette = 1=#cc0000\n";

    fn write(dir: &Path, file: &str, text: &str) {
        std::fs::write(dir.join(file), text).expect("write");
    }

    #[test]
    fn lists_builtins_first_then_user_themes_by_name() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "Paper", GHOSTTY);
        write(dir.path(), "b.json", r#"{ "name": "Aurora", "extends": "Nord" }"#);
        write(dir.path(), "broken.json", "{ nope");

        let entries = catalog_in(dir.path());
        let builtins = tokens::builtins().len();
        assert!(entries[..builtins].iter().all(|e| e.builtin));
        let user: Vec<&str> = entries[builtins..].iter().map(|e| e.label.as_str()).collect();
        assert_eq!(user, ["Aurora", "Paper"]);
        let paper = &entries[builtins + 1];
        assert_eq!(paper.tokens.appearance, Appearance::Light);
        assert!(paper.value.ends_with("Paper"));
    }

    #[test]
    fn selection_matches_name_case_insensitively_or_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let entry = catalog_in(dir.path()).remove(0);
        assert!(entry.is_selected_by(&entry.label.to_uppercase()));
        assert!(!entry.is_selected_by("something else"));
    }

    #[test]
    fn an_unnamed_theme_is_labelled_by_its_file_and_one_card_is_active() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "b.json", r##"{ "extends": "Nord", "ui": { "primary": "#ff0000" } }"##);
        let entries = catalog_in(dir.path());
        let b = entries.iter().find(|e| !e.builtin).expect("user theme");
        assert_eq!((b.label.as_str(), b.tokens.name.as_str()), ("b", "b"));
        let active = |setting: &str| -> Vec<&str> {
            entries.iter().filter(|e| e.is_selected_by(setting)).map(|e| e.label.as_str()).collect()
        };
        assert_eq!(active("Nord"), ["Nord"]);
        assert_eq!(active("b"), ["b"]);
        assert_eq!(active(&b.value), ["b"]);
    }

    #[test]
    fn a_user_theme_named_like_a_built_in_is_the_one_selected() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(
            dir.path(),
            "dracula.json",
            r##"{ "name": "Dracula", "extends": "Dracula", "ui": { "primary": "#00ff00" } }"##,
        );
        let entries = catalog_in(dir.path());
        let active: Vec<(bool, &str)> = entries
            .iter()
            .filter(|e| e.is_selected_by("Dracula"))
            .map(|e| (e.builtin, e.label.as_str()))
            .collect();
        assert_eq!(active, [(false, "Dracula")]);
    }

    #[test]
    fn extends_resolves_within_the_dir() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(
            dir.path(),
            "base.json",
            r##"{ "name": "Base", "extends": "Nord", "ui": { "primary": "#123456" } }"##,
        );
        write(dir.path(), "kid.json", r#"{ "name": "Kid", "extends": "Base" }"#);
        let entries = catalog_in(dir.path());
        let kid = entries.iter().find(|e| e.label == "Kid").expect("kid resolves");
        assert_eq!(kid.tokens.get("primary").map(|c| c.to_hex()).as_deref(), Some("#123456"));
    }

    #[test]
    fn fingerprint_changes_when_a_file_is_added() {
        let dir = tempfile::tempdir().expect("temp dir");
        let before = fingerprint_in(dir.path());
        write(dir.path(), "Paper", GHOSTTY);
        assert_ne!(before, fingerprint_in(dir.path()));
    }

    /// `cargo test -p wormhole-core --lib catalog_timing -- --ignored --nocapture`
    /// The slow case: one chain of user themes, each extending the last.
    #[test]
    #[ignore = "timing, run by hand"]
    fn catalog_timing() {
        let _ = tokens::builtins();
        for count in [10, 40] {
            let dir = tempfile::tempdir().expect("temp dir");
            for i in 0..count {
                let extends = if i == 0 { "Nord".to_string() } else { format!("T{}", i - 1) };
                let text = format!(r#"{{ "name": "T{i}", "extends": "{extends}" }}"#);
                write(dir.path(), &format!("t{i}.json"), &text);
            }
            let start = std::time::Instant::now();
            let entries = catalog_in(dir.path());
            let cold = start.elapsed();
            let start = std::time::Instant::now();
            let _ = catalog_in(dir.path());
            let warm = start.elapsed();
            let listed = entries.len() - tokens::builtins().len();
            eprintln!("{count} chained themes ({listed} listed): {cold:?} cold, {warm:?} with parsed files");
        }
    }
}
