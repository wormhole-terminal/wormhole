//! User settings: `config.json` (comments allowed) in the data dir, live-reloaded when edited
//! by hand and written back when changed from the settings panel.
//!
//! A file that doesn't read in full is never written over: what it says is
//! applied key by key (one bad value doesn't reset the rest), and changes made
//! in the app stay in memory until the file is fixed. Saving edits the file in
//! place (see `file`), so comments and keys we don't know survive.
//!
//! The domain also holds what is a setting in all but file: a profile, a whole set of settings
//! (`profile`), the feature flags (`features`), the table of features and which are on
//! (`feature_table`) with the names of the panel's tabs (`tab`), how much the window shows and its
//! three presets (`presets`), the Settings window's table of every
//! setting (`settings_index`) and what a program may do with the clipboard (`clipboard`). Tier 3: it uses
//! the tiers below for the values it stores and checks (`storage`, `theme`, `agents`, `update`,
//! `view`, and `project` for a trusted project's overrides). `git`, `notify`, `ipc`, `terminal`
//! and `onboarding` read settings; config never uses them, so a setting can't depend on what
//! it configures.

pub mod clipboard;
pub mod feature_table;
pub mod features;
mod file;
mod font_slots;
pub mod presets;
pub mod profile;
mod settings;
pub mod settings_index;
pub mod tab;
mod theme_setting;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Map, Value};

pub use self::feature_table::{FEATURES, FeatureId, FeatureInfo, FeatureKind, FeatureMask};
#[cfg(test)]
pub(crate) use self::settings::keys;
pub use self::settings::{
    APP_SETTINGS, AgentAccess, BranchColor, ChangesScope, DockBadge, HotkeyAction, InFront, LINE_HEIGHT,
    MenuBarIcon, NotifySound, OPACITY, OptionAsMeta, PROJECT_SETTINGS, PlainChanges, SCROLLBACK_LINES, Settings,
    SharedChanges, ShellProfile, SizeUnits, ThemeMode, TitleBorder,
};
pub use self::tab::TabId;
use self::theme_setting::{ThemeStamp, default_tokens, resolve_theme};
pub use self::theme_setting::{
    resolve_theme_file, resolve_theme_file_in, tokens_from_json, tokens_from_json_in,
};
use crate::storage::files::{self, LinkPolicy};
use crate::storage::jsonc;
use crate::storage::jsonc::suggest::did_you_mean;
use crate::storage::paths::{DataRoot, ProfilePaths};
use crate::theme::tokens::{Appearance, ThemeTokens};
use crate::view::motion::MotionChoices;

/// Largest `config.json` read; real ones are about 1 KiB.
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

/// The settings file plus the theme it resolves to.
pub struct Config {
    pub settings: Settings,
    /// The theme shown: the dark or the light pick, by `theme_mode` and the system's appearance.
    pub tokens: ThemeTokens,
    /// A problem to show the user (bad JSON, unknown theme, …): the most
    /// important of the ones below.
    pub error: Option<String>,
    path: PathBuf,
    modified: Option<SystemTime>,
    /// The user's themes, which `settings.theme` and `extends` may name.
    themes: PathBuf,
    /// The system's appearance, which picks the theme under Match system. Dark until the app
    /// says otherwise: `Config` itself never asks the system.
    appearance: Appearance,
    /// What `tokens` were resolved from; resolving is skipped while it holds.
    theme_stamp: Option<ThemeStamp>,
    /// The file on disk didn't read in full, so saving would lose what it
    /// says; set until it reads again.
    unreadable: bool,
    /// Why the file didn't read.
    load_error: Option<String>,
    /// Why the last save failed.
    save_error: Option<String>,
    /// Why the theme didn't resolve (the default is shown instead).
    theme_error: Option<String>,
    /// A lesser problem: keys that mean nothing (any more).
    notice: Option<String>,
}

