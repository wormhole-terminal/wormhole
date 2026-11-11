//! What a launch brings forward from a version before the rename (`andromeda`), as the GPUI app does at launch
//! (`main.rs` for the data folder, `app/launch.rs` for the hooks): the data folder's move and the agent hooks installed
//! under the old name. The Swift app only opens `Wormhole-swift*` data folders, which are new, so there is nothing to bring
//! over yet and its launch runs these only the day it opens the real folder; core's own rule already keeps the move away from
//! any run with `WORMHOLE_DATA_DIR` set.

use std::path::Path;

use wormhole_core::agents::integrations;
use wormhole_core::storage::store::migrate_legacy_data;

/// What was brought forward, for the log.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct LegacyUpgrades {
    /// The data folder moved from the old name to the new (its path); none when there was none to move.
    pub moved_data_folder: Option<String>,
    /// The agents whose old hooks were replaced with today's.
    pub hooks_replaced: Vec<String>,
    /// What could not be done, said.
    pub problems: Vec<String>,
}

/// The data folder of the old name moves to the new one, once, and only when no `WORMHOLE_DATA_DIR` names another (core's
/// rule). Before anything opens a file in the data folder.
#[uniffi::export]
pub fn legacy_move_data_folder() -> LegacyUpgrades {
    let mut done = LegacyUpgrades::default();
    match migrate_legacy_data() {
        Ok(moved) => done.moved_data_folder = moved.map(|path| path.display().to_string()),
        Err(err) => done.problems.push(format!("the earlier version's data folder: {err:#}")),
    }
    done
}

/// Agent hooks an earlier version installed under its old names are replaced with today's, for each agent that has any in the
/// home `agent_home` (else the person's, or `WORMHOLE_AGENT_HOME`); an agent without any is left alone.
#[uniffi::export]
pub fn legacy_replace_agent_hooks(agent_home: Option<String>) -> LegacyUpgrades {
    let home = agent_home.map_or_else(integrations::home, |home| Path::new(&home).to_path_buf());
    let mut done = LegacyUpgrades::default();
    for (agent, result) in integrations::upgrade_legacy(&home) {
        match result {
            Ok(()) => done.hooks_replaced.push(format!("{agent:?}")),
            Err(err) => done.problems.push(format!("the hooks of {agent:?}: {err}")),
        }
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Claude Code settings file with a hook under the old codename's marker.
    fn write_legacy_claude(home: &Path, command: &str) {
        let dir = home.join(".claude");
        std::fs::create_dir_all(&dir).expect("folder");
        let settings = serde_json::json!({
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": command }] }] }
        });
        std::fs::write(dir.join("settings.json"), settings.to_string()).expect("settings");
    }

    #[test]
    fn old_hooks_are_replaced_in_the_home_given_and_a_home_without_any_is_left_alone() {
        let scratch = tempfile::tempdir().expect("a scratch home");
        let marker = "ANDROMEDA_BIN";
        write_legacy_claude(scratch.path(), "\"$ANDROMEDA_BIN\" hook --agent claude Stop");
        let done = legacy_replace_agent_hooks(Some(scratch.path().display().to_string()));
        assert_eq!(done.hooks_replaced, ["Claude"], "{done:?}");
        assert!(done.problems.is_empty(), "{done:?}");
        let text = std::fs::read_to_string(scratch.path().join(".claude/settings.json")).expect("settings");
        assert!(!text.contains(marker), "the old command is gone: {text}");
        // Run again: nothing old is left to replace.
        assert!(
            legacy_replace_agent_hooks(Some(scratch.path().display().to_string())).hooks_replaced.is_empty()
        );
        let empty = tempfile::tempdir().expect("an empty home");
        assert_eq!(
            legacy_replace_agent_hooks(Some(empty.path().display().to_string())),
            LegacyUpgrades::default()
        );
    }

    #[test]
    fn the_data_folder_does_not_move_in_a_run_with_its_own_data_folder() {
        // Every run of the experiment names its data folder (`WORMHOLE_DATA_DIR`): core's rule leaves the real one alone.
        crate::isolate_for_tests();
        assert_eq!(legacy_move_data_folder(), LegacyUpgrades::default());
    }
}
