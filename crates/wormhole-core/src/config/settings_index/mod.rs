//! Every setting the settings window shows, as data: its sections (and the nav groups they sit
//! in), the groups of each section, and each setting's label, description, search words and the
//! `config.json` keys it writes. The window draws its pages from this table (adding only the
//! controls, which need GPUI), the search runs over it (`search`), and so will the command
//! palette's Settings scope. It also says whether a setting differs from its default, which
//! project overrode it, and puts settings back to their defaults.
//!
//! A new setting is a row here plus its control in the app's `settings_window/registry.rs`; a
//! test fails while a `config.json` key has neither a row nor a place in [`UNLISTED`].

#[cfg(test)]
mod english;
mod key_path;
mod search;
mod table;

use wormhole_text::keys::{
    SETTINGS_NAV_AGENTS, SETTINGS_NAV_APP, SETTINGS_NAV_LOOK, SETTINGS_NAV_PREVIEW, SETTINGS_NAV_WORK,
};
use wormhole_text::{Key, Language, text, text_in};

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
    /// How the app moves, one choice and live preview each. Behind [`crate::config::features::FeatureFlag::Motion`].
    Motion,
    Terminal,
    Sessions,
    Git,
    Agents,
    /// Other terminals' settings, brought over, and how they compare.
    Import,
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
            NavGroup::Look => text(SETTINGS_NAV_LOOK),
            NavGroup::Work => text(SETTINGS_NAV_WORK),
            NavGroup::Agents => text(SETTINGS_NAV_AGENTS),
            NavGroup::App => text(SETTINGS_NAV_APP),
            NavGroup::Preview => text(SETTINGS_NAV_PREVIEW),
        }
    }
}

/// What a section is called, what its page says under the title, and where the nav lists it.
pub struct SectionInfo {
    title: Key,
    intro: Key,
    pub nav: NavGroup,
}

impl SectionInfo {
    pub fn title(&self) -> &'static str {
        text(self.title)
    }

    /// The line under the page's title.
    pub fn intro(&self) -> &'static str {
        text(self.intro)
    }
}

/// A titled run of settings on a page (an empty title: rows without a label above them).
pub struct Group {
    title: Option<Key>,
    /// A paragraph under the title.
    blurb: Option<Key>,
    /// A quiet note at the right of the title ("Follows the terminal until you pick one").
    aside: Option<Key>,
    pub entries: &'static [Entry],
}

impl Group {
    pub fn title(&self) -> &'static str {
        self.title.map_or("", text)
    }

    /// A paragraph under the title; empty for none.
    pub fn blurb(&self) -> &'static str {
        self.blurb.map_or("", text)
    }

    /// A quiet note at the right of the title ("Follows the terminal until you pick one"); empty for none.
    pub fn aside(&self) -> &'static str {
        self.aside.map_or("", text)
    }
}

/// One setting.
pub struct Entry {
    /// Names the setting's row and control in the app, and the setting in tests and devctl.
    /// Unique across the sections.
    pub id: &'static str,
    label: Key,
    /// The quiet line under the label. A page may say something more current in its place (the font
    /// in use, how many custom fonts were found).
    description: Option<Key>,
    /// Words that find it besides its label and description.
    keywords: Option<Key>,
    /// The `config.json` keys it writes: what "changed", the reset arrow and "set by a project"
    /// look at. A key inside an object is its path (`motion.insert`), so each choice of an object
    /// has its own changed mark and reset. Empty for a row that stores nothing there (a folder
    /// button, the agents' hooks).
    pub keys: &'static [&'static str],
    /// The feature it is about, if it is one of an optional feature: while that is off the setting
    /// does nothing. `None` for the rest.
    pub feature: Option<crate::config::FeatureId>,
    /// The feature flag that hides the row while it is off (an unfinished feature's setting on a page
    /// that ships): neither the page nor the search shows it then.
    flag: Option<crate::config::features::FeatureFlag>,
}

