//! Every setting the settings window shows, as data: its sections (and the nav groups they sit
//! in), the groups of each section, and each setting's label, description, search words and the
//! `config.json` keys it writes. The window draws its pages from this table (adding only the
//! controls, which need GPUI), the search runs over it (`search`), and so will the command
//! palette's Settings scope. It also says whether a setting differs from its default, which
//! project overrode it, and puts settings back to their defaults.
//!
//! A new setting is a row here plus its control in the app's `settings_page/registry.rs`; a
//! test fails while a `config.json` key has neither a row nor a place in [`UNLISTED`].

mod search;
mod table;

use serde_json::{Map, Value};

use crate::config::{PROJECT_SETTINGS, Settings};
pub use search::{Hit, Query, search, search_visible};

/// A page of the settings window, in nav order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SectionId {
    /// The profile Wormhole is in: its name, folders, environment, git identity and agent logins.
    Profile,
    Appearance,
    Fonts,
    Terminal,
    Sessions,
    Git,
    Agents,
    Notifications,
    MenuBar,
    Updates,
    Advanced,
    // Preview (dev builds only): sections behind a feature flag, listed last.
    Projects,
    FeatureFlags,
}

/// The headings the nav groups its sections under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavGroup {
    /// The profile page, first and without a heading of its own.
    Profile,
    Look,
    Work,
    Agents,
    App,
    /// In-progress, feature-flagged sections, shown only in a development build.
    Preview,
}

impl NavGroup {
    pub fn title(self) -> &'static str {
        match self {
            NavGroup::Profile => "",
            NavGroup::Look => "Look",
            NavGroup::Work => "Work",
            NavGroup::Agents => "Agents",
            NavGroup::App => "App",
            NavGroup::Preview => "Preview",
        }
    }
}

/// What a section is called, what its page says under the title, and where the nav lists it.
pub struct SectionInfo {
    pub title: &'static str,
    pub intro: &'static str,
    pub nav: NavGroup,
}

/// A titled run of settings on a page (an empty title: rows without a label above them).
pub struct Group {
    pub title: &'static str,
    /// A paragraph under the title; empty for none.
    pub blurb: &'static str,
    /// A quiet note at the right of the title ("Follows the terminal until you pick one").
    pub aside: &'static str,
    pub entries: &'static [Entry],
}

/// One setting.
pub struct Entry {
    /// Names the setting's row and control in the app, and the setting in tests and devctl.
    /// Unique across the sections.
    pub id: &'static str,
    pub label: &'static str,
    /// The quiet line under the label; empty for none. A page may say something more
    /// current in its place (the font in use, how many custom fonts were found).
    pub description: &'static str,
    /// Words that find it besides its label and description.
    pub keywords: &'static str,
    /// The `config.json` keys it writes: what "changed", the reset arrow and "set by a project"
    /// look at. Empty for a row that stores nothing there (a folder button, the agents' hooks).
    pub keys: &'static [&'static str],
}

impl SectionId {
    pub const ALL: [SectionId; 13] = [
        SectionId::Profile,
        SectionId::Appearance,
        SectionId::Fonts,
        SectionId::Terminal,
        SectionId::Sessions,
        SectionId::Git,
        SectionId::Agents,
        SectionId::Notifications,
        SectionId::MenuBar,
        SectionId::Updates,
        SectionId::Advanced,
        // Preview, last: behind a feature flag and shown only in a dev build.
        SectionId::Projects,
        SectionId::FeatureFlags,
    ];

    pub fn info(self) -> &'static SectionInfo {
        table::section(self)
    }

    pub fn title(self) -> &'static str {
        self.info().title
    }

    pub fn groups(self) -> &'static [Group] {
        table::groups(self)
    }

    /// Every setting of the section, with the group it is in.
    pub fn entries(self) -> impl Iterator<Item = (&'static Group, &'static Entry)> {
        self.groups().iter().flat_map(|group| group.entries.iter().map(move |entry| (group, entry)))
    }

    /// Whether this section appears in the nav (and its settings in search) for this build. The
    /// Preview sections are gated: the feature-flag list shows only in a development build or on
    /// the Unstable channel, and a feature's own section (Project folders) shows only while its
    /// flag is on, so it moves from hidden to shown the moment the flag is turned on, and vanishes
    /// in a release build where the feature isn't there. Everything else is always shown.
    pub fn is_visible(self, settings: &Settings) -> bool {
        self.is_visible_in(settings, settings.flag_mode())
    }

    /// [`Self::is_visible`] for a given [`FlagMode`](crate::config::features::FlagMode), so the tests,
    /// which are always dev builds, can check what a release shows too.
    pub fn is_visible_in(self, settings: &Settings, mode: crate::config::features::FlagMode) -> bool {
        match self {
            SectionId::FeatureFlags => mode.shows_switches(),
            _ => self
                .feature_flag()
                .is_none_or(|flag| crate::config::features::enabled_in(flag, &settings.feature_flags, mode)),
        }
    }

    /// The feature flag whose section this is, if any: such a section shows only while the flag is
    /// on. Lets the preview notice name the flag without hard-coding it per section.
    pub fn feature_flag(self) -> Option<crate::config::features::FeatureFlag> {
        match self {
            SectionId::Projects => Some(crate::config::features::FeatureFlag::ProjectFolders),
            _ => None,
        }
    }

    /// Whether this section is a preview feature (shown with a "Preview" mark and a notice), as
    /// opposed to one that always ships.
    pub fn is_preview(self) -> bool {
        self == SectionId::FeatureFlags || self.feature_flag().is_some()
    }

    /// The sections shown in the nav, in order, for this build and settings.
    pub fn visible(settings: &Settings) -> impl Iterator<Item = SectionId> + '_ {
        SectionId::ALL.into_iter().filter(move |section| section.is_visible(settings))
    }
}

