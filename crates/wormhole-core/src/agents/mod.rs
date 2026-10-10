//! Coding agents Wormhole knows about: how to recognise them in a
//! terminal, and how (if at all) they report their status.
//!
//! Detection works for every agent in the registry. Exact status needs the
//! agent to call back into Wormhole; `integrations` installs that for the
//! agents with a documented hook mechanism, the rest fall back to the
//! terminal-output heuristic.

mod detect;
mod icons;
mod identify;
pub mod integrations;
mod path;

use serde::{Deserialize, Serialize};

pub(crate) use detect::shell_holds_terminal;
pub use detect::{Foreground, command_label, foreground_agent, foreground_command, shell_in_foreground};
pub use icons::{ICON_PREFIX, icon_bytes};
pub use identify::identify;
pub(crate) use path::find_on_path;
pub use path::{AgentRow, AgentsSnapshot, installed};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentId {
    Claude,
    Codex,
    Gemini,
    Copilot,
    Cursor,
    /// `opencode` is what the installed plugin passes as `--agent`; kebab-case
    /// alone would put a dash in it.
    #[serde(rename = "opencode")]
    OpenCode,
    Aider,
    Amp,
    Goose,
    Qwen,
    Crush,
    Droid,
    Auggie,
    Kiro,
    Cline,
}

pub struct Agent {
    pub id: AgentId,
    pub name: &'static str,
    /// The command users type; also what `installed` looks for on `PATH`.
    pub command: &'static str,
    /// Executable names that identify the agent (process or script name).
    pub executables: &'static [&'static str],
    /// Substrings of a script path that identify it when it runs under an
    /// interpreter, e.g. the npm package directory.
    pub packages: &'static [&'static str],
    /// The agent's mark: two capitals, the initials of its name's words ("CLI"
    /// aside) or a one-word name's first two letters, unless a mark is fixed for it (Codex is CX, so
    /// it isn't Claude Code's CC again; Gemini is GM). Settings and the first-run sheet show it,
    /// and a sidebar row for an agent without a logo.
    pub monogram: &'static str,
}

impl Agent {
    /// Asset key of the bundled monochrome logo, if there is one.
    pub fn icon(&self) -> Option<&'static str> {
        icons::icon_asset(self.id)
    }

    /// The name without what kind of program it is ("Claude" for Claude Code, "Codex" for Codex
    /// CLI), where a line names the agent at work ("Claude working · 4m").
    pub fn short_name(&self) -> &'static str {
        [" CLI", " Code", " Agent"].iter().find_map(|kind| self.name.strip_suffix(kind)).unwrap_or(self.name)
    }
}

pub static AGENTS: &[Agent] = &[
    Agent {
        id: AgentId::Claude,
        name: "Claude Code",
        command: "claude",
        executables: &["claude"],
        packages: &["@anthropic-ai/claude-code", "/.local/share/claude/versions/"],
        monogram: "CC",
    },
    Agent {
        id: AgentId::Codex,
        name: "Codex CLI",
        command: "codex",
        executables: &["codex"],
        packages: &["@openai/codex"],
        monogram: "CX",
    },
    Agent {
        id: AgentId::Gemini,
        name: "Gemini CLI",
        command: "gemini",
        executables: &["gemini"],
        packages: &["@google/gemini-cli"],
        monogram: "GM",
    },
    Agent {
        id: AgentId::Copilot,
        name: "GitHub Copilot CLI",
        command: "copilot",
        executables: &["copilot"],
        packages: &["@github/copilot"],
        monogram: "GC",
    },
    Agent {
        id: AgentId::Cursor,
        name: "Cursor Agent",
        command: "cursor-agent",
        executables: &["cursor-agent"],
        packages: &["/cursor-agent/"],
        monogram: "CA",
    },
    Agent {
        id: AgentId::OpenCode,
        name: "OpenCode",
        command: "opencode",
        executables: &["opencode"],
        packages: &["opencode-ai"],
        monogram: "OC",
    },
    Agent {
        id: AgentId::Aider,
        name: "Aider",
        command: "aider",
        executables: &["aider"],
        packages: &["aider-chat"],
        monogram: "AI",
    },
    Agent {
        id: AgentId::Amp,
        name: "Amp",
        command: "amp",
        executables: &["amp"],
        packages: &["@ampcode/cli", "@sourcegraph/amp"],
        monogram: "AM",
    },
    Agent {
        id: AgentId::Goose,
        name: "Goose",
        command: "goose",
        executables: &["goose"],
        packages: &[],
        monogram: "GO",
    },
    Agent {
        id: AgentId::Qwen,
        name: "Qwen Code",
        command: "qwen",
        executables: &["qwen"],
        packages: &["@qwen-code/qwen-code"],
        monogram: "QC",
    },
    Agent {
        id: AgentId::Crush,
        name: "Crush",
        command: "crush",
        executables: &["crush"],
        packages: &["@charmland/crush"],
        monogram: "CR",
    },
    Agent {
        id: AgentId::Droid,
        name: "Factory Droid",
        command: "droid",
        executables: &["droid"],
        packages: &["@factory/cli"],
        monogram: "FD",
    },
    Agent {
        id: AgentId::Auggie,
        name: "Auggie",
        command: "auggie",
        executables: &["auggie"],
        packages: &["@augmentcode/auggie"],
        monogram: "AU",
    },
    Agent {
        id: AgentId::Kiro,
        name: "Kiro CLI",
        command: "kiro-cli",
        executables: &["kiro-cli", "kiro-cli-chat"],
        packages: &[],
        monogram: "KI",
    },
    Agent {
        id: AgentId::Cline,
        name: "Cline CLI",
        command: "cline",
        executables: &["cline"],
        packages: &["@cline/cli"],
        monogram: "CL",
    },
];

/// The registry entry for `id`.
pub fn find(id: AgentId) -> Option<&'static Agent> {
    AGENTS.iter().find(|a| a.id == id)
}

/// [`find`], for ids from the closed enum, which are all registered (a test
/// checks); Claude stands in otherwise.
pub fn agent(id: AgentId) -> &'static Agent {
    find(id).unwrap_or(&AGENTS[0])
}

#[cfg(test)]
mod tests;
