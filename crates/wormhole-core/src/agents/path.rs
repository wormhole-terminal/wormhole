//! Whether an agent is installed (its command on `PATH` or in the usual spots),
//! and the settings page's snapshot of every agent.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::integrations::{self, State};
use super::{AGENTS, Agent, AgentId};
use crate::platform::fs as platform_fs;
use crate::platform::paths;

/// Whether the agent's command is on `PATH` (plus the usual install spots
/// that apps launched from Finder don't see).
pub fn installed(agent: &Agent) -> bool {
    find_on_path(agent.command).is_some()
}

pub(crate) fn find_on_path(command: &str) -> Option<PathBuf> {
    let dirs = search_dirs(
        std::env::var_os("WORMHOLE_AGENT_PATH"),
        std::env::var_os("PATH").unwrap_or_default(),
        &paths::home().unwrap_or_default(),
    );
    dirs.into_iter().map(|dir| dir.join(command)).find(|p| is_executable(p))
}

/// Where an agent's command is looked for: `PATH`, then the usual install spots an app started
/// from the Finder doesn't have on it. `WORMHOLE_AGENT_PATH`, when set, is the whole list
/// instead (split like `PATH`): a test instance sets it empty, so what the Mac running it has
/// installed stays out of its screenshots.
fn search_dirs(agent_path: Option<OsString>, path: OsString, home: &Path) -> Vec<PathBuf> {
    if let Some(only) = agent_path {
        // An empty entry would mean the current folder, as it does on `PATH`; here it means none.
        return std::env::split_paths(&only).filter(|dir| !dir.as_os_str().is_empty()).collect();
    }
    let extra = [
        home.join(".local/bin"),
        home.join(".claude/local"),
        home.join(".bun/bin"),
        home.join(".cargo/bin"),
        home.join(".npm-global/bin"),
        home.join(".opencode/bin"),
        // Where pi's own installer puts it when no folder on `PATH` will do.
        home.join(".pi/agent/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ];
    std::env::split_paths(&path).chain(extra).collect()
}

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && platform_fs::is_executable(&m))
}

/// One agent's row in Settings → Coding agents.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentRow {
    pub id: AgentId,
    /// Its command is on `PATH` (or in the usual install spots).
    pub installed: bool,
    /// The state of Wormhole's hooks for it; `None` when it has no integration.
    pub hooks: Option<Result<State, String>>,
}

/// What the settings page shows about every agent, read in one go. Reading it
/// stats every `PATH` entry per agent and parses their config files: too slow
/// to repeat on every render (hover alone re-renders), so load it when the page
/// opens and after an install or removal, off the UI thread if possible.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentsSnapshot {
    pub rows: Vec<AgentRow>,
}

impl AgentsSnapshot {
    /// `home` is where agent configs live (see `integrations::home`).
    pub fn load(home: &Path) -> Self {
        let rows = AGENTS
            .iter()
            .map(|agent| AgentRow {
                id: agent.id,
                installed: installed(agent),
                hooks: integrations::spec(agent.id).map(|spec| spec.state(home)),
            })
            .collect();
        Self { rows }
    }

    pub fn row(&self, id: AgentId) -> Option<&AgentRow> {
        self.rows.iter().find(|row| row.id == id)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{AgentsSnapshot, search_dirs};
    use crate::agents::integrations::{self, State};
    use crate::agents::{AGENTS, AgentId};

    #[test]
    fn commands_are_looked_for_on_path_and_in_the_usual_spots() {
        let home = Path::new("/Users/ada");
        let dirs = search_dirs(None, "/usr/bin:/bin".into(), home);
        assert_eq!(&dirs[..2], [PathBuf::from("/usr/bin"), PathBuf::from("/bin")], "PATH first");
        for spot in
            ["/Users/ada/.local/bin", "/Users/ada/.claude/local", "/opt/homebrew/bin", "/usr/local/bin"]
        {
            assert!(dirs.contains(&PathBuf::from(spot)), "{spot}");
        }
    }

    /// A test instance says where alone to look (nowhere, as the visual runner sets it), so a
    /// screenshot doesn't list what the Mac running it has installed.
    #[test]
    fn an_agent_path_of_its_own_replaces_the_search() {
        let home = Path::new("/Users/ada");
        assert!(search_dirs(Some("".into()), "/usr/bin:/bin".into(), home).is_empty());
        assert_eq!(
            search_dirs(Some("/a:/b".into()), "/usr/bin:/bin".into(), home),
            [PathBuf::from("/a"), PathBuf::from("/b")],
            "neither PATH nor the usual spots"
        );
    }

    #[test]
    fn the_snapshot_has_every_agent_and_its_hooks() {
        let home = tempfile::tempdir().expect("tempdir");
        let path = home.path().join(".config/opencode/plugins/wormhole.js");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, "export const Mine = 1;\n").expect("write");
        integrations::spec(AgentId::Claude).expect("claude").install(home.path()).expect("install");

        let snapshot = AgentsSnapshot::load(home.path());
        assert_eq!(snapshot.rows.len(), AGENTS.len());
        let hooks = |id| snapshot.row(id).expect("row").hooks.clone();
        assert_eq!(hooks(AgentId::Claude), Some(Ok(State::Installed)));
        assert_eq!(hooks(AgentId::Codex), Some(Ok(State::NotInstalled)));
        assert_eq!(hooks(AgentId::Aider), None, "no integration");
        // Someone else's file where ours would go is not ours.
        assert_eq!(hooks(AgentId::OpenCode), Some(Ok(State::NotInstalled)));
    }
}