/// Every setting, with its section and group.
pub fn entries() -> impl Iterator<Item = (SectionId, &'static Group, &'static Entry)> {
    SectionId::ALL.into_iter().flat_map(|section| section.entries().map(move |(g, e)| (section, g, e)))
}

/// The setting called `id`.
pub fn entry(id: &str) -> Option<(SectionId, &'static Group, &'static Entry)> {
    entries().find(|(_, _, entry)| entry.id == id)
}

/// `config.json` keys without a row of their own: `notifications` is the master switch, which
/// the Notifications section's switches set (on while any kind is on); `folder_colors` is a list the
/// Project tab sets, with nothing to set in a form.
pub const UNLISTED: &[&str] = &["notifications", "folder_colors"];

fn as_map(settings: &Settings) -> Map<String, Value> {
    match serde_json::to_value(settings) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// The settings as values to compare, and the defaults beside them.
pub struct Values {
    current: Map<String, Value>,
    defaults: Map<String, Value>,
}

impl Values {
    pub fn of(settings: &Settings) -> Self {
        Self { current: as_map(settings), defaults: as_map(&Settings::default()) }
    }

    /// Whether `key` is set to something other than its default.
    pub fn changed(&self, key: &str) -> bool {
        self.current.get(key) != self.defaults.get(key)
    }

    /// Whether any key of `entry` differs from its default.
    pub fn entry_changed(&self, entry: &Entry) -> bool {
        entry.keys.iter().any(|key| self.changed(key))
    }

    /// Every key that differs from its default, sorted: what "Reset all settings" puts back.
    pub fn changed_keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> =
            self.current.keys().map(String::as_str).filter(|key| self.changed(key)).collect();
        keys.sort_unstable();
        keys
    }
}

/// `settings` with `keys` back at their defaults (the reset arrow of a row).
pub fn reset(settings: &Settings, keys: &[&str]) -> Settings {
    let mut map = as_map(settings);
    let defaults = as_map(&Settings::default());
    for key in keys {
        if let Some(value) = defaults.get(*key) {
            map.insert((*key).to_string(), value.clone());
        }
    }
    serde_json::from_value(Value::Object(map)).unwrap_or_else(|_| settings.clone())
}

