//! The Agents page's data and the one thing it does: which coding agents are on this Mac, the state of
//! Wormhole's status hooks in each (`agents::AgentsSnapshot`), and installing or removing them
//! (`integrations::Spec`). The rows are core's; the sentences are the catalogs'.
//!
//! Installing writes into an agent's own config (`~/.claude/settings.json`): the experiment never
//! touches the real one, so writing needs `WORMHOLE_AGENT_HOME` (a folder of the test's own) and
//! refuses without it. Reading is harmless and uses the same home.

use std::path::{Path, PathBuf};

use wormhole_core::agents::integrations::{self, State};
use wormhole_core::agents::{AGENTS, AgentsSnapshot};
use wormhole_core::platform::paths;
use wormhole_text::keys::{
    SETTINGS_AGENTS_INSTALLED_HOOKS_IN, SETTINGS_AGENTS_INSTALLED_HOOKS_INCOMPLETE_IN,
    SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_HOOKS_GO,
    SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_NO_HOOK, SETTINGS_AGENTS_STATUS_FROM_OUTPUT,
    SETTINGS_AGENTS_STATUS_HOOKS_AVAILABLE,
};
use wormhole_text::text;

use super::error::SettingsError;

/// The state of Wormhole's hooks in one agent.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum SettingsHookState {
    /// The agent has no hook mechanism: its status is read from the terminal's output.
    NoHooks,
    NotInstalled,
    Installed,
    /// Some of ours are missing (an upgrade added events): repair.
    Partial,
    /// The agent's config does not read; `reason` is core's sentence.
    Unreadable {
        reason: String,
    },
}

/// One agent's row.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsAgentRow {
    /// The command the person types (`claude`): the id `set_agent_hooks` takes.
    pub id: String,
    pub name: String,
    pub two_letter_mark: String,
    /// Its command is on `PATH` or in the usual install spots.
    pub on_path: bool,
    pub hooks: SettingsHookState,
    /// The line under the name, in the catalogs' words, with where the hooks go when there are any.
    pub detail: String,
    /// Where the hooks are (`~/.claude/settings.json`), to reveal.
    pub config_path: Option<String>,
    /// Something to do in the agent afterwards (Codex asks to trust new hooks once).
    pub note: Option<String>,
}

fn hook_state(state: Option<&Result<State, String>>) -> SettingsHookState {
    match state {
        None => SettingsHookState::NoHooks,
        Some(Ok(State::NotInstalled)) => SettingsHookState::NotInstalled,
        Some(Ok(State::Installed)) => SettingsHookState::Installed,
        Some(Ok(State::Partial)) => SettingsHookState::Partial,
        Some(Err(reason)) => SettingsHookState::Unreadable { reason: reason.clone() },
    }
}

/// The page's rows for the agents' configs under `home`, in the registry's order.
pub(crate) fn rows_in(home: &Path) -> Vec<SettingsAgentRow> {
    let snapshot = AgentsSnapshot::load(home);
    AGENTS
        .iter()
        .filter_map(|agent| {
            let row = snapshot.row(agent.id)?;
            let spec = integrations::spec(agent.id);
            let hooks = hook_state(row.hooks.as_ref());
            let detail = match (row.installed, &hooks) {
                (false, SettingsHookState::NoHooks) => text(SETTINGS_AGENTS_STATUS_FROM_OUTPUT).to_string(),
                (false, _) => text(SETTINGS_AGENTS_STATUS_HOOKS_AVAILABLE).to_string(),
                (true, SettingsHookState::Installed) => {
                    with_path(SETTINGS_AGENTS_INSTALLED_HOOKS_IN, spec, home)
                }
                (true, SettingsHookState::Partial) => {
                    with_path(SETTINGS_AGENTS_INSTALLED_HOOKS_INCOMPLETE_IN, spec, home)
                }
                (true, SettingsHookState::NoHooks) => {
                    text(SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_NO_HOOK).to_string()
                }
                (true, _) => text(SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_HOOKS_GO).to_string(),
            };
            let installed_hooks = matches!(hooks, SettingsHookState::Installed | SettingsHookState::Partial);
            Some(SettingsAgentRow {
                id: agent.command.to_string(),
                name: agent.name.to_string(),
                two_letter_mark: agent.monogram.to_string(),
                on_path: row.installed,
                detail,
                config_path: spec.map(|spec| spec.path(home).to_string_lossy().into_owned()),
                // The note is for after an install, so it shows once hooks exist.
                note: spec.and_then(|spec| spec.note).filter(|_| installed_hooks).map(str::to_string),
                hooks,
            })
        })
        .collect()
}

