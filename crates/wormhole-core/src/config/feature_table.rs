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

impl FeatureMask {
    /// The actions that belong to a feature that is off (the keymap's rows are tagged with their
    /// feature's key): what the menus, the palette and the shortcut sheet leave out.
    pub fn hidden_actions(self) -> impl Iterator<Item = crate::keys::keymap::ActionId> {
        crate::keys::keymap::entries().iter().filter_map(move |entry| {
            let feature = FeatureId::from_key(entry.feature?)?;
            (!self.has(feature)).then_some(entry.id)
        })
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

    /// The mask with `id` switched on or off, as a person's own switch will do it. An always-on feature
    /// stays on: there is no switch for it.
    #[must_use]
    pub const fn with(self, id: FeatureId, on: bool) -> Self {
        let bit = 1u16 << id as u8;
        if on || matches!(FEATURES[id as usize].kind, FeatureKind::Always) {
            Self(self.0 | bit)
        } else {
            Self(self.0 & !bit)
        }
    }

    /// Which features `settings` leave on: an always-on one whatever they say; an optional one
    /// when its gate (if it has one) passes in this build and the person's own switch is on
    /// ([`Settings::feature_switched_on`]).
    pub fn resolve(settings: &Settings) -> Self {
        Self::resolve_in(settings, settings.flag_mode())
    }

    /// [`Self::resolve`] for a given [`FlagMode`], so the tests can ask what a release does.
    pub fn resolve_in(settings: &Settings, mode: FlagMode) -> Self {
        let mut mask = Self(0);
        for info in &FEATURES {
            let on = match info.kind {
                FeatureKind::Always => true,
                FeatureKind::Optional { .. } => {
                    settings.feature_switched_on(info.id)
                        && info.gate.is_none_or(|flag| enabled_in(flag, &settings.feature_flags, mode))
                }
            };
            mask = mask.with(info.id, on);
        }
        mask
    }
}

impl Settings {
    /// The person's own switch for feature `id`: what `features` says under its key, else the
    /// feature's default. An always-on feature has no switch and is on. This is not whether the
    /// feature is on: a feature still behind its flag is off whatever its switch says, so what to show
    /// asks [`FeatureMask`] (which reads both).
    pub fn feature_switched_on(&self, id: FeatureId) -> bool {
        let info = id.info();
        match info.kind {
            FeatureKind::Always => true,
            FeatureKind::Optional { default_on } => {
                self.features.get(info.key).copied().unwrap_or(default_on)
            }
        }
    }

    /// Switch an optional feature on or off; an always-on one has no switch. The default is kept as
    /// no entry, so a file says only what the person changed.
    pub fn switch_feature(&mut self, id: FeatureId, on: bool) {
        let info = id.info();
        let FeatureKind::Optional { default_on } = info.kind else { return };
        if on == default_on {
            self.features.remove(info.key);
        } else {
            self.features.insert(info.key.to_string(), on);
        }
    }
}

/// What `features` in a settings file says that does nothing: a key that names no feature, or one that
/// can't be switched off. `None` when all of it counts.
pub(crate) fn features_notice(features: &std::collections::BTreeMap<String, bool>) -> Option<String> {
    let idle: Vec<String> = features
        .keys()
        .filter(|key| {
            FEATURES
                .iter()
                .find(|info| info.key == key.as_str())
                .is_none_or(|info| info.kind == FeatureKind::Always)
        })
        .map(|key| format!("`{key}`"))
        .collect();
    (!idle.is_empty()).then(|| {
        let optional: Vec<&str> = FEATURES
            .iter()
            .filter(|info| matches!(info.kind, FeatureKind::Optional { .. }))
            .map(|info| info.key)
            .collect();
        format!("`features` → {} can't be switched (these can: {})", idle.join(", "), optional.join(", "))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn a_switch_is_kept_only_while_it_differs_from_the_default() {
        let mut settings = Settings::default();
        assert!(settings.feature_switched_on(FeatureId::Notes));
        settings.switch_feature(FeatureId::Notes, false);
        assert_eq!(settings.features.get("notes"), Some(&false));
        assert!(!settings.feature_switched_on(FeatureId::Notes));
        settings.switch_feature(FeatureId::Notes, true);
        assert!(settings.features.is_empty(), "the default is no entry");
        settings.switch_feature(FeatureId::Changes, false);
        assert!(
            settings.features.is_empty() && settings.feature_switched_on(FeatureId::Changes),
            "always on"
        );
        let read: Settings =
            crate::storage::jsonc::parse(r#"{ "features": { "history": false, "later": true } }"#)
                .expect("reads");
        assert!(!read.feature_switched_on(FeatureId::History));
        assert!(!FeatureMask::resolve(&read).has(FeatureId::History));
    }

    #[test]
    fn a_key_that_switches_nothing_is_said_so() {
        let said = |pairs: &[(&str, bool)]| {
            features_notice(&pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect())
        };
        assert_eq!(said(&[("notes", false), ("history", true)]), None);
        let notice = said(&[("changes", false), ("notes_and_tasks", false)]).expect("a notice");
        assert!(notice.contains("`changes`") && notice.contains("`notes_and_tasks`"), "{notice}");
        assert!(notice.contains("history, notes, project"), "{notice}");
    }

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
    fn every_tag_in_the_keymap_names_a_feature() {
        for entry in crate::keys::keymap::entries() {
            if let Some(key) = entry.feature {
                assert!(FeatureId::from_key(key).is_some(), "{:?} is tagged with {key:?}", entry.id);
            }
        }
    }

    #[test]
    fn an_off_feature_hides_the_actions_it_is_tagged_on() {
        use crate::keys::keymap::ActionId::{NewNote, ShowChanges, ShowNotes, StashPrompt};
        let off = FeatureMask::resolve_in(&settings_with(&[("notes_and_tasks", false)]), FlagMode::Dev);
        let hidden: Vec<_> = off.hidden_actions().collect();
        assert_eq!(hidden, [ShowNotes, NewNote, StashPrompt]);
        let on = FeatureMask::resolve_in(&settings_with(&[("notes_and_tasks", true)]), FlagMode::Dev);
        assert!(on.hidden_actions().next().is_none());
        assert!(!hidden.contains(&ShowChanges), "an always-on feature hides nothing");
    }

    /// The Project tab's line is the feature's, as History's and Notes' are: with project folders off (a
    /// release build today) the menus leave it out instead of showing it greyed.
    #[test]
    fn project_folders_off_hide_the_project_tabs_line() {
        use crate::keys::keymap::ActionId::ShowProject;
        let hides = |mask: FeatureMask| mask.hidden_actions().any(|id| id == ShowProject);
        assert!(hides(FeatureMask::resolve_in(&settings_with(&[("project_folders", false)]), FlagMode::Dev)));
        assert!(hides(FeatureMask::resolve_in(&settings_with(&[]), FlagMode::Release)), "as a release ships");
        assert!(!hides(FeatureMask::resolve_in(&settings_with(&[("project_folders", true)]), FlagMode::Dev)));
    }

    #[test]
    fn a_switch_turns_an_optional_feature_off_and_never_an_always_on_one() {
        let all = FeatureMask::resolve_in(&settings_with(&[]), FlagMode::Dev);
        let off = all.with(FeatureId::History, false);
        assert!(!off.has(FeatureId::History) && off.has(FeatureId::Changes) && off.has(FeatureId::Worktrees));
        assert_eq!(off.with(FeatureId::History, true), all, "and on again");
        assert_eq!(all.with(FeatureId::Changes, false), all, "no switch for an always-on feature");
        assert_eq!(all.with(FeatureId::Worktrees, false), all);
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