/// Whether a project sets one of `entry`'s keys: `overridden` is what
/// `Settings::with_overrides_reported` says the project's overlay applied. A key a project may
/// not change (the app-wide ones) never counts.
pub fn set_by_project(entry: &Entry, overridden: &[String]) -> bool {
    entry.keys.iter().any(|key| PROJECT_SETTINGS.contains(key) && overridden.iter().any(|o| o == key))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{SectionId, UNLISTED, Values, entries, entry, reset, set_by_project};
    use crate::config::{APP_SETTINGS, PROJECT_SETTINGS, Settings, keys};

    #[test]
    fn every_config_key_has_a_row_or_is_listed_as_waiting_for_one() {
        let shown: HashSet<&str> = entries().flat_map(|(_, _, e)| e.keys.iter().copied()).collect();
        for key in keys() {
            let unlisted = UNLISTED.contains(&key.as_str());
            assert!(shown.contains(key.as_str()) || unlisted, "{key}: give it a row in settings_index");
            assert!(!(shown.contains(key.as_str()) && unlisted), "{key} has a row: take it out of UNLISTED");
        }
        // And a row names only keys that exist.
        let known = keys();
        for (_, _, entry) in entries() {
            for key in entry.keys {
                assert!(known.iter().any(|k| k == key), "{}: no config key {key}", entry.id);
            }
        }
    }

    #[test]
    fn ids_are_unique_and_rows_are_named() {
        let mut seen = HashSet::new();
        for (section, group, entry) in entries() {
            assert!(seen.insert(entry.id), "{} appears twice", entry.id);
            assert!(!entry.label.is_empty(), "{} has no label", entry.id);
            assert!(!group.entries.is_empty(), "an empty group in {section:?}");
        }
        assert_eq!(entry("tab-width").map(|(s, _, _)| s), Some(SectionId::Git));
    }

    #[test]
    fn the_preview_sections_show_only_in_a_dev_build_or_on_unstable_and_while_their_flag_is_on() {
        use crate::config::features::FlagMode::{Dev as DEV, Release as RELEASE, Unstable as UNSTABLE};
        let preview = [SectionId::Projects, SectionId::FeatureFlags];
        let defaults = Settings::default();
        // A release build: no flag list, and a flagged feature stays hidden whatever the file says.
        let asked_on =
            Settings { feature_flags: [("project_folders".to_string(), true)].into(), ..Settings::default() };
        for settings in [&defaults, &asked_on] {
            for section in preview {
                assert!(!section.is_visible_in(settings, RELEASE), "{section:?} in a release build");
            }
        }
        // A dev build: the flag list, and the feature's section while its flag is on (the default).
        assert!(SectionId::FeatureFlags.is_visible_in(&defaults, DEV));
        assert!(SectionId::Projects.is_visible_in(&defaults, DEV));
        let off = Settings {
            feature_flags: [("project_folders".to_string(), false)].into(),
            ..Settings::default()
        };
        assert!(!SectionId::Projects.is_visible_in(&off, DEV));
        assert!(SectionId::FeatureFlags.is_visible_in(&off, DEV), "the switch to turn it back on stays");
        // Every other section ships in both.
        for section in SectionId::ALL.into_iter().filter(|s| !preview.contains(s)) {
            assert!(!section.is_preview());
            assert!(section.is_visible_in(&off, RELEASE) && section.is_visible_in(&off, DEV), "{section:?}");
        }
        assert!(preview.iter().all(|s| s.is_preview()));
        // A release on Unstable: the flag list, and a feature's section once a tester turns it on.
        assert!(SectionId::FeatureFlags.is_visible_in(&defaults, UNSTABLE));
        assert!(!SectionId::Projects.is_visible_in(&defaults, UNSTABLE));
        assert!(SectionId::Projects.is_visible_in(&asked_on, UNSTABLE));
    }

    #[test]
    fn every_section_has_a_page() {
        for section in SectionId::ALL {
            assert!(!section.title().is_empty());
            assert!(!section.groups().is_empty(), "{section:?} has nothing to show");
        }
    }

    #[test]
    fn a_changed_value_is_noticed_and_reset_puts_back_only_its_keys() {
        let settings = Settings { tab_width: 8, diff_wrap: true, font_size: 16.0, ..Settings::default() };
        let values = Values::of(&settings);
        assert!(values.changed("tab_width") && values.changed("diff_wrap"));
        assert!(!values.changed("blur"));
        assert_eq!(values.changed_keys(), ["diff_wrap", "font_size", "tab_width"]);
        let (_, _, font) = entry("font-terminal").expect("the terminal font row");
        assert!(values.entry_changed(font), "the font row covers its size too");

        let back = reset(&settings, &["tab_width"]);
        assert_eq!(back.tab_width, Settings::default().tab_width);
        assert!(back.diff_wrap, "the other changes stay");
        assert!(Values::of(&Settings::default()).changed_keys().is_empty());
    }

    #[test]
    fn only_keys_a_project_may_change_count_as_set_by_one() {
        let (_, _, wrap) = entry("diff-wrap").expect("row");
        let (_, _, hotkey) = entry("hotkey").expect("row");
        assert!(set_by_project(wrap, &["diff_wrap".into()]));
        assert!(!set_by_project(wrap, &["tab_width".into()]));
        // The hotkey is the app's: an overlay that names it was never applied.
        assert!(APP_SETTINGS.contains(&"hotkey") && !PROJECT_SETTINGS.contains(&"hotkey"));
        assert!(!set_by_project(hotkey, &["hotkey".into()]));
        // Nor is the theme: a project's comes from its theme.json, which the page asks about.
        let (_, _, theme) = entry("theme").expect("row");
        let (_, _, mode) = entry("theme-mode").expect("row");
        assert!(!set_by_project(theme, &["theme".into(), "theme_light".into()]));
        assert!(!set_by_project(mode, &["theme_mode".into()]));
    }
}
