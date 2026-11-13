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
    let dir = resolved_within(agent_home, dir).ok_or_else(|| SettingsError::Agents {
        reason: "hooks are not written outside WORMHOLE_AGENT_HOME in the experiment".into(),
    })?;
    let done = if install { spec.install_in(&dir) } else { spec.uninstall_in(&dir) };
    done.map_err(|reason| SettingsError::Agents { reason })
}

/// `dir` as the disk resolves it, when that lies under `root` as the disk resolves it. A `..` is refused outright and
/// every link on the way is followed, so a folder reached through a link out of `root` is refused. The folder itself
/// need not exist yet (an install makes it): the deepest part that does is resolved and the rest is plain names.
fn resolved_within(root: &Path, dir: &Path) -> Option<PathBuf> {
    use std::path::Component;
    if !dir
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::RootDir | Component::CurDir))
    {
        return None;
    }
    let mut missing = Vec::new();
    let mut existing = dir;
    let mut real = loop {
        match std::fs::canonicalize(existing) {
            Ok(real) => break real,
            // A link that leads nowhere is still a link: refused, never written through.
            Err(_) if existing.symlink_metadata().is_ok() => return None,
            Err(_) => {
                missing.push(existing.file_name()?);
                existing = existing.parent()?;
            }
        }
    };
    real.extend(missing.iter().rev());
    real.starts_with(std::fs::canonicalize(root).ok()?).then_some(real)
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::set_login_hooks_in;

    #[test]
    fn a_login_folder_is_checked_by_where_it_leads_not_by_how_it_is_written() {
        let scratch = tempfile::tempdir().expect("scratch");
        let home = scratch.path().join("agent-home");
        let outside = scratch.path().join("real-home");
        std::fs::create_dir_all(&home).expect("home");
        std::fs::create_dir_all(&outside).expect("outside");

        // Inside the folder, new or not yet made: allowed, and written there.
        set_login_hooks_in(&home, "claude", &home.join("work/.claude"), true).expect("inside");
        assert!(home.join("work/.claude/settings.json").exists());

        // Written with `..` so that it only starts with the folder.
        let dotted = home.join("..").join("real-home").join(".claude");
        assert!(set_login_hooks_in(&home, "claude", &dotted, true).is_err(), "`..` is refused");
        assert!(!outside.join(".claude").exists(), "nothing written outside");

        // Reached through a symbolic link that leaves the folder.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, home.join("link")).expect("link");
            let through = home.join("link").join(".claude");
            assert!(set_login_hooks_in(&home, "claude", &through, true).is_err(), "a link out is refused");
            assert!(!outside.join(".claude").exists(), "nothing written through the link");
            assert!(set_login_hooks_in(&home, "claude", Path::new("/definitely/not/under"), true).is_err());
        }
    }
}
