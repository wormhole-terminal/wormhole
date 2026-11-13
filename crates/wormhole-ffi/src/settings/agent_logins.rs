//! The profiles' own logins of an agent on the Agents page (`sections/agents.rs` `LoginRow`): each is a second place
//! for the status hooks, with its folder, the hooks' state there and the one button that fits. What is in a login folder
//! is core's (`Spec::state_in`, `install_in`, `uninstall_in`); the profile's folders are the profile list's.
//!
//! Like the default location, hooks are only written under `WORMHOLE_AGENT_HOME` (see `agents`): the experiment never
//! touches a real agent folder, so a login folder outside it is refused.

use std::path::{Path, PathBuf};

use wormhole_core::agents::AGENTS;
use wormhole_core::agents::integrations::{self, State};
use wormhole_core::platform::paths;
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::view::sidebar::display_path;
use wormhole_text::keys::{
    SETTINGS_AGENTS_HOOKS_INCOMPLETE, SETTINGS_AGENTS_NO_HOOKS, SETTINGS_AGENTS_STATUS_HOOKS_ON,
    settings_agents_login_state,
};
use wormhole_text::text;

use super::agents::{SettingsHookState, agent_home_for_writing};
use super::error::SettingsError;
use crate::isolation;

/// One profile's login of an agent.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsAgentLogin {
    /// The agent's command (`claude`): the id `settings_set_login_hooks` takes.
    pub agent: String,
    /// The profile's name.
    pub profile: String,
    /// The login folder, absolute.
    pub dir: String,
    /// The folder as the page shows it, `~` for the home folder.
    pub shown: String,
    /// "Work profile · Status hooks on in", in the catalogs' words; the folder follows.
    pub line: String,
    pub hooks: SettingsHookState,
}

fn state_of(state: &Result<State, String>) -> SettingsHookState {
    match state {
        Ok(State::NotInstalled) => SettingsHookState::NotInstalled,
        Ok(State::Installed) => SettingsHookState::Installed,
        Ok(State::Partial) => SettingsHookState::Partial,
        Err(reason) => SettingsHookState::Unreadable { reason: reason.clone() },
    }
}

pub(crate) fn logins_in(root: &DataRoot) -> Vec<SettingsAgentLogin> {
    let list = crate::profile::list_in(root);
    let home = paths::home();
    let mut out = Vec::new();
    for agent in AGENTS {
        let Some(spec) = integrations::spec(agent.id) else { continue };
        for profile in list.profiles() {
            for (id, dir) in profile.login_dirs(home.as_deref()) {
                if id != agent.id {
                    continue;
                }
                let state = spec.state_in(&dir);
                let badge = match &state {
                    Ok(State::Installed) => text(SETTINGS_AGENTS_STATUS_HOOKS_ON),
                    Ok(State::Partial) => text(SETTINGS_AGENTS_HOOKS_INCOMPLETE),
                    _ => text(SETTINGS_AGENTS_NO_HOOKS),
                };
                out.push(SettingsAgentLogin {
                    agent: agent.command.to_string(),
                    profile: profile.name.clone(),
                    dir: dir.to_string_lossy().into_owned(),
                    shown: display_path(&dir, home.as_deref()),
                    line: settings_agents_login_state(&profile.name, badge),
                    hooks: state_of(&state),
                });
            }
        }
    }
    out
}

/// Installs or removes the hooks of `agent` in the login folder `dir`, which must lie under `agent_home`.
pub(crate) fn set_login_hooks_in(
    agent_home: &Path,
    agent: &str,
    dir: &Path,
    install: bool,
) -> Result<(), SettingsError> {
    let found = AGENTS
        .iter()
        .find(|known| known.command == agent)
        .ok_or_else(|| SettingsError::Agents { reason: format!("no agent `{agent}`") })?;
    let spec = integrations::spec(found.id)
        .ok_or_else(|| SettingsError::Agents { reason: format!("{} has no hooks to set", found.name) })?;
    if !dir.starts_with(agent_home) {
        return Err(SettingsError::Agents {
            reason: "hooks are not written outside WORMHOLE_AGENT_HOME in the experiment".into(),
        });
    }
    let done = if install { spec.install_in(dir) } else { spec.uninstall_in(dir) };
    done.map_err(|reason| SettingsError::Agents { reason })
}

/// Every profile's login of an agent with the hooks' state there. Reads each login's config: call it off the main
/// thread, with the agents' list.
#[uniffi::export]
pub fn settings_agent_logins() -> Result<Vec<SettingsAgentLogin>, SettingsError> {
    isolation::require()?;
    Ok(logins_in(&DataRoot::current()))
}

/// Installs (`install`) or removes Wormhole's status hooks of `agent` in the login folder `dir` of a profile. Refuses
/// unless `WORMHOLE_AGENT_HOME` names a folder of its own that holds `dir`.
#[uniffi::export]
pub fn settings_set_login_hooks(agent: String, dir: String, install: bool) -> Result<(), SettingsError> {
    isolation::require()?;
    set_login_hooks_in(&agent_home_for_writing()?, &agent, &PathBuf::from(dir), install)
}
