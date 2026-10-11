//! What the tests show the welcome: the app's state for a first run on an ordinary Mac.

use super::model::Welcome;
use crate::agents::AgentId;
use crate::intro::Handover;
use crate::ipc::control::{
    AgentFact, AgentsFacts, HooksFact, LookFacts, PlaceFact, PrimaryFact, ThemeFact, TrustFacts, WelcomeState,
};

pub fn place(name: &str, path: &str, detail: &str) -> PlaceFact {
    PlaceFact { name: name.into(), path: path.into(), detail: detail.into() }
}

pub fn look() -> LookFacts {
    let theme = |name: &str, light: bool| ThemeFact { name: name.into(), light };
    let primary =
        |name: &str, hex: &str, ansi: Option<u8>| PrimaryFact { name: name.into(), hex: hex.into(), ansi };
    LookFacts {
        themes: vec![
            theme("Tokyo Night", false),
            theme("Catppuccin Mocha", false),
            theme("Nord", false),
            theme("Rose Pine", false),
            theme("Gruvbox Dark", false),
            theme("GitHub Light", true),
        ],
        theme: "Tokyo Night".into(),
        primaries: vec![
            primary("Theme", "", None),
            primary("Green", "#9ece6a", Some(2)),
            primary("Magenta", "#bb9af7", Some(5)),
            primary("Red", "#f7768e", Some(1)),
            primary("Yellow", "#e0af68", Some(3)),
            primary("Cyan", "#7dcfff", Some(6)),
        ],
        primary: String::new(),
        font: "Menlo".into(),
        font_size: 13.0,
        ligatures: true,
    }
}

pub fn agent(id: AgentId, name: &str, hooks: HooksFact, path: Option<&str>) -> AgentFact {
    AgentFact { id, name: name.into(), hooks, path: path.map(Into::into) }
}

/// Claude Code without hooks, Codex with, Aider (no hooks to install).
pub fn agents() -> AgentsFacts {
    AgentsFacts {
        found: vec![
            agent(AgentId::Claude, "Claude Code", HooksFact::Missing, Some("~/.claude/settings.json")),
            agent(AgentId::Codex, "Codex CLI", HooksFact::Installed, Some("~/.codex/hooks.json")),
            agent(AgentId::Aider, "Aider", HooksFact::Unsupported, None),
        ],
        others: 11,
    }
}

pub fn state(handover: Handover) -> WelcomeState {
    WelcomeState {
        wait_ms: 0,
        reduce_motion: false,
        handover,
        first_run: true,
        places: vec![
            place("terminal", "/Users/me/Projects/terminal", "~/Projects"),
            place("Home", "/Users/me", "~"),
            place("Desktop", "/Users/me/Desktop", "~/Desktop"),
            place("Documents", "/Users/me/Documents", "~/Documents"),
        ],
        look: look(),
        agents: Some(agents()),
        trust: None,
        problem: None,
    }
}

pub fn trust() -> TrustFacts {
    TrustFacts {
        folder: "/Users/me/Projects/terminal".into(),
        name: "terminal".into(),
        dir: ".wormhole".into(),
        commands: 2,
        asking: false,
    }
}

/// The same project with the Touch ID prompt up.
pub fn trust_asking(trust: TrustFacts) -> TrustFacts {
    TrustFacts { asking: true, ..trust }
}

/// A live welcome with a shell to hand over to.
pub fn welcome(handover: Handover) -> Welcome {
    Welcome::new(state(handover), true, true)
}
