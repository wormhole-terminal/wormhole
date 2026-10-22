//! Project features and the per-user switches for them.
//!
//! A committed `.wormhole/` expresses what the *committer* wants; each user decides what they
//! accept, both globally ("never apply project themes") and per project.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{ProjectError, parse_json, read_state_file, set_aside, write_atomic};
use crate::storage::fsutil::LinkPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Settings,
    Theme,
    Actions,
    Prompts,
    Layout,
    Notes,
}

impl Feature {
    pub const ALL: [Feature; 6] =
        [Self::Settings, Self::Theme, Self::Actions, Self::Prompts, Self::Layout, Self::Notes];

    /// Features that can run commands or change the process environment need an approved
    /// [`super::TrustStore`] entry. Settings counts because `config.json` carries `env` and `shell`.
    /// Prompts are only pasted (and submitting them is an explicit action), so they don't.
    pub fn requires_trust(&self) -> bool {
        matches!(self, Self::Settings | Self::Actions | Self::Layout)
    }

    /// Stable key used in the prefs file.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Theme => "theme",
            Self::Actions => "actions",
            Self::Prompts => "prompts",
            Self::Layout => "layout",
            Self::Notes => "notes",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Theme => "Theme",
            Self::Actions => "Actions",
            Self::Prompts => "Prompts",
            Self::Layout => "Terminal layout",
            Self::Notes => "Shared notes",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Settings => "Setting overrides, environment variables and shell from config.json",
            Self::Theme => "The project's color theme from theme.json",
            Self::Actions => "Commands and key bindings from actions.json",
            Self::Prompts => "Reusable prompts from the prompts folder",
            Self::Layout => "Terminals opened (and commands started) with the project, from layout.json",
            Self::Notes => "Notes and next steps synced with notes.md",
        }
    }

    /// File (or directory, for prompts) inside `.wormhole/` backing this feature.
    pub fn file_name(&self) -> &'static str {
        match self {
            Self::Settings => super::CONFIG_FILE,
            Self::Theme => super::THEME_FILE,
            Self::Actions => super::ACTIONS_FILE,
            Self::Prompts => super::PROMPTS_DIR,
            Self::Layout => super::LAYOUT_FILE,
            Self::Notes => super::NOTES_FILE,
        }
    }

    /// What trusting the project applies for this feature, as a noun phrase for the approval notice
    /// (see [`trust_applies`]); empty for a feature that needs no trust.
    fn trust_phrase(&self) -> &'static str {
        match self {
            Self::Settings => "settings in config.json",
            Self::Actions => "actions in actions.json",
            Self::Layout => "terminal layout in layout.json",
            Self::Theme | Self::Prompts | Self::Notes => "",
        }
    }
}

/// What the approval notice says trusting applies when the project has no command to list (a
/// `config.json` with only `font_size` still needs trust, since the whole Settings feature does):
/// "its settings in config.json and actions in actions.json".
pub fn trust_applies(features: &[Feature]) -> String {
    let phrases: Vec<&str> = features.iter().map(Feature::trust_phrase).filter(|p| !p.is_empty()).collect();
    match phrases.as_slice() {
        [] => String::new(),
        [only] => format!("its {only}"),
        [init @ .., last] => format!("its {} and {last}", init.join(", ")),
    }
}

/// How a feature stands in a project, which decides the line under its switch: the same in the
/// Project tab and in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureStanding {
    /// Its file isn't there.
    Missing,
    /// The file is there and sets nothing: a new project's `config.json` has every key commented
    /// out. Trust has nothing to apply, so it is not a warning.
    Empty,
    /// It sets something that only takes effect once the project is trusted (again, when its
    /// files changed since).
    NeedsTrust,
    /// It applies: it needs no trust, or the project has it.
    Ready,
}

