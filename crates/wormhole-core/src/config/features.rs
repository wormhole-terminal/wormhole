//! App-wide feature flags: features still in progress that are shown and switchable only in a
//! development build (`cargo run`, `scripts/dev.sh`, the tests) or a release on the Unstable update
//! channel, where testers try them in a signed build. Any other release can't turn an unfinished
//! feature on by editing `config.json`: there each flag takes its release default, and the flag UI
//! isn't built at all.
//!
//! This is not the per-project [`crate::project::Feature`] switches (which parts of a trusted
//! `.wormhole/` a person accepts). A `FeatureFlag` gates whole features of Wormhole itself; the
//! first one gates the entire project-folder feature and its sidebar module, another the first-run
//! welcome that runs in the terminal itself, another Liquid Glass.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::update::UpdateChannel;

/// A feature of Wormhole kept behind a flag until it is finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
    /// The `.wormhole/` project folders as a whole: discovery, trust, the settings/theme/actions/
    /// layout overlays, shared notes, the sidebar's Project tab and its "Create project folder",
    /// and project-owned folder colours.
    ProjectFolders,
    /// The first-run welcome as a program in the first terminal (`Wormhole welcome`), with the
    /// window's sidebar and chrome showing, instead of the intro window that stands in for the
    /// terminal. Off, the first launch and Help → Welcome to Wormhole open the intro window.
    TerminalWelcome,
    /// Notes and next steps (tasks) of each terminal: the right panel's Notes tab, its actions
    /// (New Note, Stash Prompt, Show Notes), the open-items count in the sidebar and in the close
    /// questions, and the project notes sync. What was written stays in the database.
    NotesAndTasks,
    /// Terminals that outlive the app: each runs under its own small `wormhole-host` process, so
    /// quitting, crashing or updating Wormhole detaches from them and the next launch attaches
    /// again to the same screen and the same running programs (`repo-docs/docs/session-host.md`).
    PersistentTerminals,
    /// Liquid Glass under the sidebar's and the panel's cards on macOS 26 and later, where the
    /// system allows it (not with Reduce Transparency or Increase Contrast). Off, every surface
    /// paints itself. It was a setting (`liquid_glass`) until 2026-10-25.
    LiquidGlass,
    /// Driving Wormhole from scripts and coding agents beyond `list`, `status`, `notify`, `send-text`
    /// and `new-terminal`: `wh events` and, as the phases land, reading terminals, keys and mouse,
    /// windows and splits, waiting and running, and `wh mcp` (`repo-docs/docs/automation.md`). Off,
    /// the new actions answer that they are not in this build yet.
    Automation,
}

impl FeatureFlag {
    pub const ALL: [FeatureFlag; 6] = [
        Self::ProjectFolders,
        Self::TerminalWelcome,
        Self::NotesAndTasks,
        Self::PersistentTerminals,
        Self::LiquidGlass,
        Self::Automation,
    ];

    /// The key this flag has under `feature_flags` in `config.json`.
    pub fn key(self) -> &'static str {
        match self {
            Self::ProjectFolders => "project_folders",
            Self::TerminalWelcome => "terminal_welcome",
            Self::NotesAndTasks => "notes_and_tasks",
            Self::PersistentTerminals => "persistent_terminals",
            Self::LiquidGlass => "liquid_glass",
            Self::Automation => "automation",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }

