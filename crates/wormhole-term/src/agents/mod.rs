//! The coding agents Wormhole knows: the registry (who they are, what they run as) and how to
//! recognise one in a terminal. Detection works for every agent in the registry; what the
//! terminal layer does with it is in `program_status` and in the `Pty`'s foreground queries.
//! Installing, hooks and logos are core's (`wormhole_core::agents`).

mod detect;
mod identify;

use serde::{Deserialize, Serialize};

pub(crate) use detect::shell_holds_terminal;
pub use detect::{Foreground, command_label, foreground_agent, foreground_command, shell_in_foreground};
pub use identify::identify;

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
    Pi,
    /// macOS 27's `fm`, Apple's command for its on-device Foundation Models: only `fm chat` is the agent
    /// (see `identify`).
    Fm,
}

/// What an agent's status tile is painted with. A brand's colour is the brand's, not the theme's:
/// the tile is the agent's app icon, which looks the same on a light sidebar and in every theme, so
/// it is kept here beside the logo it goes with (each entry says where its colour comes from), not
/// as a design token. What the theme does decide stays a token: the black tile (`tile.background`)
/// and the colour of the logo on a brand's colour (`tile.mark.brand`, white).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    /// The theme's black tile: for a brand that is black and white.
    Black,
    /// One colour, `0xRRGGBB`.
    Solid(u32),
    /// From the top left to the bottom right, `0xRRGGBB` each.
    Gradient(u32, u32),
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
    /// Asset key of the bundled monochrome logo, if there is one (`agent-icons/…`; core holds the
    /// bytes behind the key).
    logo: Option<&'static str>,
    /// What its status tile is painted with (see [`Tile`]).
    pub tile: Tile,
}

impl Agent {
    /// Asset key of the bundled monochrome logo, if there is one.
    pub fn icon(&self) -> Option<&'static str> {
        self.logo
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
        logo: Some("agent-icons/claude.svg"),
        // Anthropic's orange: Lobe Icons' and Simple Icons' Claude (and Claude Code) colour.
        tile: Tile::Solid(0xd97757),
    },
    Agent {
        id: AgentId::Codex,
        name: "Codex CLI",
        command: "codex",
        executables: &["codex"],
        packages: &["@openai/codex"],
        monogram: "CX",
        logo: Some("agent-icons/codex.svg"),
        // The deep blue end of Codex's own gradient mark (Lobe Icons' colour Codex: #b1a7ff, #7a9dff,
        // #3941ff); its lighter stops are too light for the white logo.
        tile: Tile::Solid(0x3941ff),
    },
    Agent {
        id: AgentId::Gemini,
        name: "Gemini CLI",
        command: "gemini",
        executables: &["gemini"],
        packages: &["@google/gemini-cli"],
        monogram: "GM",
        logo: Some("agent-icons/gemini.svg"),
        // The design's blue to violet (the boards' RF-12). Google's own sparkle has four stops, no two-stop
        // gradient; its main blue is #3186ff.
        tile: Tile::Gradient(0x3b7bff, 0x8f6cf5),
    },
    Agent {
        id: AgentId::Copilot,
        name: "GitHub Copilot CLI",
        command: "copilot",
        executables: &["copilot"],
        packages: &["@github/copilot"],
        monogram: "GC",
        logo: Some("agent-icons/copilot.svg"),
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::Cursor,
        name: "Cursor Agent",
        command: "cursor-agent",
        executables: &["cursor-agent"],
        packages: &["/cursor-agent/"],
        monogram: "CA",
        logo: Some("agent-icons/cursor.svg"),
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::OpenCode,
        name: "OpenCode",
        command: "opencode",
        executables: &["opencode"],
        packages: &["opencode-ai"],
        monogram: "OC",
        logo: Some("agent-icons/opencode.svg"),
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::Aider,
        name: "Aider",
        command: "aider",
        executables: &["aider"],
        packages: &["aider-chat"],
        monogram: "AI",
        logo: None,
        // The dark green of Aider's own app icon (its site's apple-touch-icon), under a light "a".
        tile: Tile::Solid(0x014f2c),
    },
    Agent {
        id: AgentId::Amp,
        name: "Amp",
        command: "amp",
        executables: &["amp"],
        packages: &["@ampcode/cli", "@sourcegraph/amp"],
        monogram: "AM",
        logo: Some("agent-icons/amp.svg"),
        // Black: ampcode.com's dark theme colour #091c1e is all but black.
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::Goose,
        name: "Goose",
        command: "goose",
        executables: &["goose"],
        packages: &[],
        monogram: "GO",
        logo: Some("agent-icons/goose.svg"),
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::Qwen,
        name: "Qwen Code",
        command: "qwen",
        executables: &["qwen"],
        packages: &["@qwen-code/qwen-code"],
        monogram: "QC",
        logo: Some("agent-icons/qwen.svg"),
        // Lobe Icons' Qwen colour (Simple Icons has the like #6950ef).
        tile: Tile::Solid(0x615ced),
    },
    Agent {
        id: AgentId::Crush,
        name: "Crush",
        command: "crush",
        executables: &["crush"],
        packages: &["@charmland/crush"],
        monogram: "CR",
        logo: None,
        // Charm's "Charple" (charmbracelet/x `exp/charmtone`), Crush's primary.
        tile: Tile::Solid(0x6b50ff),
    },
    Agent {
        id: AgentId::Droid,
        name: "Factory Droid",
        command: "droid",
        executables: &["droid"],
        packages: &["@factory/cli"],
        monogram: "FD",
        logo: None,
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::Auggie,
        name: "Auggie",
        command: "auggie",
        executables: &["auggie"],
        packages: &["@augmentcode/auggie"],
        monogram: "AU",
        logo: None,
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::Kiro,
        name: "Kiro CLI",
        command: "kiro-cli",
        executables: &["kiro-cli", "kiro-cli-chat"],
        packages: &[],
        monogram: "KI",
        logo: Some("agent-icons/kiro.svg"),
        // The tile of kiro.dev's own icon (icon.svg), with a white ghost on it.
        tile: Tile::Solid(0x9046ff),
    },
    Agent {
        id: AgentId::Cline,
        name: "Cline CLI",
        command: "cline",
        executables: &["cline"],
        packages: &["@cline/cli"],
        monogram: "CL",
        logo: Some("agent-icons/cline.svg"),
        // Black: Cline's slate (#323b43 in Lobe Icons) is all but black.
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::Pi,
        name: "Pi",
        command: "pi",
        executables: &["pi"],
        // The package moved from its author's scope to Earendil's; an older install keeps the first.
        packages: &["@mariozechner/pi-coding-agent", "@earendil-works/pi-coding-agent"],
        monogram: "PI",
        logo: Some("agent-icons/pi.svg"),
        tile: Tile::Black,
    },
    Agent {
        id: AgentId::Fm,
        name: "Foundation Models",
        // Part of macOS 27 (`/usr/bin/fm`), so it is installed wherever that is.
        command: "fm",
        executables: &["fm"],
        packages: &[],
        monogram: "FM",
        // Apple's marks can't be bundled: the two letters stand.
        logo: None,
        tile: Tile::Black,
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