impl FeatureStanding {
    /// What to say about `feature`; `None` when it just applies, and its own description reads.
    pub fn hint(self, feature: Feature) -> Option<String> {
        match self {
            Self::Missing => Some(format!("No {} in this project", feature.file_name())),
            Self::Empty => Some(format!("Nothing set yet in {}", feature.file_name())),
            Self::NeedsTrust => Some("Needs trust before it takes effect".to_string()),
            Self::Ready => None,
        }
    }

    /// Whether the hint is a warning: something is held back until the person decides.
    pub fn warns(self) -> bool {
        self == Self::NeedsTrust
    }
}

/// Per-user feature switches, stored as `project-prefs.json` in the app's data dir.
///
/// Maps are keyed by strings rather than [`Feature`] so a prefs file written by a newer version
/// with more features still loads (unknown keys are kept and saved back).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectPrefs {
    #[serde(default)]
    defaults: BTreeMap<String, bool>,
    /// Canonical project root -> overrides.
    #[serde(default)]
    projects: BTreeMap<String, BTreeMap<String, bool>>,
    #[serde(skip)]
    path: PathBuf,
}

impl ProjectPrefs {
    pub const FILE_NAME: &'static str = "project-prefs.json";

    /// Load from `data_dir`; a missing file gives the defaults (everything enabled).
    pub fn load(data_dir: &Path) -> Result<Self, ProjectError> {
        let path = data_dir.join(Self::FILE_NAME);
        let mut prefs = match read_state_file(&path)? {
            Some(text) => parse_json::<Self>(&path, &text)?,
            None => Self::default(),
        };
        prefs.path = path;
        Ok(prefs)
    }

    /// [`Self::load`]; a file that doesn't load is moved aside (`project-prefs.json.bad`) and the
    /// defaults returned with the error. They can be saved again, so the user's choices from now
    /// on aren't lost, and the old file is still there to look at.
    pub fn load_or_recover(data_dir: &Path) -> (Self, Option<ProjectError>) {
        match Self::load(data_dir) {
            Ok(prefs) => (prefs, None),
            Err(err) => {
                let path = data_dir.join(Self::FILE_NAME);
                set_aside(&path);
                (Self { path, ..Self::default() }, Some(err))
            }
        }
    }

    pub fn save(&self) -> Result<(), ProjectError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ProjectError::invalid(&self.path, e.to_string()))?;
        // The user's own data dir: a symlinked file there is their dotfiles setup.
        write_atomic(&self.path, json.as_bytes(), LinkPolicy::Follow)
    }

    /// Project override, else the global default, else enabled.
    pub fn enabled(&self, root: &Path, feature: Feature) -> bool {
        self.project_override(root, feature).unwrap_or_else(|| self.default_enabled(feature))
    }

    /// Whether `root`'s terminals sync their notes with its `notes.md`: switched on, and the file is
    /// there (`file_present`) or the person switched Notes on for this project, which makes it.
    /// [`Self::enabled`] alone is true by default, so a project without a `notes.md` would show Notes
    /// on while nothing syncs; the switch and the sync both ask this instead.
    pub fn notes_sync(&self, root: &Path, file_present: bool) -> bool {
        self.enabled(root, Feature::Notes)
            && (file_present || self.project_override(root, Feature::Notes) == Some(true))
    }

    pub fn default_enabled(&self, feature: Feature) -> bool {
        self.defaults.get(feature.key()).copied().unwrap_or(true)
    }

    /// The explicit per-project value, if any (for showing "inherited" in the UI).
    pub fn project_override(&self, root: &Path, feature: Feature) -> Option<bool> {
        self.projects.get(&root_key(root))?.get(feature.key()).copied()
    }

    pub fn set_enabled(&mut self, root: &Path, feature: Feature, enabled: bool) {
        self.projects.entry(root_key(root)).or_default().insert(feature.key().to_string(), enabled);
    }

    /// Drop the per-project override so the global default applies again.
    pub fn clear_override(&mut self, root: &Path, feature: Feature) {
        let key = root_key(root);
        if let Some(map) = self.projects.get_mut(&key) {
            map.remove(feature.key());
            if map.is_empty() {
                self.projects.remove(&key);
            }
        }
    }

    pub fn set_default(&mut self, feature: Feature, enabled: bool) {
        self.defaults.insert(feature.key().to_string(), enabled);
    }

    pub fn enabled_features(&self, root: &Path) -> Vec<Feature> {
        Feature::ALL.into_iter().filter(|f| self.enabled(root, *f)).collect()
    }
}

