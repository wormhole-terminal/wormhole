//! The features of Wormhole as data: which there are, what each is called, whether a person can do
//! without it, and which panel tabs it brings. The window asks this table (through [`FeatureMask`])
//! instead of naming a feature or its flag; the app's side of the table (`window::feature`) holds
//! what a feature does and refers to these ids.
//!
//! This is not [`FeatureFlag`]: a flag gates a feature that is still being built, and graduates
//! away. A feature here stays; it may name the flag that gates it for now.

use super::Settings;
use super::features::{FeatureFlag, FlagMode, enabled_in};
use super::tab::TabId;

/// A feature of the app. The order is the table's ([`FEATURES`]) and the bit each takes in a
/// [`FeatureMask`], so a new one goes last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FeatureId {
    /// What changed in the terminal's repository: the file list, the diff and its review comments.
    Changes,
    /// The repository's commits and the branch picker.
    History,
    /// A terminal's notes and next steps.
    Notes,
    /// The coding agents' plan limits.
    Usage,
    /// The `.wormhole/` project folder.
    Project,
    /// A terminal in a worktree of its own.
    Worktrees,
}

/// Whether a person can do without a feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureKind {
    /// Part of the app: always on.
    Always,
    /// Can be switched off. `default_on` is what it is until the person says otherwise.
    Optional { default_on: bool },
}

/// What is known of a feature without any UI.
#[derive(Clone, Copy, Debug)]
pub struct FeatureInfo {
    pub id: FeatureId,
    /// The name devctl and the control commands call it, and the key under which a person's own
    /// switch will be kept.
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub kind: FeatureKind,
    /// While the feature is unfinished: the flag that gates it.
    pub gate: Option<FeatureFlag>,
    /// The panel tabs it brings, in the order the panel lists them.
    pub tabs: &'static [TabId],
}

/// Every feature, indexed by [`FeatureId`].
pub const FEATURES: [FeatureInfo; 6] = [
    FeatureInfo {
        id: FeatureId::Changes,
        key: "changes",
        title: "Changes",
        description: "The files changed in a terminal's repository, their diffs and review comments.",
        kind: FeatureKind::Always,
        gate: None,
        tabs: &[TabId::Changes],
    },
    FeatureInfo {
        id: FeatureId::History,
        key: "history",
        title: "History",
        description: "The repository's commits, and the branch picker.",
        kind: FeatureKind::Optional { default_on: true },
        gate: None,
        tabs: &[TabId::History],
    },
    FeatureInfo {
        id: FeatureId::Notes,
        key: "notes",
        title: "Notes and tasks",
        description: "Notes and next steps for each terminal.",
        kind: FeatureKind::Optional { default_on: true },
        gate: Some(FeatureFlag::NotesAndTasks),
        tabs: &[TabId::Notes],
    },
    FeatureInfo {
        id: FeatureId::Usage,
        key: "usage",
        title: "Usage",
        description: "How much of their plan the coding agents in view have used.",
        kind: FeatureKind::Always,
        gate: None,
        tabs: &[TabId::Usage],
    },
    FeatureInfo {
        id: FeatureId::Project,
        key: "project",
        title: "Project folders",
        description: "The .wormhole project folder: trust, per-project settings, actions and colours.",
        kind: FeatureKind::Optional { default_on: true },
        gate: Some(FeatureFlag::ProjectFolders),
        tabs: &[TabId::Project],
    },
    FeatureInfo {
        id: FeatureId::Worktrees,
        key: "worktrees",
        title: "Worktrees",
        description: "A terminal in a new worktree of its repository.",
        kind: FeatureKind::Always,
        gate: None,
        tabs: &[],
    },
];

impl FeatureId {
    /// Every feature, in the table's order.
    pub const ALL: [FeatureId; FEATURES.len()] =
        [Self::Changes, Self::History, Self::Notes, Self::Usage, Self::Project, Self::Worktrees];

    /// What the table says of this feature.
    pub const fn info(self) -> &'static FeatureInfo {
        &FEATURES[self as usize]
    }

    /// The feature called `key` ([`FeatureInfo::key`]).
    pub fn from_key(key: &str) -> Option<Self> {
        FEATURES.iter().find(|info| info.key == key).map(|info| info.id)
    }

    /// The feature that brings `tab`.
    pub fn of_tab(tab: TabId) -> Option<Self> {
        FEATURES.iter().find(|info| info.tabs.contains(&tab)).map(|info| info.id)
    }
}

/// Which features are on: one bit each. Reading it is a mask and a compare, with no lookup by name
/// and no allocation, which is why the settings keep one worked out ([`Self::resolve`]) for the
/// frame to ask.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureMask(u16);

impl FeatureMask {
    /// Whether `id` is on. An always-on feature is set whatever the person's settings say.
    pub const fn has(self, id: FeatureId) -> bool {
        self.0 & (1 << id as u8) != 0
    }