impl Config {
    /// The Default profile's `config.json`: the data folder's own, where it has always been.
    pub fn path() -> PathBuf {
        DataRoot::current().default_profile().config_file()
    }

    pub fn themes_dir() -> PathBuf {
        DataRoot::current().default_profile().themes_dir()
    }

    /// Load the Default profile's `config.json`, creating it with defaults if it doesn't exist.
    pub fn load() -> Self {
        Self::load_for(&DataRoot::current().default_profile())
    }

    /// Load a profile's `config.json` and themes, creating the file with defaults if it doesn't exist.
    pub fn load_for(paths: &ProfilePaths) -> Self {
        Self::load_from(paths.config_file(), paths.themes_dir())
    }

    fn load_from(path: PathBuf, themes: PathBuf) -> Self {
        let mut config = Self { path, themes, ..Self::placeholder() };
        config.apply(config.settings.clone());
        if config.path.exists() {
            config.reload();
        } else {
            config.write();
        }
        config
    }

    /// An in-memory default that touches no files (for swapping values out, and tests): no
    /// themes folder to read and no file to save to.
    pub fn placeholder() -> Self {
        Self {
            settings: Settings::default(),
            tokens: default_tokens(),
            error: None,
            path: PathBuf::new(),
            modified: None,
            themes: PathBuf::new(),
            appearance: Appearance::Dark,
            theme_stamp: None,
            unreadable: false,
            load_error: None,
            save_error: None,
            theme_error: None,
            notice: None,
        }
    }

    /// A config for `path` that hasn't read it yet, with its themes next to it
    /// (tests must never see the user's own).
    #[cfg(test)]
    fn at(path: PathBuf) -> Self {
        Self { themes: path.with_extension("themes"), path, ..Self::placeholder() }
    }

    /// Whether `config.json` failed to read in full. Until it is fixed, the
    /// settings come from what did read, and changes aren't saved.
    pub fn is_unreadable(&self) -> bool {
        self.unreadable
    }

    fn reload(&mut self) {
        self.modified = mtime(&self.path);
        let text = match files::read_to_string_capped(&self.path, MAX_CONFIG_BYTES, LinkPolicy::Follow) {
            Ok(text) => text,
            Err(err) => return self.mark_unreadable(&err.to_string(), None),
        };
        match jsonc::parse::<Settings>(&text) {
            Ok(settings) => {
                self.unreadable = false;
                self.load_error = None;
                self.notice = notice(&text);
                self.apply(settings.sanitized());
            }
            Err(err) => {
                let map = jsonc::parse::<Map<String, Value>>(&text).ok();
                self.mark_unreadable(&err, map.as_ref());
                self.notice = notice(&text);
                self.refresh_error();
            }
        }
    }

    /// The file is there but didn't read: apply whatever keys did (all of
    /// them except the broken ones), and stop saving over it.
    fn mark_unreadable(&mut self, err: &str, readable: Option<&Map<String, Value>>) {
        self.unreadable = true;
        self.load_error =
            Some(format!("config.json: {err} (changes made in Wormhole aren't saved until it's fixed)"));
        if let Some(map) = readable {
            self.apply(Settings::default().with_overrides(map).sanitized());
        }
        self.refresh_error();
    }

    /// Take `settings`, resolving the theme again only if the theme shown, or a file
    /// it may come from, changed: most settings have nothing to do with it.
    fn apply(&mut self, settings: Settings) {
        let shown = settings.shown_theme(self.appearance);
        let stamp = ThemeStamp::of(&self.themes, shown);
        if self.theme_stamp.as_ref() != Some(&stamp) {
            let (tokens, error) = resolve_theme(&self.themes, shown);
            self.tokens = tokens;
            self.theme_error = error;
            self.theme_stamp = Some(stamp);
        }
        self.settings = settings;
        self.refresh_error();
    }