    /// The name shown on its switch and in the preview notice.
    pub fn title(self) -> &'static str {
        match self {
            Self::ProjectFolders => "Project folders",
            Self::TerminalWelcome => "Terminal welcome",
            Self::NotesAndTasks => "Notes and tasks",
            Self::PersistentTerminals => "Persistent terminals",
            Self::LiquidGlass => "Liquid Glass",
            Self::Automation => "Automation",
        }
    }

    /// One line under the switch: what turning it on brings.
    pub fn description(self) -> &'static str {
        match self {
            Self::ProjectFolders => {
                "The .wormhole project folder: trust, per-project settings and theme, actions, \
                 shared notes, the sidebar's Project tab and project folder colours."
            }
            Self::TerminalWelcome => {
                "The first-run welcome as a program in the first terminal, with the sidebar showing, \
                 instead of the intro window."
            }
            Self::NotesAndTasks => {
                "Notes and next steps for each terminal: the right panel's Notes tab, their keys and \
                 the open count in the sidebar."
            }
            Self::PersistentTerminals => {
                "Terminals keep running when Wormhole quits or updates, and the next launch picks them \
                 up again with their screen and programs."
            }
            Self::LiquidGlass => {
                "Glass under the sidebar's and the panel's cards on macOS 26 and later, unless Reduce \
                 Transparency or Increase Contrast is on. Off: every surface painted."
            }
            Self::Automation => {
                "The command line and coding agents watch and drive Wormhole: wh events, and later \
                 reading terminals, keys, windows and splits, waiting and running."
            }
        }
    }

    /// What a shipped release build does with this flag, whatever `config.json` says. A feature
    /// that is still being built ships off until it graduates to on here.
    pub fn released(self) -> bool {
        match self {
            Self::ProjectFolders => false,
            // Until the user has seen it and approved: the intro window stays the first run.
            Self::TerminalWelcome => false,
            // Hidden for now; what people wrote stays in the database.
            Self::NotesAndTasks => false,
            // Off until the host has been through its phases and the user has approved it.
            Self::PersistentTerminals => false,
            // Unfinished: glass behind every surface still needs its polish.
            Self::LiquidGlass => false,
            // Until its last phase has landed and the user has approved it.
            Self::Automation => false,
        }
    }
}

/// Whether this is a development build: `cargo run`, `scripts/dev.sh` and the tests, where
/// `debug_assertions` is on. A notarised release is a release build, so this is off and neither
/// the feature-flag UI nor the `config.json` overrides exist.
pub fn dev_build() -> bool {
    cfg!(debug_assertions)
}

/// Who decides whether a flag is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagMode {
    /// A development build: the `feature_flags` map, and a flag with no entry there is **on**, so
    /// a dev build behaves as it always has and a person must switch a flag off to see what a
    /// release user gets.
    Dev,
    /// A release on the Unstable channel: the map too, but a flag with no entry is what the
    /// release does, so switching to Unstable changes nothing until a tester turns a flag on.
    Unstable,
    /// Any other release: the flag's release default, whatever the map says.
    Release,
}

impl FlagMode {
    pub fn of(dev_build: bool, channel: UpdateChannel) -> Self {
        match (dev_build, channel) {
            (true, _) => Self::Dev,
            (false, UpdateChannel::Unstable) => Self::Unstable,
            (false, UpdateChannel::Stable) => Self::Release,
        }
    }

    /// Whether the switches are shown (Settings → Feature flags) and the map is read.
    pub fn shows_switches(self) -> bool {
        self != Self::Release
    }

    /// What `flag` is with no entry in the map.
    pub fn default_for(self, flag: FeatureFlag) -> bool {
        match self {
            Self::Dev => true,
            Self::Unstable | Self::Release => flag.released(),
        }
    }
}