    /// Which features `settings` leave on: an always-on one whatever they say; an optional one
    /// when its gate (if it has one) passes in this build and the person's own switch is on. That
    /// switch is not in `config.json` yet, so it is the feature's default.
    pub fn resolve(settings: &Settings) -> Self {
        Self::resolve_in(settings, settings.flag_mode())
    }

    /// [`Self::resolve`] for a given [`FlagMode`], so the tests can ask what a release does.
    pub fn resolve_in(settings: &Settings, mode: FlagMode) -> Self {
        let mut bits = 0;
        for info in &FEATURES {
            let on = match info.kind {
                FeatureKind::Always => true,
                FeatureKind::Optional { default_on } => {
                    default_on && info.gate.is_none_or(|flag| enabled_in(flag, &settings.feature_flags, mode))
                }
            };
            bits |= u16::from(on) << info.id as u8;
        }
        Self(bits)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn settings_with(flags: &[(&str, bool)]) -> Settings {
        Settings {
            feature_flags: flags.iter().map(|(key, on)| (key.to_string(), *on)).collect::<BTreeMap<_, _>>(),
            ..Settings::default()
        }
    }

    #[test]
    fn the_table_is_indexed_by_id_and_every_key_is_distinct() {
        for (at, info) in FEATURES.iter().enumerate() {
            assert_eq!(info.id as usize, at, "{} is out of order", info.key);
            assert_eq!(FeatureId::ALL[at], info.id);
            assert_eq!(info.id.info().key, info.key);
            assert_eq!(FeatureId::from_key(info.key), Some(info.id));
            assert!(!info.title.is_empty() && !info.description.is_empty(), "{}", info.key);
        }
        assert_eq!(FeatureId::from_key("nope"), None);
        assert!(FEATURES.len() <= 16, "a mask is 16 bits");
    }

    #[test]
    fn every_tab_belongs_to_exactly_one_feature() {
        for tab in TabId::ALL {
            let owners = FEATURES.iter().filter(|info| info.tabs.contains(&tab)).count();
            assert_eq!(owners, 1, "{tab:?}");
            assert!(FeatureId::of_tab(tab).is_some());
        }
    }

    #[test]
    fn a_gate_is_a_flag_of_an_optional_feature() {
        for info in &FEATURES {
            if info.gate.is_some() {
                assert!(matches!(info.kind, FeatureKind::Optional { .. }), "{} is always on", info.key);
            }
        }
    }

    #[test]
    fn the_mask_matches_the_flags_the_frame_reads_today() {
        // Notes follow `NotesAndTasks` and Project `ProjectFolders`; nothing else is gated.
        let cases = [
            settings_with(&[]),
            settings_with(&[("notes_and_tasks", false)]),
            settings_with(&[("project_folders", false)]),
            settings_with(&[("notes_and_tasks", false), ("project_folders", false)]),
            settings_with(&[("notes_and_tasks", true), ("project_folders", true)]),
        ];
        for settings in cases {
            let mask = FeatureMask::resolve(&settings);
            assert_eq!(mask.has(FeatureId::Notes), settings.feature_enabled(FeatureFlag::NotesAndTasks));
            assert_eq!(mask.has(FeatureId::Project), settings.feature_enabled(FeatureFlag::ProjectFolders));
            for always in [FeatureId::Changes, FeatureId::History, FeatureId::Usage, FeatureId::Worktrees] {
                assert!(mask.has(always), "{always:?}");
            }
        }
    }

    #[test]
    fn a_release_build_leaves_the_unfinished_features_off_and_a_tester_can_turn_them_on() {
        let on = settings_with(&[("notes_and_tasks", true), ("project_folders", true)]);
        let release = FeatureMask::resolve_in(&on, FlagMode::Release);
        assert!(!release.has(FeatureId::Notes) && !release.has(FeatureId::Project), "the map is ignored");
        assert!(release.has(FeatureId::Changes) && release.has(FeatureId::History));
        let unstable = FeatureMask::resolve_in(&on, FlagMode::Unstable);
        assert!(unstable.has(FeatureId::Notes) && unstable.has(FeatureId::Project));
        let untouched = FeatureMask::resolve_in(&settings_with(&[]), FlagMode::Unstable);
        assert!(!untouched.has(FeatureId::Notes), "off until turned on");
    }

    #[test]
    fn the_mask_reads_each_bit_alone() {
        let none = FeatureMask::default();
        assert!(FeatureId::ALL.iter().all(|&id| !none.has(id)));
        let mask = FeatureMask::resolve(&settings_with(&[("notes_and_tasks", false)]));
        assert!(!mask.has(FeatureId::Notes));
        assert!(mask.has(FeatureId::Project) && mask.has(FeatureId::Changes));
    }
}