fn with_path(message: wormhole_text::Key, spec: Option<&integrations::Spec>, home: &Path) -> String {
    match spec {
        Some(spec) => format!("{} {}", text(message), spec.display_path(home)),
        None => text(message).to_string(),
    }
}

/// Installs or removes the hooks of the agent whose command is `agent` under `home`.
pub(crate) fn set_hooks_in(home: &Path, agent: &str, install: bool) -> Result<(), SettingsError> {
    let found = AGENTS
        .iter()
        .find(|known| known.command == agent)
        .ok_or_else(|| SettingsError::Agents { reason: format!("no agent `{agent}`") })?;
    let spec = integrations::spec(found.id)
        .ok_or_else(|| SettingsError::Agents { reason: format!("{} has no hooks to set", found.name) })?;
    let done = if install { spec.install(home) } else { spec.uninstall(home) };
    done.map_err(|reason| SettingsError::Agents { reason })
}

/// Where hooks may be written: `WORMHOLE_AGENT_HOME`, never the real home.
fn agent_home_for_writing() -> Result<PathBuf, SettingsError> {
    let home = std::env::var_os("WORMHOLE_AGENT_HOME")
        .map(PathBuf::from)
        .filter(|home| !home.as_os_str().is_empty() && Some(home) != paths::home().as_ref());
    home.ok_or_else(|| SettingsError::Agents {
        reason:
            "WORMHOLE_AGENT_HOME is not set to a folder of its own: hooks are not written into the real home"
                .into(),
    })
}

/// Every agent with its hooks' state, read under `WORMHOLE_AGENT_HOME` or the home folder. Stats every
/// `PATH` entry and parses the agents' configs: call it off the main thread, and again only after an
/// install or a removal.
#[uniffi::export]
pub fn settings_agent_rows() -> Vec<SettingsAgentRow> {
    rows_in(&integrations::home())
}

/// Installs (`install`) or removes Wormhole's status hooks in the agent whose command is `agent`.
/// Merges into the agent's config and keeps a backup, as core does. Refuses unless `WORMHOLE_AGENT_HOME`
/// names a folder of its own.
#[uniffi::export]
pub fn settings_set_agent_hooks(agent: String, install: bool) -> Result<(), SettingsError> {
    set_hooks_in(&agent_home_for_writing()?, &agent, install)
}

#[cfg(test)]
mod tests {
    use super::{SettingsHookState, rows_in, set_hooks_in};

    fn state_of(home: &std::path::Path, agent: &str) -> SettingsHookState {
        rows_in(home).into_iter().find(|row| row.id == agent).expect("a row").hooks
    }

    #[test]
    fn hooks_are_installed_repaired_and_removed_inside_the_home_given() {
        let home = tempfile::tempdir().expect("temp home");
        assert_eq!(state_of(home.path(), "claude"), SettingsHookState::NotInstalled);

        set_hooks_in(home.path(), "claude", true).expect("installs");
        assert_eq!(state_of(home.path(), "claude"), SettingsHookState::Installed);
        assert!(home.path().join(".claude/settings.json").exists(), "written under the home given");
        assert_eq!(
            state_of(home.path(), "codex"),
            SettingsHookState::NotInstalled,
            "the others are left alone"
        );

        set_hooks_in(home.path(), "claude", false).expect("removes");
        assert_eq!(state_of(home.path(), "claude"), SettingsHookState::NotInstalled);
    }

    #[test]
    fn an_agent_that_does_not_exist_or_has_no_hooks_is_refused() {
        let home = tempfile::tempdir().expect("temp home");
        assert!(set_hooks_in(home.path(), "no-such-agent", true).is_err());
        let rows = rows_in(home.path());
        let without = rows.iter().find(|row| row.hooks == SettingsHookState::NoHooks).expect("one without");
        assert!(set_hooks_in(home.path(), &without.id, true).is_err());
        assert!(!without.detail.is_empty());
    }

    #[test]
    fn the_rows_cover_every_agent_with_words() {
        let home = tempfile::tempdir().expect("temp home");
        let rows = rows_in(home.path());
        assert_eq!(rows.len(), wormhole_core::agents::AGENTS.len());
        assert!(rows.iter().all(|row| !row.name.is_empty() && !row.detail.is_empty()));
    }
}
