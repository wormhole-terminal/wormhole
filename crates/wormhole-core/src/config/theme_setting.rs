//! Themes as settings see them: resolving `settings.theme` (a user theme's
//! or a built-in's name, or a path) to tokens, importing and saving theme
//! files. Finding themes is `theme::registry`'s job.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{Config, Settings, ThemeMode};
use crate::platform::paths;
use crate::storage::files::{self, LinkPolicy};
use crate::theme;
use crate::theme::registry::{self, DEFAULT_THEME, ThemeSource, UserTheme, UserThemes, load_theme_path};
use crate::theme::tokens::{self, Appearance, ThemeFile, ThemeTokens};

impl Config {
    /// Themes in the themes dir (JSON token themes and imported terminal
    /// themes), as (display name, path).
    pub fn imported_themes() -> Vec<(String, PathBuf)> {
        let mut themes: Vec<(String, PathBuf)> =
            UserThemes::read(&Self::themes_dir()).themes.into_iter().map(|t| (t.name, t.path)).collect();
        themes.sort();
        themes
    }

    /// Copy a theme file into the themes dir and select it. A different file
    /// already there under the same name is kept: the copy gets a new name.
    pub fn import_theme(&mut self, source: &Path) -> Result<(), String> {
        load_theme_path(&self.themes, source)?;
        let target = import_into(&self.themes, source)?;
        let value = target.to_string_lossy().into_owned();
        self.update(|s| s.theme = value);
        Ok(())
    }

    /// Save a JSON theme (e.g. from the Theme Studio) into the themes dir.
    /// A draft that extends the theme of its own name is folded into it, a
    /// theme of the same name is replaced, and another theme's file is never
    /// overwritten (see `save_theme_in`).
    pub fn save_theme(file: &ThemeFile) -> Result<PathBuf, String> {
        save_theme_in(&Self::themes_dir(), file)
    }

    /// The file a theme saved as `name` replaces, if the themes dir already has
    /// a JSON theme by that name (so the caller can ask first).
    pub fn saved_theme_path(name: &str) -> Option<PathBuf> {
        Self::saved_theme_path_in(&Self::themes_dir(), name)
    }

    /// [`save_theme`](Self::save_theme) into a profile's themes folder.
    pub fn save_theme_to(themes: &Path, file: &ThemeFile) -> Result<PathBuf, String> {
        save_theme_in(themes, file)
    }

    /// [`saved_theme_path`](Self::saved_theme_path) in a profile's themes folder.
    pub fn saved_theme_path_in(themes: &Path, name: &str) -> Option<PathBuf> {
        named_in(&UserThemes::read(themes), name)
    }
}

impl Settings {
    /// The theme a window shows: the dark pick (`theme`), the light pick (`theme_light`), or
    /// under Match system the one for the system's `appearance`.
    pub fn shown_theme(&self, appearance: Appearance) -> &str {
        let light = match self.theme_mode {
            ThemeMode::Dark => false,
            ThemeMode::Light => true,
            ThemeMode::System => appearance == Appearance::Light,
        };
        if light { &self.theme_light } else { &self.theme }
    }

    /// Whether the theme picker leaves `value` (a theme as `theme` names it) out.
    pub fn theme_hidden(&self, value: &str) -> bool {
        self.hidden_themes.iter().any(|hidden| hidden == value)
    }

    /// Leave `value` out of the theme picker, or bring it back. Hidden once, however often it is
    /// hidden; true when that changed anything.
    pub fn set_theme_hidden(&mut self, value: &str, hidden: bool) -> bool {
        if self.theme_hidden(value) == hidden {
            return false;
        }
        if hidden {
            self.hidden_themes.push(value.to_string());
        } else {
            self.hidden_themes.retain(|v| v != value);
        }
        true
    }
}

/// The JSON theme in `themes` whose name is `name` (any case). A file that
/// doesn't parse as a theme is not one: saving never replaces it.
fn named_in(themes: &UserThemes, name: &str) -> Option<PathBuf> {
    named_json(themes, name).map(|t| t.path.clone())
}

fn named_json<'a>(themes: &'a UserThemes, name: &str) -> Option<&'a UserTheme> {
    themes.themes.iter().find(|t| t.file().is_some() && t.name.eq_ignore_ascii_case(name))
}

