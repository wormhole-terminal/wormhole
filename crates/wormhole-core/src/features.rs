//! App-wide feature flags: features still in progress that are shown and switchable only in a
//! development build (`cargo run`, `scripts/dev.sh`, the tests). A shipped release can't turn an
//! unfinished feature on by editing `config.json`: there each flag takes its release default,
//! and the flag UI isn't built at all.
//!
//! This is not the per-project [`crate::project::Feature`] switches (which parts of a trusted
//! `.wormhole/` a person accepts). A `FeatureFlag` gates whole features of Wormhole itself; the
//! first one gates the entire project-folder feature and its sidebar module, another the sidebar's
//! funnel.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A feature of Wormhole kept behind a flag until it is finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
    /// The `.wormhole/` project folders as a whole: discovery, trust, the settings/theme/actions/
    /// layout overlays, shared notes, the sidebar's Project tab and its "Create project folder",
    /// and project-owned folder colours.
    ProjectFolders,
    /// The sidebar's funnel: the Terminals header's button and its popover (search, Show, Hide
    /// idle, Group by, Density, Row details) and the "Filtered" chip. With it off the list is
    /// never filtered, whatever a window saved.
    SidebarFilter,
}

impl FeatureFlag {
    pub const ALL: [FeatureFlag; 2] = [Self::ProjectFolders, Self::SidebarFilter];

    /// The key this flag has under `feature_flags` in `config.json`.
    pub fn key(self) -> &'static str {
        match self {
            Self::ProjectFolders => "project_folders",
            Self::SidebarFilter => "sidebar_filter",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }

    /// The name shown on its switch and in the preview notice.
    pub fn title(self) -> &'static str {
        match self {
            Self::ProjectFolders => "Project folders",
            Self::SidebarFilter => "Sidebar filter",
        }
    }

    /// One line under the switch: what turning it on brings.
    pub fn description(self) -> &'static str {
        match self {
            Self::ProjectFolders => {
                "The .wormhole project folder: trust, per-project settings and theme, actions, \
                 shared notes, the sidebar's Project tab and project folder colours."
            }
            Self::SidebarFilter => {
                "The funnel in the Terminals header and its popover (search, Show, Hide idle, Group by, \
                 Density, Row details), and the \"Filtered\" chip."
            }
        }
    }

    /// What a shipped release build does with this flag, whatever `config.json` says. A feature
    /// that is still being built ships off until it graduates to on here.
    pub fn released(self) -> bool {
        match self {
            Self::ProjectFolders => false,
            Self::SidebarFilter => false,
        }
    }
}

/// Whether this is a development build: `cargo run`, `scripts/dev.sh` and the tests, where
/// `debug_assertions` is on. A notarised release is a release build, so this is off and neither
/// the feature-flag UI nor the `config.json` overrides exist.
pub fn dev_build() -> bool {
    cfg!(debug_assertions)
}

/// Whether `flag` is on, from the `feature_flags` map of `config.json` and whether this is a dev
/// build. A dev build reads the map and a flag with no entry there defaults **on**, so a dev
/// build behaves as it always has and a person must switch a flag off to see what a release user
/// gets. A release build ignores the map and uses the flag's release default.
pub fn enabled_in(flag: FeatureFlag, flags: &BTreeMap<String, bool>, dev: bool) -> bool {
    if dev { flags.get(flag.key()).copied().unwrap_or(true) } else { flag.released() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dev_build_defaults_a_flag_on_and_obeys_the_map() {
        let empty = BTreeMap::new();
        assert!(enabled_in(FeatureFlag::ProjectFolders, &empty, true), "on by default in a dev build");
        let off = BTreeMap::from([("project_folders".to_string(), false)]);
        assert!(!enabled_in(FeatureFlag::ProjectFolders, &off, true), "the map can switch it off");
        let on = BTreeMap::from([("project_folders".to_string(), true)]);
        assert!(enabled_in(FeatureFlag::ProjectFolders, &on, true));
    }

    #[test]
    fn a_release_build_ignores_the_map_and_takes_the_release_default() {
        let on = BTreeMap::from([("project_folders".to_string(), true)]);
        // Even with the file saying on, a release build keeps the feature off until it graduates.
        assert!(!enabled_in(FeatureFlag::ProjectFolders, &on, false));
        assert_eq!(
            enabled_in(FeatureFlag::ProjectFolders, &on, false),
            FeatureFlag::ProjectFolders.released()
        );
    }

    #[test]
    fn the_sidebar_filter_ships_off_and_a_dev_build_can_switch_it() {
        assert!(!FeatureFlag::SidebarFilter.released());
        let off = BTreeMap::from([("sidebar_filter".to_string(), false)]);
        assert!(!enabled_in(FeatureFlag::SidebarFilter, &off, true));
        assert!(enabled_in(FeatureFlag::SidebarFilter, &BTreeMap::new(), true));
        // Switching one flag leaves the other alone.
        assert!(enabled_in(FeatureFlag::ProjectFolders, &off, true));
    }

    #[test]
    fn every_flag_has_a_distinct_key_and_words() {
        for flag in FeatureFlag::ALL {
            assert!(!flag.title().is_empty() && !flag.description().is_empty());
            let same = FeatureFlag::ALL.iter().filter(|other| other.key() == flag.key()).count();
            assert_eq!(same, 1, "{flag:?}");
        }
    }

    #[test]
    fn keys_round_trip() {
        for flag in FeatureFlag::ALL {
            assert_eq!(FeatureFlag::from_key(flag.key()), Some(flag));
        }
        assert_eq!(FeatureFlag::from_key("nope"), None);
    }
}
