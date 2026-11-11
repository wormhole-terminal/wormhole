//! The Agents page's skill rows: the Wormhole skill that teaches an agent to drive Wormhole through `wh`
//! (`agents::skill`), one row per agent that loads skills, and the line that adds `wh mcp` to Claude Code.
//! Nothing is written by reading; a row's own button installs, updates or removes the skill, and like the
//! hooks it writes only under `WORMHOLE_AGENT_HOME` (never the real home).

use std::path::Path;

use wormhole_core::agents::skill::{self, State};
use wormhole_core::agents::{self, integrations};
use wormhole_core::ipc::control::doctor::mcp_add_command;
use wormhole_text::keys::{
    SETTINGS_AGENTS_SKILL_COVERED, SETTINGS_AGENTS_SKILL_CURRENT, SETTINGS_AGENTS_SKILL_FOREIGN,
    SETTINGS_AGENTS_SKILL_NOT_INSTALLED, SETTINGS_AGENTS_SKILL_OUTDATED,
};
use wormhole_text::text;

use super::agents::agent_home_for_writing;
use super::error::SettingsError;

/// Where one agent's skill stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SettingsSkillState {
    NotInstalled,
    Current,
    /// Ours, from another build: Update.
    Outdated,
    /// Something else of the same name is in the way: never touched.
    Foreign,
    /// Not installed here, but a folder the agent also reads has it.
    Covered,
    /// The folder does not read; the row's `problem` says why.
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsSkillRow {
    /// The target's word (`claude`): what `settings_set_skill` takes.
    pub id: String,
    pub name: String,
    pub two_letter_mark: String,
    pub state: SettingsSkillState,
    /// "Skill installed in" and its kin, in the catalogs' words.
    pub detail: String,
    /// The skill's folder as a person says it (`~/.claude/skills/wormhole`), or the covering one.
    pub shown: String,
    pub problem: Option<String>,
}

/// The rows for the skills under `home`: the agents on this Mac's PATH, and any that has the skill already.
pub(crate) fn rows_in(home: &Path) -> Vec<SettingsSkillRow> {
    skill::states(home)
        .into_iter()
        .filter_map(|(target, state)| {
            let agent = agents::agent(target.agent);
            let found = agents::installed(agent);
            let covered = skill::covered_by(target, home).filter(|_| state == Ok(State::NotInstalled));
            if !found && covered.is_none() && matches!(state, Ok(State::NotInstalled)) {
                return None;
            }
            let (state, problem) = match (&state, &covered) {
                (Ok(State::NotInstalled), Some(_)) => (SettingsSkillState::Covered, None),
                (Ok(State::NotInstalled), None) => (SettingsSkillState::NotInstalled, None),
                (Ok(State::Current), _) => (SettingsSkillState::Current, None),
                (Ok(State::Outdated), _) => (SettingsSkillState::Outdated, None),
                (Ok(State::Foreign), _) => (SettingsSkillState::Foreign, None),
                (Err(why), _) => (SettingsSkillState::Unreadable, Some(why.clone())),
            };
            let detail = match state {
                SettingsSkillState::Covered => SETTINGS_AGENTS_SKILL_COVERED,
                SettingsSkillState::Current => SETTINGS_AGENTS_SKILL_CURRENT,
                SettingsSkillState::Outdated => SETTINGS_AGENTS_SKILL_OUTDATED,
                SettingsSkillState::Foreign => SETTINGS_AGENTS_SKILL_FOREIGN,
                _ => SETTINGS_AGENTS_SKILL_NOT_INSTALLED,
            };
            Some(SettingsSkillRow {
                id: target.word(),
                name: agent.name.to_string(),
                two_letter_mark: agent.monogram.to_string(),
                state,
                detail: text(detail).to_string(),
                shown: covered.unwrap_or_else(|| target.display_path(home)),
                problem,
            })
        })
        .collect()
}

pub(crate) fn set_skill_in(home: &Path, word: &str, install: bool) -> Result<(), SettingsError> {
    let target = skill::by_word(word)
        .ok_or_else(|| SettingsError::Agents { reason: format!("no agent skill `{word}`") })?;
    let done = if install {
        skill::install(target, home).map(drop)
    } else {
        skill::uninstall(target, home).map(drop)
    };
    done.map_err(|reason| SettingsError::Agents { reason })
}

/// The skill of every agent that loads skills and is here, read under `WORMHOLE_AGENT_HOME` or the home
/// folder. Stats the PATH and the skill folders: off the main thread.
#[uniffi::export]
pub fn settings_skill_rows() -> Vec<SettingsSkillRow> {
    rows_in(&integrations::home())
}

/// Installs (`install`) or removes the skill of the target `word`. Refuses unless `WORMHOLE_AGENT_HOME`
/// names a folder of its own.
#[uniffi::export]
pub fn settings_set_skill(word: String, install: bool) -> Result<(), SettingsError> {
    set_skill_in(&agent_home_for_writing()?, &word, install)
}

/// The `claude mcp add …` line the page shows to copy: Wormhole edits no other app's MCP settings.
#[uniffi::export]
pub fn settings_mcp_command() -> String {
    mcp_add_command()
}

#[cfg(test)]
mod tests {
    use wormhole_core::agents::AgentId;
    use wormhole_core::agents::skill;

    use super::{SettingsSkillState, rows_in, set_skill_in};

    #[test]
    fn a_skill_is_installed_and_removed_inside_the_home_given_and_nowhere_else() {
        let home = tempfile::tempdir().expect("temp home");
        let word = skill::target(AgentId::Claude).expect("Claude loads skills").word();
        let state = |home: &std::path::Path| {
            rows_in(home).into_iter().find(|row| row.id == word).map(|row| row.state)
        };
        set_skill_in(home.path(), &word, true).expect("installs");
        assert_eq!(state(home.path()), Some(SettingsSkillState::Current));
        assert!(home.path().join(".claude/skills/wormhole").exists());
        set_skill_in(home.path(), &word, false).expect("removes");
        assert_ne!(state(home.path()), Some(SettingsSkillState::Current));
        assert!(set_skill_in(home.path(), "no-such-agent", true).is_err());
    }
}