/// Save `file` over the version `theme` of a file that ships several: into that version's
/// entry, leaving the file's shared layers and its other versions as they are. A draft that
/// extends the version itself is folded into the version's own layers.
fn save_into_variant(theme: &UserTheme, short: &str, file: &ThemeFile) -> Result<PathBuf, String> {
    let mut source = theme.source().cloned().ok_or("not a theme file")?;
    let entry = source
        .variants
        .iter_mut()
        .find(|v| v.name == short)
        .ok_or_else(|| format!("{}: no variant {short:?}", theme.path.display()))?;
    let extends_itself = file.extends.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(&theme.name));
    let layers = if extends_itself { file.layered_over(&entry.to_file()) } else { file.clone() };
    entry.set_layers(&layers);
    files::write_atomic(&theme.path, source.to_json_pretty().as_bytes(), LinkPolicy::Follow)
        .map_err(|e| e.to_string())?;
    Ok(theme.path.clone())
}

/// Save `file` into `dir`, and return where it went.
///
/// A draft that `extends` the user theme of its own name (the Studio's "save
/// over my theme") is folded into that theme first: that theme's layers go
/// under the draft's and the result extends what that theme extended. Saved
/// as is, it would extend itself, and that theme's own layers would be lost.
///
/// A JSON theme by the same name is replaced in its own file, so a name stays
/// one theme. Otherwise the theme gets a new file named after it; a file that
/// holds something else is never overwritten (`-2`, `-3`, … is added instead).
fn save_theme_in(dir: &Path, file: &ThemeFile) -> Result<PathBuf, String> {
    let themes = UserThemes::read(dir);
    let name = file.name.clone().unwrap_or_else(|| "custom".into());
    if let Some(theme) = named_json(&themes, &name)
        && let Some((_, short)) = &theme.family
    {
        return save_into_variant(theme, short, file);
    }
    let extends_itself = file.extends.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(&name));
    let file = match extends_itself.then(|| themes.theme_file(&name)).flatten() {
        Some(parent) => file.layered_over(&parent),
        None => file.clone(),
    };
    let path = named_in(&themes, &name).unwrap_or_else(|| free_path(dir, &file_stem_for(&name), "json"));
    files::write_atomic(&path, file.to_json_pretty().as_bytes(), LinkPolicy::Follow)
        .map_err(|e| e.to_string())?;
    Ok(path)
}

/// Copy `source` into `dir` under its own name, or a free variant of it when
/// a different file has that name. Re-importing the same file reuses it.
fn import_into(dir: &Path, source: &Path) -> Result<PathBuf, String> {
    let bytes = files::read_capped(source, tokens::MAX_THEME_BYTES as u64, LinkPolicy::Follow)
        .map_err(|e| e.to_string())?;
    let file_name = source.file_name().ok_or("not a file")?;
    let same = dir.join(file_name);
    // Capped like the source: a file of that name that is bigger cannot be the same.
    let same_bytes = files::read_capped(&same, tokens::MAX_THEME_BYTES as u64, LinkPolicy::Follow)
        .is_ok_and(|existing| existing == bytes);
    let target = if same_bytes {
        same
    } else {
        let stem = Path::new(file_name).file_stem().map(|s| s.to_string_lossy().into_owned());
        let ext = Path::new(file_name).extension().map(|e| e.to_string_lossy().into_owned());
        free_path(dir, stem.as_deref().unwrap_or("theme"), ext.as_deref().unwrap_or(""))
    };
    files::write_atomic(&target, &bytes, LinkPolicy::Follow).map_err(|e| e.to_string())?;
    Ok(target)
}

/// A file stem for a theme name: letters, digits and `-`; never empty, and
/// never a hidden file (which the picker skips).
fn file_stem_for(name: &str) -> String {
    let stem: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' }).collect();
    let stem = stem.trim_matches('-');
    if stem.is_empty() { "theme".into() } else { stem.into() }
}

/// `dir/<stem>.<ext>`, or `dir/<stem>-2.<ext>` and so on if that is taken.
fn free_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let with = |stem: String| dir.join(if ext.is_empty() { stem } else { format!("{stem}.{ext}") });
    let first = with(stem.to_string());
    if !first.exists() {
        return first;
    }
    (2..).map(|n| with(format!("{stem}-{n}"))).find(|p| !p.exists()).unwrap_or(first)
}

/// Resolve a theme file (e.g. the Theme Studio's draft), with user themes
/// available for `extends`.
pub fn resolve_theme_file(file: &ThemeFile) -> Result<ThemeTokens, String> {
    resolve_theme_file_in(&Config::themes_dir(), file)
}