    /// The problem to show: the file before the theme, the theme before the
    /// primary color, the notice last.
    fn refresh_error(&mut self) {
        self.error = self
            .load_error
            .clone()
            .or_else(|| self.save_error.clone())
            .or_else(|| self.theme_error.clone())
            .or_else(|| self.settings.primary_error())
            .or_else(|| self.settings.branch_color_error())
            .or_else(|| self.notice.clone());
    }

    /// Save the settings, unless the file on disk can't be read in full.
    /// Returns whether it was written.
    fn write(&mut self) -> bool {
        // The placeholder has nowhere to save to (an empty path would leave a temp file in the
        // working directory before the rename fails).
        if self.unreadable || self.path.as_os_str().is_empty() {
            return false;
        }
        let saved = self.save();
        self.save_error = saved.as_ref().err().map(|err| format!("could not save settings: {err}"));
        if let Err(err) = &saved {
            tracing::warn!("config.json: {err}");
        }
        self.modified = mtime(&self.path);
        self.refresh_error();
        saved.is_ok()
    }

    fn save(&mut self) -> Result<(), String> {
        let text = match files::read_to_string_capped(&self.path, MAX_CONFIG_BYTES, LinkPolicy::Follow) {
            Ok(old) => file::updated_text(&old, &self.settings).map_err(|err| {
                // Changed on disk since it was read, and not readable now: keep it.
                self.unreadable = true;
                self.load_error = Some(format!(
                    "config.json: {err} (changes made in Wormhole aren't saved until it's fixed)"
                ));
                err
            })?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => file::fresh_text(&self.settings)?,
            Err(err) => return Err(err.to_string()),
        };
        files::write_atomic(&self.path, text.as_bytes(), LinkPolicy::Follow).map_err(|e| e.to_string())
    }

    /// Whether [`poll`](Self::poll) would reload: `config.json` changed on disk, or a file the theme
    /// may come from did. Reads the disk and changes nothing, so a caller that guards a shared value
    /// can ask before it takes the value to change.
    pub fn changed_on_disk(&self) -> bool {
        let current = mtime(&self.path);
        if current.is_some() && current != self.modified {
            return true;
        }
        let stamp = ThemeStamp::of(&self.themes, self.settings.shown_theme(self.appearance));
        self.theme_stamp.as_ref() != Some(&stamp)
    }

    /// Reload if `config.json` changed on disk, or resolve the theme again if
    /// a file it may come from changed (the selected theme edited by hand, or
    /// saved from the Studio). Returns true if anything was reloaded.
    pub fn poll(&mut self) -> bool {
        let current = mtime(&self.path);
        if current.is_some() && current != self.modified {
            self.reload();
            return true;
        }
        let stamp = ThemeStamp::of(&self.themes, self.settings.shown_theme(self.appearance));
        if self.theme_stamp.as_ref() == Some(&stamp) {
            return false;
        }
        self.apply(self.settings.clone());
        true
    }

    /// The system's appearance changed (or is known for the first time): under Match system
    /// the other pick shows. Returns whether the appearance was new to it.
    pub fn set_appearance(&mut self, appearance: Appearance) -> bool {
        if self.appearance == appearance {
            return false;
        }
        self.appearance = appearance;
        self.apply(self.settings.clone());
        true
    }

    /// The name of the theme shown (`theme` or `theme_light`).
    pub fn shown_theme(&self) -> &str {
        self.settings.shown_theme(self.appearance)
    }

    /// Change settings (e.g. from the settings panel) and persist them.
    pub fn update(&mut self, f: impl FnOnce(&mut Settings)) {
        let mut settings = self.settings.clone();
        f(&mut settings);
        self.apply(settings.sanitized());
        self.write();
    }
}