impl Entry {
    pub fn label(&self) -> &'static str {
        text(self.label)
    }

    /// The quiet line under the label; empty for none.
    pub fn description(&self) -> &'static str {
        self.description.map_or("", text)
    }

    /// Words that find it besides its label and description; empty for none.
    pub fn keywords(&self) -> &'static str {
        self.keywords.map_or("", text)
    }

    /// The label, the description and the search words in one particular language (empty where it has
    /// none): what the search index is built from, once per language.
    fn words_in(&self, language: Language) -> [&'static str; 3] {
        let of = |key: Option<Key>| key.map_or("", |key| text_in(language, key));
        [text_in(language, self.label), of(self.description), of(self.keywords)]
    }

    /// Whether the row is shown for these settings: always, unless an unfinished feature's flag
    /// that it hangs on is off in this build.
    pub fn is_shown(&self, settings: &Settings) -> bool {
        self.flag.is_none_or(|flag| settings.feature_enabled(flag))
    }

    /// Whether the setting is of use with these features on: always, unless it is about a feature
    /// that is off.
    pub fn is_of_use(&self, features: crate::config::FeatureMask) -> bool {
        self.feature.is_none_or(|feature| features.has(feature))
    }
}

impl SectionId {
    pub const ALL: [SectionId; 15] = [
        SectionId::Profile,
        SectionId::Appearance,
        SectionId::Fonts,
        SectionId::Motion,
        SectionId::Terminal,
        SectionId::Sessions,
        SectionId::Git,
        SectionId::Agents,
        SectionId::Import,
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
        self.info().title()
    }

    pub fn groups(self) -> &'static [Group] {
        table::groups(self)
    }

    /// Every setting of the section, with the group it is in.
    pub fn entries(self) -> impl Iterator<Item = (&'static Group, &'static Entry)> {
        self.groups().iter().flat_map(|group| group.entries.iter().map(move |entry| (group, entry)))
    }

    /// Whether this section appears in the nav (and its settings in search) for this build. The
    /// Preview sections are gated: the feature-flag list shows only in a development build, on the
    /// Unstable channel or with the flags unlocked, and a feature's own section (Project folders) shows only while its
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
            // Project folders switched off as a feature (`features`) have nothing to set either.
            SectionId::Projects
                if !settings.feature_switched_on(crate::config::feature_table::FeatureId::Project) =>
            {
                false
            }
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
            SectionId::Motion => Some(crate::config::features::FeatureFlag::Motion),
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
/// Project tab sets, with nothing to set in a form; `features`, the person's switches for optional
/// features, which the first run's walk sets until Settings has a page of them.
pub const UNLISTED: &[&str] = &["notifications", "folder_colors", "features"];

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
        key_path::get(&self.current, key) != key_path::get(&self.defaults, key)
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
        match key_path::get(&defaults, key) {
            Some(value) => key_path::set(&mut map, key, value.clone()),
            // Stored only while it differs (the motion choices): its default is that it isn't there.
            None => key_path::remove(&mut map, key),
        }
    }
    serde_json::from_value(Value::Object(map)).unwrap_or_else(|_| settings.clone())
}