/// [`resolve_theme_file`] with the themes of a profile (`ProfilePaths::themes_dir`).
pub fn resolve_theme_file_in(themes: &Path, file: &ThemeFile) -> Result<ThemeTokens, String> {
    UserThemes::read(themes).resolve_file(file)
}

/// Resolve a theme given as JSON text (e.g. a project's `.wormhole/theme.json`).
pub fn tokens_from_json(text: &str) -> Result<ThemeTokens, String> {
    tokens_from_json_in(&Config::themes_dir(), text)
}

/// [`tokens_from_json`] with the themes of a profile.
pub fn tokens_from_json_in(themes: &Path, text: &str) -> Result<ThemeTokens, String> {
    resolve_theme_file_in(themes, &tokens::parse_theme_file(text)?)
}

/// The default theme's tokens, what is shown when the chosen theme fails.
pub(super) fn default_tokens() -> ThemeTokens {
    registry::builtin_named(DEFAULT_THEME).cloned().unwrap_or_else(|| tokens::derive(&theme::builtins()[0]))
}

/// `settings.theme` resolved against the themes in `dir`: a user theme's name,
/// a built-in's, or a path, in that order. Falls back to the default with an
/// error.
pub(super) fn resolve_theme(dir: &Path, name: &str) -> (ThemeTokens, Option<String>) {
    let themes = UserThemes::read(dir);
    if let Some(theme) = themes.named(name) {
        return match themes.resolve(theme) {
            Ok(tokens) => (tokens, None),
            Err(err) => (default_tokens(), Some(labelled(name, err))),
        };
    }
    if let Some(tokens) = registry::builtin_named(name) {
        return (tokens.clone(), None);
    }
    let path = PathBuf::from(expand_home(name));
    if path.exists() {
        return match load_theme_path(dir, &path) {
            Ok(tokens) => (tokens, None),
            Err(err) => (default_tokens(), Some(labelled(name, err))),
        };
    }
    (default_tokens(), Some(format!("unknown theme {name:?}")))
}

/// `theme "<name>": <err>`, unless the error already starts with that label.
fn labelled(name: &str, err: String) -> String {
    let label = format!("theme {name:?}");
    if err.starts_with(&label) { err } else { format!("{label}: {err}") }
}

/// What resolving `settings.theme` depends on besides the program: the files
/// in the themes dir (any `extends` may name one) and, for a theme given as a
/// path, that file. The same stamp means resolving again gives the same tokens.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ThemeStamp {
    name: String,
    files: registry::Fingerprint,
    file: Option<(u64, Option<SystemTime>)>,
}

impl ThemeStamp {
    pub(super) fn of(dir: &Path, name: &str) -> Self {
        let file = std::fs::metadata(expand_home(name)).ok().map(|m| (m.len(), m.modified().ok()));
        Self { name: name.to_string(), files: registry::fingerprint(dir), file }
    }
}