/// Whether `flag` is on, from the `feature_flags` map of `config.json` and `mode` (see
/// [`FlagMode`]).
pub fn enabled_in(flag: FeatureFlag, flags: &BTreeMap<String, bool>, mode: FlagMode) -> bool {
    if mode.shows_switches() {
        flags.get(flag.key()).copied().unwrap_or(mode.default_for(flag))
    } else {
        flag.released()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEV: FlagMode = FlagMode::Dev;
    const RELEASE: FlagMode = FlagMode::Release;

    #[test]
    fn a_dev_build_defaults_a_flag_on_and_obeys_the_map() {
        let empty = BTreeMap::new();
        assert!(enabled_in(FeatureFlag::ProjectFolders, &empty, DEV), "on by default in a dev build");
        let off = BTreeMap::from([("project_folders".to_string(), false)]);
        assert!(!enabled_in(FeatureFlag::ProjectFolders, &off, DEV), "the map can switch it off");
        let on = BTreeMap::from([("project_folders".to_string(), true)]);
        assert!(enabled_in(FeatureFlag::ProjectFolders, &on, DEV));
    }

    #[test]
    fn a_release_build_ignores_the_map_and_takes_the_release_default() {
        let on = BTreeMap::from([("project_folders".to_string(), true)]);
        // Even with the file saying on, a release build keeps the feature off until it graduates.
        assert!(!enabled_in(FeatureFlag::ProjectFolders, &on, RELEASE));
        assert_eq!(
            enabled_in(FeatureFlag::ProjectFolders, &on, RELEASE),
            FeatureFlag::ProjectFolders.released()
        );
    }

    #[test]
    fn the_terminal_welcome_is_on_in_a_dev_build_and_off_in_a_release() {
        assert!(!FeatureFlag::TerminalWelcome.released());
        assert!(enabled_in(FeatureFlag::TerminalWelcome, &BTreeMap::new(), DEV));
        let on = BTreeMap::from([("terminal_welcome".to_string(), true)]);
        assert!(!enabled_in(FeatureFlag::TerminalWelcome, &on, RELEASE), "a release ignores the map");
        assert_eq!(FeatureFlag::from_key("terminal_welcome"), Some(FeatureFlag::TerminalWelcome));
    }

    #[test]
    fn notes_and_tasks_has_its_key_and_ships_hidden() {
        assert_eq!(FeatureFlag::NotesAndTasks.key(), "notes_and_tasks");
        assert_eq!(FeatureFlag::from_key("notes_and_tasks"), Some(FeatureFlag::NotesAndTasks));
        assert!(!FeatureFlag::NotesAndTasks.released());
        assert!(enabled_in(FeatureFlag::NotesAndTasks, &BTreeMap::new(), DEV));
        let on = BTreeMap::from([("notes_and_tasks".to_string(), true)]);
        assert!(!enabled_in(FeatureFlag::NotesAndTasks, &on, RELEASE));
    }

    #[test]
    fn persistent_terminals_is_on_in_a_dev_build_and_off_in_a_release() {
        let flag = FeatureFlag::PersistentTerminals;
        assert!(!flag.released());
        assert_eq!(flag.key(), "persistent_terminals");
        assert!(enabled_in(flag, &BTreeMap::new(), DEV));
        let on = BTreeMap::from([("persistent_terminals".to_string(), true)]);
        assert!(!enabled_in(flag, &on, RELEASE), "a release ignores the map");
        assert!(!enabled_in(flag, &BTreeMap::new(), FlagMode::Unstable), "off until a tester turns it on");
        assert!(enabled_in(flag, &on, FlagMode::Unstable));
        let off = BTreeMap::from([("persistent_terminals".to_string(), false)]);
        assert!(!enabled_in(flag, &off, DEV));
        assert_eq!(FeatureFlag::from_key("persistent_terminals"), Some(flag));
    }

    #[test]
    fn automation_is_on_in_a_dev_build_and_off_in_a_release() {
        let flag = FeatureFlag::Automation;
        assert_eq!(flag.key(), "automation");
        assert!(!flag.released());
        assert!(enabled_in(flag, &BTreeMap::new(), DEV));
        let on = BTreeMap::from([("automation".to_string(), true)]);
        assert!(!enabled_in(flag, &on, RELEASE), "a release ignores the map");
        assert!(enabled_in(flag, &on, FlagMode::Unstable));
        assert_eq!(FeatureFlag::from_key("automation"), Some(flag));
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
    fn a_retired_flags_key_in_config_json_changes_nothing() {
        // The own engine was a flag until it became the only engine; a config.json from then may
        // still carry its key.
        let old = BTreeMap::from([
            ("own_engine".to_string(), false),
            ("sidebar_filter".to_string(), false),
            ("project_folders".to_string(), false),
        ]);
        assert_eq!(FeatureFlag::from_key("own_engine"), None);
        assert_eq!(FeatureFlag::from_key("sidebar_filter"), None, "the sidebar's funnel is gone");
        assert!(!enabled_in(FeatureFlag::ProjectFolders, &old, DEV));
        assert!(enabled_in(FeatureFlag::TerminalWelcome, &old, DEV));
    }

    #[test]
    fn unstable_reads_the_map_but_starts_from_what_a_release_does() {
        let unstable = FlagMode::of(false, UpdateChannel::Unstable);
        assert_eq!(unstable, FlagMode::Unstable);
        assert!(unstable.shows_switches());
        let empty = BTreeMap::new();
        assert!(!enabled_in(FeatureFlag::ProjectFolders, &empty, unstable), "off until turned on");
        let on = BTreeMap::from([("project_folders".to_string(), true)]);
        assert!(enabled_in(FeatureFlag::ProjectFolders, &on, unstable), "a tester can turn it on");
        assert_eq!(FlagMode::of(false, UpdateChannel::Stable), FlagMode::Release);
        assert!(!FlagMode::Release.shows_switches());
        assert_eq!(FlagMode::of(true, UpdateChannel::Stable), FlagMode::Dev);
    }

    #[test]
    fn keys_round_trip() {
        for flag in FeatureFlag::ALL {
            assert_eq!(FeatureFlag::from_key(flag.key()), Some(flag));
        }
        assert_eq!(FeatureFlag::from_key("nope"), None);
    }
}
