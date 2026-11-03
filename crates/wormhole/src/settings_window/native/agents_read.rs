//! What the Agents page reads from disk: which agents are installed, what state their hooks are in
//! (for the default location and for each profile's login), and each agent's skill.

use std::path::{Path, PathBuf};

use wormhole_core::agents::integrations::{self, Spec, State};
use wormhole_core::agents::{self, AGENTS, Agent};
use wormhole_core::config::profile::Profile;

use wormhole_core::agents::skill::{self, State as SkillState, Target};

/// What the Agents page shows, read from disk: which agents are installed and what state
/// their hooks are in. That is a few hundred `stat`s (PATH) and a parse of each agent's
/// config, so it is read when the window opens and after a button changed something, on the
/// background executor, and not on every frame the window draws.
pub(crate) struct AgentsSnapshot {
    home: PathBuf,
    pub(in crate::settings_window) rows: Vec<AgentRow>,
    /// The Wormhole skill in each skills folder (`agent_skills`).
    pub(in crate::settings_window) skills: Vec<SkillRow>,
}

pub(crate) struct AgentRow {
    pub(crate) agent: &'static Agent,
    pub(crate) installed: bool,
    /// How hooks are written for this agent, if they can be.
    pub(crate) spec: Option<&'static Spec>,
    /// `None` when there is no spec.
    pub(crate) state: Option<Result<State, String>>,
    /// The profiles' login folders for this agent: a second location for the hooks each.
    pub(crate) logins: Vec<LoginRow>,
}

/// Where a profile's login of an agent keeps its config, and the state of the hooks there.
pub(crate) struct LoginRow {
    pub(in crate::settings_window) profile: String,
    pub(in crate::settings_window) dir: PathBuf,
    pub(in crate::settings_window) state: Result<State, String>,
}

impl AgentRow {
    pub(in crate::settings_window) fn hooks(&self) -> Option<State> {
        self.state.as_ref().and_then(|state| state.as_ref().ok()).copied()
    }
}

impl AgentsSnapshot {
    /// Read every agent's state under `home`, installed agents first (the registry's order
    /// otherwise).
    pub(crate) fn load(home: &Path, profiles: &[Profile]) -> Self {
        let user_home = crate::settings_window::native::paths::home();
        let mut rows: Vec<AgentRow> = AGENTS
            .iter()
            .map(|agent| {
                let spec = integrations::spec(agent.id);
                AgentRow {
                    agent,
                    installed: agents::installed(agent),
                    spec,
                    state: spec.map(|spec| spec.state(home)),
                    logins: spec
                        .map(|spec| {
                            profiles
                                .iter()
                                .flat_map(|p| {
                                    p.login_dirs(user_home.as_deref())
                                        .into_iter()
                                        .filter(|(id, _)| *id == agent.id)
                                        .map(|(_, dir)| (p.name.clone(), dir))
                                })
                                .map(|(profile, dir)| LoginRow { state: spec.state_in(&dir), profile, dir })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect();
        rows.sort_by_key(|row| !row.installed);
        Self { home: home.to_path_buf(), rows, skills: load_skills(home) }
    }

    pub(crate) fn installed(&self) -> impl Iterator<Item = &AgentRow> {
        self.rows.iter().filter(|row| row.installed)
    }

    pub(in crate::settings_window) fn others(&self) -> impl Iterator<Item = &AgentRow> {
        self.rows.iter().filter(|row| !row.installed)
    }

    pub(crate) fn home(&self) -> &Path {
        &self.home
    }
}

/// One agent's skill as the files have it, read with the rest of the page (`AgentsSnapshot::load`).
pub(crate) struct SkillRow {
    pub(in crate::settings_window) target: &'static Target,
    /// Whether the agent's command is on this Mac's `PATH`.
    pub(in crate::settings_window) found: bool,
    pub(in crate::settings_window) state: Result<SkillState, String>,
    /// Where the skill's folder is, as a person says it.
    pub(in crate::settings_window) shown: String,
    /// The folder, read by this agent as well, that has the skill when its own has not.
    pub(in crate::settings_window) covered: Option<String>,
}

fn load_skills(home: &Path) -> Vec<SkillRow> {
    skill::states(home)
        .into_iter()
        .map(|(target, state)| SkillRow {
            target,
            found: agents::installed(agents::agent(target.agent)),
            covered: skill::covered_by(target, home).filter(|_| state == Ok(SkillState::NotInstalled)),
            state,
            shown: target.display_path(home),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use wormhole_core::agents::integrations::{self, State};
    use wormhole_core::agents::{AGENTS, AgentId};

    use super::AgentsSnapshot;

    /// A scratch home directory, removed at the end of the test.
    struct Home(std::path::PathBuf);

    impl Home {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("wormhole-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("scratch home");
            Self(dir)
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn state_of(snapshot: &AgentsSnapshot, id: AgentId) -> Option<State> {
        let row = snapshot.rows.iter().find(|row| row.agent.id == id).expect("every agent has a row");
        row.state.as_ref().and_then(|state| state.as_ref().ok()).copied()
    }

    #[test]
    fn the_snapshot_lists_every_agent_with_its_hook_state() {
        let home = Home::new("agents-state");
        let before = AgentsSnapshot::load(&home.0, &[]);
        assert_eq!(before.rows.len(), AGENTS.len());
        assert_eq!(state_of(&before, AgentId::Claude), Some(State::NotInstalled));

        let spec = integrations::spec(AgentId::Claude).expect("Claude Code has hooks");
        spec.install(&home.0).expect("installs into the scratch home");
        let after = AgentsSnapshot::load(&home.0, &[]);
        assert_eq!(state_of(&after, AgentId::Claude), Some(State::Installed));
        // Another agent's files were not touched.
        assert_eq!(state_of(&after, AgentId::Codex), Some(State::NotInstalled));

        spec.uninstall(&home.0).expect("removes them again");
        assert_eq!(state_of(&AgentsSnapshot::load(&home.0, &[]), AgentId::Claude), Some(State::NotInstalled));
    }

    #[test]
    fn installed_agents_come_first_and_the_registry_order_holds_otherwise() {
        let home = Home::new("agents-order");
        let snapshot = AgentsSnapshot::load(&home.0, &[]);
        let flags: Vec<bool> = snapshot.rows.iter().map(|row| row.installed).collect();
        let first_missing = flags.iter().position(|installed| !installed).unwrap_or(flags.len());
        assert!(flags[first_missing..].iter().all(|installed| !installed), "{flags:?}");
        // Within each group, the registry order.
        let order = |installed: bool| -> Vec<usize> {
            snapshot
                .rows
                .iter()
                .filter(|row| row.installed == installed)
                .map(|row| AGENTS.iter().position(|a| a.id == row.agent.id).expect("in the registry"))
                .collect()
        };
        for installed in [true, false] {
            let positions = order(installed);
            assert!(positions.windows(2).all(|w| w[0] < w[1]), "{positions:?}");
        }
    }
}