fn expand_home(path: &str) -> String {
    match (path.strip_prefix("~/"), paths::home()) {
        (Some(rest), Some(home)) => format!("{}/{rest}", home.display()),
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    /// A theme is hidden once however often it is hidden, and shown again by its name.
    #[test]
    fn a_hidden_theme_is_listed_once_and_comes_back() {
        let mut s = super::Settings::default();
        assert!(s.set_theme_hidden("Nord", true));
        assert!(!s.set_theme_hidden("Nord", true), "already hidden");
        assert!(s.set_theme_hidden("Dracula", true));
        assert_eq!(s.hidden_themes, ["Nord", "Dracula"]);
        assert!(s.theme_hidden("Nord"));
        assert!(s.set_theme_hidden("Nord", false));
        assert!(!s.set_theme_hidden("Nord", false), "already shown");
        assert_eq!(s.hidden_themes, ["Dracula"]);
    }

    use std::path::Path;

    use super::{import_into, resolve_theme, save_theme_in};
    use crate::theme::registry::UserThemes;
    use crate::theme::registry::{DEFAULT_THEME, DEFAULT_THEME_LIGHT};
    use crate::theme::tokens::{self, Appearance, ThemeFile, ThemeTokens};

    fn write(dir: &Path, file: &str, text: &str) {
        std::fs::write(dir.join(file), text).expect("write theme");
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("read dir")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn resolve_in(dir: &Path, file: &ThemeFile) -> ThemeTokens {
        UserThemes::read(dir).resolve_file(file).expect("resolves")
    }

    fn saved(path: &Path) -> ThemeFile {
        tokens::parse_theme_file(&std::fs::read_to_string(path).expect("read")).expect("parses")
    }

    #[test]
    fn saving_over_the_base_folds_it_in() {
        let dir = tempfile::tempdir().expect("temp dir");
        let base =
            r##"{"name": "Foo", "extends": "Nord", "ui": {"primary": "#ff0000"}, "metrics": {"radius": 9}}"##;
        // Stored under another stem, as an import or a hand-made file would be.
        write(dir.path(), "my foo.json", base);
        let mut draft: ThemeFile = tokens::parse_theme_file(
            r##"{"name": "Foo Custom", "extends": "Foo", "ui": {"border": "#00ff00"}}"##,
        )
        .expect("draft");
        let previewed = resolve_in(dir.path(), &draft);

        // The user names the draft after the theme it was made from and saves.
        draft.name = Some("Foo".into());
        let path = save_theme_in(dir.path(), &draft).expect("saves");
        assert_eq!(path, dir.path().join("my foo.json"));
        assert_eq!(files(dir.path()), ["my foo.json"]);
        let file = saved(&path);
        assert_eq!(file.extends.as_deref(), Some("Nord"));
        assert_eq!(file.ui.keys().collect::<Vec<_>>(), ["border", "primary"]);
        assert_eq!(file.metrics.radius, Some(9.0));
        let mut resolved = resolve_in(dir.path(), &file);
        resolved.name = previewed.name.clone();
        assert_eq!(resolved, previewed);
    }

    #[test]
    fn a_user_theme_named_like_a_built_in_may_still_extend_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let nord =
            ThemeFile { name: Some("Nord".into()), extends: Some("Nord".into()), ..Default::default() };
        let path = save_theme_in(dir.path(), &nord).expect("saves");
        assert_eq!(saved(&path), nord);
        assert_eq!(
            resolve_in(dir.path(), &saved(&path)).terminal,
            resolve_theme(dir.path(), "Nord").0.terminal
        );
    }

    #[test]
    fn saving_never_overwrites_another_theme() {
        let dir = tempfile::tempdir().expect("temp dir");
        let theme = |name: &str, primary: &str| {
            let mut file = ThemeFile { name: Some(name.into()), ..Default::default() };
            file.ui.insert("primary".into(), primary.into());
            file
        };
        let first = save_theme_in(dir.path(), &theme("My Theme", "#111111")).expect("saves");
        assert_eq!(first, dir.path().join("My-Theme.json"));
        // Another theme whose name maps to the same file name gets its own.
        let other = save_theme_in(dir.path(), &theme("My-Theme", "#222222")).expect("saves");
        assert_eq!(other, dir.path().join("My-Theme-2.json"));
        // Saving a theme again replaces it, wherever it is.
        let again = save_theme_in(dir.path(), &theme("my theme", "#333333")).expect("saves");
        assert_eq!(again, first);
        assert_eq!(saved(&first).ui["primary"], "#333333");
        assert_eq!(saved(&other).ui["primary"], "#222222");
        // A file of another kind is not overwritten either.
        write(dir.path(), "Paper.json", "not a theme");
        let paper = save_theme_in(dir.path(), &theme("Paper", "#444444")).expect("saves");
        assert_eq!(paper, dir.path().join("Paper-2.json"));
        // Names that leave nothing for a file name don't become hidden files.
        let odd = save_theme_in(dir.path(), &theme("!!!", "#555555")).expect("saves");
        assert_eq!(odd, dir.path().join("theme.json"));
    }

    const FAMILY: &str = r##"{
      "name": "Dusk",
      "extends": "Nord",
      "ui": { "primary": "#c08a1e" },
      "variants": [
        { "name": "Dark", "metrics": { "radius": 3 } },
        { "name": "Light", "appearance": "light", "terminal": { "background": "#f6f2ea", "foreground": "#2b2a35" } }
      ]
    }"##;

    #[test]
    fn saving_over_a_version_changes_only_its_entry() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "dusk.json", FAMILY);
        // The Studio's draft of "Dusk Light", named after it and saved.
        let mut draft = tokens::parse_theme_file(
            r##"{"name": "Dusk Light Custom", "extends": "Dusk Light", "ui": {"border": "#00ff00"}}"##,
        )
        .expect("draft");
        let previewed = resolve_in(dir.path(), &draft);
        draft.name = Some("Dusk Light".into());
        let path = save_theme_in(dir.path(), &draft).expect("saves");
        assert_eq!((path, files(dir.path())), (dir.path().join("dusk.json"), vec!["dusk.json".to_string()]));
        let file = saved(&dir.path().join("dusk.json"));
        assert_eq!((file.name.as_deref(), file.extends.as_deref()), (Some("Dusk"), Some("Nord")));
        assert_eq!(file.ui.keys().collect::<Vec<_>>(), ["primary"], "the shared layers stay");
        let [dark, light] = &file.variants[..] else { panic!("two variants: {:?}", file.variants) };
        assert_eq!((dark.name.as_str(), dark.metrics.radius), ("Dark", Some(3.0)));
        assert!(dark.ui.is_empty(), "the other version is untouched");
        assert_eq!(light.name, "Light");
        assert_eq!(light.ui.get("border").map(String::as_str), Some("#00ff00"));
        assert_eq!(light.terminal.background.as_deref(), Some("#f6f2ea"), "its own layers stay");
        let (mut shown, error) = resolve_theme(dir.path(), "Dusk Light");
        assert!(error.is_none(), "{error:?}");
        shown.name = previewed.name.clone();
        assert_eq!(shown, previewed);
    }

    #[test]
    fn a_family_name_shows_its_first_version() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "dusk.json", FAMILY);
        for name in ["Dusk", "dusk", "Dusk Dark"] {
            let (tokens, error) = resolve_theme(dir.path(), name);
            assert_eq!((tokens.name.as_str(), error), ("Dusk Dark", None), "{name}");
        }
        assert_eq!(resolve_theme(dir.path(), "dusk light").0.appearance, tokens::Appearance::Light);
        // Wormhole's own families too.
        assert_eq!(resolve_theme(dir.path(), "Soltari").0.name, "Soltari Light");
        assert_eq!(resolve_theme(dir.path(), "Crimson").0.name, "Crimson Dark");
        assert_eq!(resolve_theme(dir.path(), "Crimson Light").0.appearance, tokens::Appearance::Light);
    }

    #[test]
    fn importing_keeps_a_different_file_of_the_same_name() {
        let themes = tempfile::tempdir().expect("temp dir");
        let downloads = tempfile::tempdir().expect("temp dir");
        write(downloads.path(), "Paper", "background = #fafafa\n");
        let first = import_into(themes.path(), &downloads.path().join("Paper")).expect("imports");
        assert_eq!(first, themes.path().join("Paper"));
        // The same file again is the same theme.
        assert_eq!(import_into(themes.path(), &downloads.path().join("Paper")).expect("imports"), first);
        write(downloads.path(), "Paper", "background = #eeeeee\n");
        let second = import_into(themes.path(), &downloads.path().join("Paper")).expect("imports");
        assert_eq!(second, themes.path().join("Paper-2"));
        assert_eq!(std::fs::read_to_string(first).expect("read"), "background = #fafafa\n");
    }

    #[test]
    fn resolves_builtin_theme_case_insensitively() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (tokens, error) = resolve_theme(dir.path(), "tokyo night");
        assert_eq!(tokens.name, "Tokyo Night");
        assert!(error.is_none());
    }

    /// A fresh config starts with Wormhole's own theme.
    #[test]
    fn the_default_theme_is_wormholes_own() {
        let dir = tempfile::tempdir().expect("temp dir");
        for (name, appearance) in
            [(DEFAULT_THEME, Appearance::Dark), (DEFAULT_THEME_LIGHT, Appearance::Light)]
        {
            let (tokens, error) = resolve_theme(dir.path(), name);
            assert_eq!((tokens.name.as_str(), tokens.appearance, error), (name, appearance, None));
        }
    }

    #[test]
    fn errors_name_the_theme_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "t39.json", r##"{ "name": "T39", "ui": { "primry": "#fff" } }"##);
        write(dir.path(), "kid.json", r#"{ "name": "Kid", "extends": "T39" }"#);
        let (_, error) = resolve_theme(dir.path(), "T39");
        assert_eq!(
            error.as_deref(),
            Some("theme \"T39\": unknown ui token \"primry\" (did you mean `primary`?)")
        );
        // An error from further up the chain still says where it came from.
        let (_, error) = resolve_theme(dir.path(), "Kid");
        assert_eq!(
            error.as_deref(),
            Some("theme \"Kid\": theme \"T39\": unknown ui token \"primry\" (did you mean `primary`?)")
        );
    }

    #[test]
    fn unknown_theme_falls_back_with_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (tokens, error) = resolve_theme(dir.path(), "definitely-not-a-theme");
        assert_eq!(tokens.name, DEFAULT_THEME);
        assert!(error.is_some());
    }
}