/// What the file says that does nothing, said plainly: keys that aren't
/// settings (a typo, or a setting that was renamed), and what is wrong inside the `motion` object
/// (a value that is no option reads as the default, the other choices are kept). `$schema` and `x-*`
/// keys are for other tools and pass silently.
fn notice(text: &str) -> Option<String> {
    let map = jsonc::parse::<Map<String, Value>>(text).ok()?;
    let known = settings::keys();
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    let unknown: Vec<String> = keys
        .into_iter()
        .filter(|key| *key != "$schema" && !key.starts_with("x-") && !known.contains(key))
        .filter(|key| !settings::LEGACY_SETTINGS.contains(&key.as_str()))
        .map(|key| match key.as_str() {
            "accent_color" => "`accent_color` is now `primary_color`".to_string(),
            "liquid_glass" => {
                "`liquid_glass` is now a feature flag: `feature_flags` → `liquid_glass`".to_string()
            }
            _ => format!("unknown setting `{key}`{}", did_you_mean(key, known.iter().map(String::as_str))),
        })
        .collect();
    let motion = match map.get("motion") {
        Some(Value::Object(motion)) => MotionChoices::problems(motion),
        _ => Vec::new(),
    };
    let features = map
        .get("features")
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .and_then(|features| feature_table::features_notice(&features));
    let said: Vec<String> = unknown.into_iter().chain(motion).chain(features).collect();
    (!said.is_empty()).then(|| format!("config.json: {}", said.join("; ")))
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Config, Settings, ThemeMode};
    use crate::storage::jsonc;
    use crate::theme::tokens::Appearance;
    use crate::view::motion::{FoldMotion, InsertMotion, MotionChoices, StatusMotion};

    /// A temp dir with `config.json` holding `text`.
    fn config_file(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config.json");
        std::fs::write(&path, text).expect("write config");
        (dir, path)
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).expect("read config")
    }

    /// The themes dir next to a test's config file, never the real one.
    fn themes_dir(config: &Path) -> PathBuf {
        config.with_file_name("themes")
    }

    const ONE_BAD_VALUE: &str = "{\n  // Picked after a long evening.\n  \"theme\": \"Nord\",\n  \"opacity\": 0.5,\n  \"hotkey\": \"ctrl-`\",\n  \"font_size\": \"16\"\n}\n";

    /// What tests build their settings on: nothing of the person's data dir is read (their
    /// themes would change what a theme name resolves to) and nothing is written.
    #[test]
    fn the_placeholder_reads_and_writes_no_file() {
        let mut config = Config::placeholder();
        assert_eq!(config.themes, PathBuf::new());
        config.update(|s| s.theme = "Nord".into());
        assert_eq!(config.tokens.name, "Nord");
        assert_eq!(config.save_error, None, "it never tried to save");
    }

    #[test]
    fn a_bad_value_keeps_the_rest_and_the_file_is_never_overwritten() {
        let (_dir, path) = config_file(ONE_BAD_VALUE);
        let mut config = Config::load_from(path.clone(), themes_dir(&path));
        assert!(config.is_unreadable());
        assert_eq!(config.settings.theme, "Nord");
        assert_eq!(config.tokens.name, "Nord");
        assert_eq!(config.settings.opacity, 0.5);
        assert_eq!(config.settings.hotkey, "ctrl-`");
        assert_eq!(config.settings.font_size, Settings::default().font_size);
        let error = config.error.clone().unwrap_or_default();
        assert!(error.starts_with("config.json: line 6"), "{error}");
        assert!(error.contains("aren't saved until it's fixed"), "{error}");

        // An ordinary click changes the app but not the file, and keeps the error up.
        config.update(|s| s.diff_wrap = true);
        assert!(config.settings.diff_wrap);
        assert_eq!(config.settings.theme, "Nord");
        assert_eq!(read(&path), ONE_BAD_VALUE);
        assert_eq!(config.error.as_deref(), Some(error.as_str()));

        // Once fixed by hand, it reads again and saves again, comments and all.
        let fixed = ONE_BAD_VALUE.replace("\"16\"", "16");
        std::fs::write(&path, &fixed).expect("write");
        config.modified = None;
        assert!(config.changed_on_disk());
        assert!(config.poll());
        assert!(!config.is_unreadable());
        assert_eq!(config.error, None);
        assert_eq!(config.settings.font_size, 16.0);
        config.update(|s| s.diff_wrap = true);
        let saved = read(&path);
        assert!(saved.starts_with("{\n  // Picked after a long evening.\n  \"theme\": \"Nord\","), "{saved}");
        assert!(saved.contains("\"diff_wrap\": true"), "{saved}");
    }

    #[test]
    fn a_motion_that_is_no_option_is_the_default_and_is_said_while_the_rest_is_kept() {
        let text =
            "{\n  \"opacity\": 0.5,\n  \"motion\": { \"insert\": \"swoosh\", \"fold\": \"classic\" }\n}\n";
        let (_dir, path) = config_file(text);
        let mut config = Config::load_from(path.clone(), themes_dir(&path));
        assert_eq!(
            config.settings.motion.insert,
            MotionChoices::default().insert,
            "the bad one is its default"
        );
        assert_eq!(config.settings.motion.fold, FoldMotion::Classic, "the good one is kept");
        assert_eq!(config.settings.opacity, 0.5);
        assert!(!config.is_unreadable(), "the file reads, so it is saved as usual");
        let error = config.error.clone().unwrap_or_default();
        assert!(error.contains("`motion.insert`") && error.contains("swoosh"), "{error}");
        assert!(error.contains("from-side"), "it names the options: {error}");
        // A change made in the app is saved, and drops the value that was never in use.
        config.update(|s| s.motion.status = StatusMotion::Wash);
        let saved = read(&path);
        assert!(!saved.contains("swoosh"), "{saved}");
        let reread: Settings = jsonc::parse(&saved).expect("reads back");
        assert_eq!((reread.motion.fold, reread.motion.status), (FoldMotion::Classic, StatusMotion::Wash));
    }

    #[test]
    fn a_motion_choice_is_saved_into_the_object_in_place() {
        let (_dir, path) = config_file("{ \"motion\": { \"insert\": \"reveal\" } } // mine\n");
        let mut config = Config::load_from(path.clone(), themes_dir(&path));
        assert_eq!(config.settings.motion.insert, InsertMotion::Reveal);
        config.update(|s| s.motion.status = StatusMotion::Wash);
        let saved = read(&path);
        let reread: Settings = jsonc::parse(&saved).expect("reads back");
        assert_eq!(reread.motion.insert, InsertMotion::Reveal, "the choice already there is kept");
        assert_eq!(reread.motion.status, StatusMotion::Wash);
        assert!(saved.contains("// mine"), "{saved}");
        // Only what was chosen is in the file: a default that changes later still reaches this person.
        assert!(!saved.contains("\"fold\"") && !saved.contains("\"column\""), "{saved}");
    }

    #[test]
    fn a_file_that_is_not_json_keeps_the_settings_in_memory() {
        let (_dir, path) = config_file("{ \"theme\": \"Nord\", ");
        let mut config = Config::load_from(path.clone(), themes_dir(&path));
        assert!(config.is_unreadable());
        assert_eq!(config.settings, Settings::default());
        config.update(|s| s.theme = "Dracula".into());
        assert_eq!(config.settings.theme, "Dracula");
        assert_eq!(read(&path), "{ \"theme\": \"Nord\", ");
    }

    #[test]
    fn a_file_broken_after_loading_is_not_saved_over() {
        let (_dir, path) = config_file("{ \"theme\": \"Nord\" }");
        let mut config = Config::load_from(path.clone(), themes_dir(&path));
        assert!(!config.is_unreadable());
        // Broken by hand, and saved to from the app before the next poll.
        std::fs::write(&path, "{ \"theme\": 7 }").expect("write");
        config.update(|s| s.diff_split = false);
        assert_eq!(read(&path), "{ \"theme\": 7 }");
        assert!(config.is_unreadable());
        assert!(config.error.as_deref().is_some_and(|e| e.contains("aren't saved")));
    }

    #[test]
    fn a_byte_order_mark_does_not_reset_the_settings() {
        let (_dir, path) = config_file("\u{feff}{ \"theme\": \"Nord\" }");
        let config = Config::load_from(path.clone(), themes_dir(&path));
        assert!(!config.is_unreadable());
        assert_eq!(config.settings.theme, "Nord");
        assert_eq!(config.error, None);
    }

    #[test]
    fn saving_keeps_comments_schema_and_unknown_keys() {
        let text = "{\n  \"$schema\": \"https://example.com/config.json\",\n  /* mine */ \"theme\": \"Nord\",\n  \"x-note\": [1, 2],\n}\n";
        let (_dir, path) = config_file(text);
        let mut config = Config::load_from(path.clone(), themes_dir(&path));
        config.update(|s| s.theme = "Dracula".into());
        let saved = read(&path);
        assert!(saved.starts_with("{\n  \"$schema\": \"https://example.com/config.json\",\n  /* mine */ \"theme\": \"Dracula\",\n  \"x-note\": [1, 2],\n"), "{saved}");
        let reloaded = Config::load_from(path.clone(), themes_dir(&path));
        assert_eq!(reloaded.settings, config.settings);
    }

    /// Files written before the schema marker was dropped still say `"config_version": 1`: they
    /// load in full and are saved as before, the stale key reported and kept like any unknown one.
    #[test]
    fn a_config_with_the_old_version_key_still_loads() {
        let (_dir, path) =
            config_file("{\n  \"config_version\": 1,\n  \"theme\": \"Nord\",\n  \"font_size\": 15\n}\n");
        let mut config = Config::load_from(path.clone(), themes_dir(&path));
        assert!(!config.is_unreadable());
        assert_eq!((config.settings.theme.as_str(), config.settings.font_size), ("Nord", 15.0));
        assert_eq!(config.error.as_deref(), Some("config.json: unknown setting `config_version`"));
        config.update(|s| s.font_size = 16.0);
        assert!(read(&path).contains("\"font_size\": 16.0"));
    }

    #[test]
    fn a_new_config_is_written_in_full() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("sub").join("config.json");
        let config = Config::load_from(path.clone(), themes_dir(&path));
        assert_eq!(config.settings, Settings::default());
        let saved: Settings = crate::storage::jsonc::parse(&read(&path)).expect("parses");
        assert_eq!(saved, Settings::default());
    }
    #[test]
    fn a_symlinked_config_stays_a_symlink() {
        let (dir, real) = config_file("{ \"theme\": \"Nord\" }");
        let link = dir.path().join("link.json");
        crate::platform::fs::symlink(&real, &link).expect("symlink");
        let mut config = Config::load_from(link.clone(), themes_dir(&link));
        config.update(|s| s.font_size = 15.0);
        assert!(std::fs::symlink_metadata(&link).expect("link").file_type().is_symlink());
        assert!(read(&real).contains("\"font_size\": 15.0"));
        let names: Vec<_> =
            std::fs::read_dir(dir.path()).expect("dir").map(|e| e.expect("entry").file_name()).collect();
        assert_eq!(names.len(), 2, "no temp files left: {names:?}");
    }

    #[test]
    fn a_setting_unrelated_to_the_theme_does_no_theme_work() {
        let (_dir, path) = config_file(r#"{ "theme": "Mine" }"#);
        let themes = themes_dir(&path);
        std::fs::create_dir(&themes).expect("themes dir");
        let mine = themes.join("mine.json");
        std::fs::write(&mine, r##"{"name": "Mine", "extends": "Nord", "ui": {"primary": "#ff0000"}}"##)
            .expect("write theme");
        let _ = crate::theme::tokens::builtins();
        let mut config = Config::load_from(path.clone(), themes.clone());
        assert_eq!(config.tokens.name, "Mine");
        let builds = crate::theme::tokens::builds_on_this_thread;
        let before = builds();
        config.update(|s| s.diff_wrap = true);
        config.update(|s| s.font_size = 15.0);
        let _ = Config::placeholder();
        assert_eq!(builds() - before, 0);

        // Choosing another theme resolves it (a built-in is already resolved).
        config.update(|s| s.theme = "Dracula".into());
        assert_eq!(config.tokens.name, "Dracula");
        config.update(|s| s.theme = "Mine".into());
        assert_eq!(builds() - before, 1);
        // So does a change to a theme file.
        std::fs::write(&mine, r##"{"name": "Mine", "extends": "Nord", "ui": {"primary": "#00ff00"}}  "##)
            .expect("write theme");
        config.update(|s| s.diff_wrap = false);
        assert_eq!(config.tokens.get("primary").map(|c| c.to_hex()).as_deref(), Some("#00ff00"));
        assert_eq!(builds() - before, 2);
    }

    /// Match system shows the light pick while the system is light, and the mode can pin
    /// either pick whatever the system says.
    #[test]
    fn match_system_shows_the_pick_for_the_system_appearance() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut config = Config::at(dir.path().join("config.json"));
        let picks =
            Settings { theme: "Nord".into(), theme_light: "GitHub Light".into(), ..Settings::default() };
        config.apply(picks.clone());
        assert_eq!(config.settings.theme_mode, ThemeMode::System);
        assert_eq!(config.tokens.name, "Nord", "dark until the app says otherwise");

        assert!(config.set_appearance(Appearance::Light));
        assert_eq!((config.tokens.name.as_str(), config.shown_theme()), ("GitHub Light", "GitHub Light"));
        assert!(!config.set_appearance(Appearance::Light), "the same appearance again is no change");
        assert!(config.set_appearance(Appearance::Dark));
        assert_eq!(config.tokens.name, "Nord");

        // Dark and Light show their pick in either appearance.
        for appearance in [Appearance::Light, Appearance::Dark] {
            config.set_appearance(appearance);
            config.apply(Settings { theme_mode: ThemeMode::Dark, ..picks.clone() });
            assert_eq!(config.tokens.name, "Nord", "{appearance:?}");
            config.apply(Settings { theme_mode: ThemeMode::Light, ..picks.clone() });
            assert_eq!(config.tokens.name, "GitHub Light", "{appearance:?}");
        }
    }

    #[test]
    fn editing_the_selected_theme_file_reloads_it() {
        let (_dir, path) = config_file(r#"{ "theme": "Mine" }"#);
        let themes = themes_dir(&path);
        std::fs::create_dir(&themes).expect("themes dir");
        let mine = themes.join("mine.json");
        std::fs::write(&mine, r##"{"name": "Mine", "ui": {"primary": "#ff0000"}}"##).expect("write theme");
        let mut config = Config::load_from(path.clone(), themes.clone());
        assert!(!config.changed_on_disk(), "nothing changed: poll would not reload");
        assert!(!config.poll());
        std::fs::write(&mine, r##"{"name": "Mine", "ui": {"primary": "#0000ff"}} "##).expect("write theme");
        assert!(config.changed_on_disk(), "the theme file changed, and asking changes nothing");
        assert!(config.changed_on_disk());
        assert!(config.poll());
        assert!(!config.changed_on_disk());
        assert_eq!(config.tokens.get("primary").map(|c| c.to_hex()).as_deref(), Some("#0000ff"));
        assert!(!config.poll());
        // A theme that breaks is reported, and the default shown, until it is fixed.
        std::fs::write(&mine, "{ nope").expect("write theme");
        assert!(config.poll());
        assert!(
            config.error.as_deref().is_some_and(|e| e.starts_with("theme \"Mine\"")),
            "{:?}",
            config.error
        );
    }

    /// `cargo test -p wormhole-core --lib config_update_timing -- --ignored --nocapture`
    #[test]
    #[ignore = "timing, run by hand"]
    fn config_update_timing() {
        let (_dir, path) = config_file(r#"{ "theme": "Mine" }"#);
        let themes = themes_dir(&path);
        std::fs::create_dir(&themes).expect("themes dir");
        std::fs::write(themes.join("mine.json"), r##"{"name": "Mine", "extends": "Nord"}"##).expect("write");
        let mut config = Config::load_from(path, themes);
        let runs = 200;
        let start = std::time::Instant::now();
        for i in 0..runs {
            config.update(|s| s.diff_wrap = i % 2 == 0);
        }
        let update = start.elapsed() / runs;
        let start = std::time::Instant::now();
        for _ in 0..runs {
            std::hint::black_box(Config::placeholder());
        }
        let placeholder = start.elapsed() / runs;
        let start = std::time::Instant::now();
        for i in 0..runs {
            config.apply(Settings { diff_wrap: i % 2 == 0, ..config.settings.clone() });
        }
        let apply = start.elapsed() / runs;
        eprintln!(
            "Config::update {update:?} (apply {apply:?}, the rest is the save), placeholder {placeholder:?}"
        );
    }

    #[test]
    fn unknown_keys_are_reported_with_a_suggestion() {
        let (_dir, path) = config_file(
            r#"{ "$schema": "s", "x-mine": 1, "font_sise": 15, "theme": "Nord", "bogus_thing_entirely": true }"#,
        );
        let config = Config::load_from(path.clone(), themes_dir(&path));
        assert_eq!(config.settings.theme, "Nord");
        assert_eq!(
            config.error.as_deref(),
            Some(
                "config.json: unknown setting `bogus_thing_entirely`; \
                 unknown setting `font_sise` (did you mean `font_size`?)"
            )
        );
        let (_dir, path) = config_file(r#"{ "theme": "Nord", "$schema": "s" }"#);
        assert_eq!(Config::load_from(path.clone(), themes_dir(&path)).error, None);
        // A setting that became a feature flag says where it went.
        let (_dir, path) = config_file(r#"{ "liquid_glass": true }"#);
        assert_eq!(
            Config::load_from(path.clone(), themes_dir(&path)).error.as_deref(),
            Some("config.json: `liquid_glass` is now a feature flag: `feature_flags` → `liquid_glass`")
        );
    }

    #[test]
    fn a_bad_primary_color_is_reported_but_kept() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut config = Config::at(dir.path().join("config.json"));
        let chosen = |color: &str| Settings {
            primary_color: color.into(),
            theme_primary: Some(false),
            ..Settings::default()
        };
        config.apply(chosen("orange").sanitized());
        assert!(config.error.as_deref().is_some_and(|e| e.contains("primary_color")));
        assert_eq!(config.settings.primary_color, "orange");
        config.apply(chosen("#ff6600"));
        assert_eq!(config.error, None);
        // While the theme's own is in use, a color kept for later is not in the way.
        config.apply(Settings { theme_primary: Some(true), ..chosen("orange") });
        assert_eq!(config.error, None);
    }

    /// A file an older build wrote has the keys that later moved into another setting: they are still
    /// read, so the file says nothing that does nothing, and no notice says it does.
    #[test]
    fn keys_still_read_from_older_files_are_not_reported() {
        let (_dir, path) =
            config_file(r#"{ "title_ports": false, "title_pull_request": true, "compact_sidebar": false }"#);
        let mut config = Config::at(path);
        config.reload();
        assert_eq!(config.error, None);
        assert!(!config.settings.title_shows(crate::view::layout::title_parts::TitlePart::Ports));
    }

    #[test]
    fn an_old_accent_color_key_is_reported() {
        let (_dir, path) = config_file(r##"{ "accent_color": "#ff6600" }"##);
        let mut config = Config::at(path);
        config.reload();
        assert!(config.error.as_deref().is_some_and(|e| e.contains("`primary_color`")));
        assert_eq!(config.settings.primary(), None);
    }
}
