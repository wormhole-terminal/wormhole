//! Where agents keep their skills, from each agent's own documentation (checked 2026-10-27). Only
//! agents whose folder is documented are here; the others (Goose, Pi and more read `~/.agents/skills`
//! as well) find the skill through the Codex row's folder, which is that shared one.

use std::path::{Path, PathBuf};

use super::NAME;
use crate::agents::AgentId;

/// An agent that loads skills, and the folder it loads them from.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub agent: AgentId,
    /// The skills folder, relative to the home directory; the skill's own folder is inside it.
    pub dir: &'static str,
    /// Where this is documented.
    pub source: &'static str,
}

/// Every agent that has a documented personal skills folder, in the registry's order.
pub const TARGETS: &[Target] = &[
    Target {
        agent: AgentId::Claude,
        dir: ".claude/skills",
        source: "https://code.claude.com/docs/en/skills",
    },
    // The shared folder of the Agent Skills ecosystem: Codex reads it first, and Gemini CLI, Copilot CLI,
    // Qwen Code, Droid, Goose, Pi and OpenCode read it too.
    Target {
        agent: AgentId::Codex,
        dir: ".agents/skills",
        source: "https://developers.openai.com/codex/skills",
    },
    Target {
        agent: AgentId::Gemini,
        dir: ".gemini/skills",
        source: "https://geminicli.com/docs/cli/skills/",
    },
    Target {
        agent: AgentId::Copilot,
        dir: ".copilot/skills",
        source: "https://docs.github.com/en/copilot/concepts/agents/about-agent-skills",
    },
    Target {
        agent: AgentId::OpenCode,
        dir: ".config/opencode/skills",
        source: "https://opencode.ai/docs/skills/",
    },
    Target {
        agent: AgentId::Qwen,
        dir: ".qwen/skills",
        source: "https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/",
    },
    Target {
        agent: AgentId::Droid,
        dir: ".factory/skills",
        source: "https://docs.factory.com/cli/configuration/skills",
    },
];

/// The target of `agent`, when it loads skills.
pub fn target(agent: AgentId) -> Option<&'static Target> {
    TARGETS.iter().find(|target| target.agent == agent)
}

impl Target {
    /// The skill's own folder under `home`.
    pub fn path(&self, home: &Path) -> PathBuf {
        home.join(self.dir).join(NAME)
    }

    /// [`path`](Self::path) the way a person says it (`~/.claude/skills/wormhole`).
    pub fn display_path(&self, home: &Path) -> String {
        let path = self.path(home);
        match path.strip_prefix(home) {
            Ok(rest) if !home.as_os_str().is_empty() => format!("~/{}", rest.display()),
            _ => path.display().to_string(),
        }
    }

    /// The agent's name on the command line (`claude`, `opencode`).
    pub fn word(&self) -> String {
        serde_json::to_value(self.agent)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default()
    }
}

/// The agent a command line spells `word`, when it loads skills.
pub fn by_word(word: &str) -> Option<&'static Target> {
    TARGETS.iter().find(|target| target.word() == word.trim().to_ascii_lowercase())
}