/// Whether a project sets one of `entry`'s keys: `overridden` is what
/// `Settings::with_overrides_reported` says the project's overlay applied. A key a project may
/// not change (the app-wide ones) never counts.
pub fn set_by_project(entry: &Entry, overridden: &[String]) -> bool {
    // A project overrides a setting as a whole: the key of a choice inside an object is the object's.
    entry
        .keys
        .iter()
        .map(|key| key_path::top(key))
        .any(|key| PROJECT_SETTINGS.contains(&key) && overridden.iter().any(|overridden| overridden == key))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{Entry, SectionId, UNLISTED, Values, entries, entry, reset, set_by_project};
    use super::{as_map, key_path};
    use crate::config::{APP_SETTINGS, PROJECT_SETTINGS, Settings, keys};

    #[test]
    fn a_row_about_a_feature_is_of_use_only_while_it_is_on() {
        use crate::config::features::FlagMode;
        use crate::config::{FeatureId, FeatureMask};
        let (_, _, notes) = entries().find(|(_, _, e)| e.id == "sidebar-notes").expect("the row");
        assert_eq!(notes.feature, Some(FeatureId::Notes));
        let mut off = Settings::default();
        off.feature_flags.insert("notes_and_tasks".into(), false);
        assert!(!notes.is_of_use(FeatureMask::resolve_in(&off, FlagMode::Dev)));
        assert!(notes.is_of_use(FeatureMask::resolve_in(&Settings::default(), FlagMode::Dev)));
        assert!(entries().filter(|(_, _, e)| e.feature.is_some()).count() >= 1);
    }

    #[test]
    fn every_config_key_has_a_row_or_is_listed_as_waiting_for_one() {
        let shown: HashSet<&str> =
            entries().flat_map(|(_, _, e)| e.keys.iter().map(|key| key_path::top(key))).collect();
        for key in keys() {
            let unlisted = UNLISTED.contains(&key.as_str());
            assert!(shown.contains(key.as_str()) || unlisted, "{key}: give it a row in settings_index");
            assert!(!(shown.contains(key.as_str()) && unlisted), "{key} has a row: take it out of UNLISTED");
        }
        // And a row names only keys that exist.
        // The motion choices are written only while chosen, so look at a file where every one is.
        use crate::view::motion::{ColumnMotion, FoldMotion, InsertMotion, MotionChoices, StatusMotion};
        let chosen = Settings {
            motion: MotionChoices {
                insert: InsertMotion::Classic,
                fold: FoldMotion::Classic,
                status: StatusMotion::Crossfade,
                column: ColumnMotion::SoftSpring,
            },
            ..Settings::default()
        };
        let written = as_map(&chosen);
        for (_, _, entry) in entries() {
            for key in entry.keys {
                assert!(key_path::get(&written, key).is_some(), "{}: no config key {key}", entry.id);
            }
        }
    }

    #[test]
    fn ids_are_unique_and_rows_are_named() {
        let mut seen = HashSet::new();
        for (section, group, entry) in entries() {
            assert!(seen.insert(entry.id), "{} appears twice", entry.id);
            assert!(!entry.label().is_empty(), "{} has no label", entry.id);
            assert!(!group.entries.is_empty(), "an empty group in {section:?}");
        }
        assert_eq!(entry("tab-width").map(|(s, _, _)| s), Some(SectionId::Git));
    }

    #[test]
    fn the_preview_sections_show_only_in_a_dev_build_or_on_unstable_and_while_their_flag_is_on() {
        use crate::config::features::FlagMode::{Dev as DEV, Release as RELEASE, Unstable as UNSTABLE};
        let preview = [SectionId::Projects, SectionId::FeatureFlags, SectionId::Motion];
        let defaults = Settings::default();
        // A release build: no flag list, and a flagged feature stays hidden whatever the file says.
        let asked_on = Settings {
            feature_flags: [("project_folders".to_string(), true), ("motion".to_string(), true)].into(),
            ..Settings::default()
        };
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
        // Motion has its own flag: on in a dev build, and switched off on its own.
        assert!(SectionId::Motion.is_visible_in(&defaults, DEV));
        assert!(SectionId::Motion.is_visible_in(&off, DEV), "project folders' flag is not its flag");
        let motion_off =
            Settings { feature_flags: [("motion".to_string(), false)].into(), ..Settings::default() };
        assert!(!SectionId::Motion.is_visible_in(&motion_off, DEV));
        assert!(SectionId::Projects.is_visible_in(&motion_off, DEV));
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
        assert!(!SectionId::Motion.is_visible_in(&defaults, UNSTABLE));
        assert!(SectionId::Motion.is_visible_in(&asked_on, UNSTABLE));
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
    fn each_choice_of_the_motion_object_has_its_own_changed_mark_and_reset() {
        use crate::view::motion::{FoldMotion, MotionChoices, StatusMotion};
        let settings = Settings {
            motion: MotionChoices {
                fold: FoldMotion::Drawer,
                status: StatusMotion::Wash,
                ..Default::default()
            },
            ..Settings::default()
        };
        let values = Values::of(&settings);
        let row = |id: &str| entry(id).expect("row").2;
        assert!(values.entry_changed(row("motion-fold")) && values.entry_changed(row("motion-status")));
        assert!(!values.entry_changed(row("motion-insert")), "its sibling choices are at their defaults");
        assert!(!values.entry_changed(row("motion-column")));
        assert_eq!(values.changed_keys(), ["motion"], "reset all puts the object back whole");

        let back = reset(&settings, row("motion-fold").keys);
        assert_eq!(back.motion.fold, FoldMotion::Tuck);
        assert_eq!(back.motion.status, StatusMotion::Wash, "the other choice stays");
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
        // A choice inside an object is set by a project when the project sets the object, if the object
        // is one a project may set at all.
        let nested = Entry {
            id: "nested",
            label: wormhole_text::keys::SETTINGS_MOTION_INSERT,
            description: None,
            keywords: None,
            keys: &["density.nested"],
            feature: None,
            flag: None,
        };
        assert!(PROJECT_SETTINGS.contains(&"density"));
        assert!(set_by_project(&nested, &["density".into()]));
        assert!(!set_by_project(&nested, &["tab_width".into()]));
        let (_, _, insert) = entry("motion-insert").expect("row");
        assert!(!set_by_project(insert, &["motion".into()]), "motion is the person's own");
    }
}