/// Canonical path string, so `/tmp/x` and `/private/tmp/x` (or a trailing `/.`) share one entry.
/// Falls back to the path as given when it no longer exists, so stale entries stay addressable.
pub(crate) fn root_key(root: &Path) -> String {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf()).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn trust_requirements() {
        let trusted: Vec<_> = Feature::ALL.into_iter().filter(Feature::requires_trust).collect();
        assert_eq!(trusted, [Feature::Settings, Feature::Actions, Feature::Layout]);
        for f in Feature::ALL {
            assert_eq!(Feature::from_key(f.key()), Some(f));
            assert!(!f.label().is_empty() && !f.description().is_empty());
            // serde and `key()` agree, so either can be used in JSON.
            assert_eq!(serde_json::to_value(f).expect("ser"), serde_json::json!(f.key()));
        }
    }

    #[test]
    fn notes_sync_only_where_the_file_is_or_the_person_asked_for_it() {
        let data = TempDir::new().expect("tempdir");
        let root = Path::new("/repo");
        let mut prefs = ProjectPrefs::load(data.path()).expect("load");
        // On by default, but with no notes.md nothing syncs: the switch must not say it does.
        assert!(prefs.enabled(root, Feature::Notes));
        assert!(!prefs.notes_sync(root, false));
        assert!(prefs.notes_sync(root, true));
        // Switching it on for the project is the ask (it makes the file); off wins over the file.
        prefs.set_enabled(root, Feature::Notes, true);
        assert!(prefs.notes_sync(root, false) && prefs.notes_sync(root, true));
        prefs.set_enabled(root, Feature::Notes, false);
        assert!(!prefs.notes_sync(root, false) && !prefs.notes_sync(root, true));
        // A default of off holds for a project that has the file, until it asks.
        prefs.clear_override(root, Feature::Notes);
        prefs.set_default(Feature::Notes, false);
        assert!(!prefs.notes_sync(root, true));
        prefs.set_enabled(root, Feature::Notes, true);
        assert!(prefs.notes_sync(root, true));
    }

    #[test]
    fn what_a_feature_row_says() {
        use FeatureStanding::{Empty, Missing, NeedsTrust, Ready};
        assert_eq!(Missing.hint(Feature::Layout).as_deref(), Some("No layout.json in this project"));
        assert_eq!(Empty.hint(Feature::Settings).as_deref(), Some("Nothing set yet in config.json"));
        assert_eq!(NeedsTrust.hint(Feature::Actions).as_deref(), Some("Needs trust before it takes effect"));
        assert_eq!(Ready.hint(Feature::Theme), None, "its description reads");
        // Only what is held back warns; an empty file is just empty.
        assert!([Missing, Empty, Ready].iter().all(|standing| !standing.warns()) && NeedsTrust.warns());
    }

    #[test]
    fn the_approval_notice_names_what_trusting_applies() {
        assert_eq!(trust_applies(&[Feature::Settings]), "its settings in config.json");
        assert_eq!(
            trust_applies(&[Feature::Settings, Feature::Layout]),
            "its settings in config.json and terminal layout in layout.json"
        );
        assert_eq!(
            trust_applies(&[Feature::Settings, Feature::Actions, Feature::Layout]),
            "its settings in config.json, actions in actions.json and terminal layout in layout.json"
        );
        assert_eq!(trust_applies(&[]), "");
        assert_eq!(trust_applies(&[Feature::Theme, Feature::Notes]), "", "nothing to approve");
    }

    #[test]
    fn defaults_and_overrides() {
        let data = TempDir::new().expect("tempdir");
        let proj = TempDir::new().expect("tempdir");
        let root = proj.path();
        let other = Path::new("/nonexistent/other");

        let mut prefs = ProjectPrefs::load(data.path()).expect("load");
        assert!(Feature::ALL.iter().all(|f| prefs.enabled(root, *f)), "all enabled by default");

        prefs.set_default(Feature::Theme, false);
        assert!(!prefs.enabled(root, Feature::Theme));
        assert!(!prefs.enabled(other, Feature::Theme));

        prefs.set_enabled(root, Feature::Theme, true);
        prefs.set_enabled(root, Feature::Actions, false);
        assert!(prefs.enabled(root, Feature::Theme), "project override beats default");
        assert!(!prefs.enabled(other, Feature::Theme));
        assert!(!prefs.enabled(root, Feature::Actions));
        assert!(prefs.enabled(other, Feature::Actions));
        // A non-canonical spelling of the same root hits the same entry.
        assert!(!prefs.enabled(&root.join("."), Feature::Actions));

        prefs.save().expect("save");
        let reloaded = ProjectPrefs::load(data.path()).expect("load");
        assert_eq!(reloaded, prefs);
        assert_eq!(reloaded.project_override(root, Feature::Actions), Some(false));
        assert_eq!(reloaded.enabled_features(root).len(), 5);

        prefs.clear_override(root, Feature::Theme);
        prefs.clear_override(root, Feature::Actions);
        assert!(!prefs.enabled(root, Feature::Theme));
        assert!(prefs.projects.is_empty());
    }

    #[test]
    fn tolerant_and_forward_compatible_file() {
        let data = TempDir::new().expect("tempdir");
        let text = r#"{
            // written by hand
            "defaults": {"notes": false, "future_feature": true,},
        }"#;
        std::fs::write(data.path().join(ProjectPrefs::FILE_NAME), text).expect("write");
        let prefs = ProjectPrefs::load(data.path()).expect("load");
        assert!(!prefs.default_enabled(Feature::Notes));
        prefs.save().expect("save");
        let saved = std::fs::read_to_string(data.path().join(ProjectPrefs::FILE_NAME)).expect("read");
        assert!(saved.contains("future_feature"));

        std::fs::write(data.path().join(ProjectPrefs::FILE_NAME), "{\"defaults\": 3}").expect("write");
        assert!(matches!(ProjectPrefs::load(data.path()), Err(ProjectError::Parse { .. })));
    }

    /// A prefs file that doesn't load used to leave the app with defaults that could
    /// never be saved (no path), so every choice made afterwards was lost.
    #[test]
    fn a_broken_file_is_kept_aside_and_saving_works_again() {
        let data = TempDir::new().expect("tempdir");
        let file = data.path().join(ProjectPrefs::FILE_NAME);
        std::fs::write(&file, "{\"defaults\": {\"notes\": false}, oops").expect("write");
        let (mut prefs, err) = ProjectPrefs::load_or_recover(data.path());
        assert!(err.is_some());
        assert!(prefs.default_enabled(Feature::Notes), "defaults");
        prefs.set_default(Feature::Theme, false);
        prefs.save().expect("save");
        assert!(!ProjectPrefs::load(data.path()).expect("load").default_enabled(Feature::Theme));
        let aside = data.path().join(format!("{}.bad", ProjectPrefs::FILE_NAME));
        assert!(std::fs::read_to_string(aside).expect("kept").contains("oops"));
        // A good file loads as before, with nothing set aside.
        let (prefs, err) = ProjectPrefs::load_or_recover(data.path());
        assert!(err.is_none() && !prefs.default_enabled(Feature::Theme));
    }
}
