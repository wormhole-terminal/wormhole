//! `wh doctor`: what an agent needs to know about driving Wormhole, in one call.
//!
//! The app answers [`Facts`] (its version and protocol, the Automation flag, the two switches, and the
//! caller's own terminal as the app finds it); everything else the command line adds from what it can
//! see without the app: the skill's state in each agent's folder and the MCP hint. [`Report::build`]
//! puts them together and says, for each thing that is wrong, what the fix is and who does it. It also
//! works with the app not running: the report says so and still has the skill's state.
//!
//! The report is data first (`--json`); `render` writes it for a person.

mod render;
mod report;

use serde::{Deserialize, Serialize};

use crate::config::AgentAccess;

pub use render::render;
pub use report::{App, Problem, Report, Severity, SkillLine, mcp_add_command};

/// What the app tells `doctor` about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    pub app_version: String,
    pub protocol: u32,
    /// The Automation feature flag.
    pub automation: bool,
    /// Settings → "Let the command line type and open terminals".
    pub control_write: bool,
    /// Settings → "Let the command line read terminals".
    pub control_read: bool,
    /// The terminal the caller runs in, when it runs in one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller: Option<Caller>,
    /// Who may control Wormhole and where the caller stands, while the Automation flag is on (it
    /// replaces the two switches above then). Absent from an older app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<Access>,
}

/// Agent access (`agent-permissions.md`) as it concerns the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Access {
    /// Settings → Coding agents → Agent access.
    pub mode: AgentAccess,
    /// Who the app takes the caller for: an agent's name, or "Scripts".
    pub who: String,
    /// Whether it is let in.
    pub standing: Standing,
}

/// Where the caller stands with the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// Everybody is let in while Wormhole is open.
    Open,
    /// Allowed while this program runs.
    Session,
    /// Allowed until Wormhole quits.
    UntilQuit,
    /// Allowed for good.
    Always,
    /// Not asked yet: the first request that types, opens or reads another terminal asks the user.
    NotAsked,
    /// The user said no a moment ago.
    DeclinedRecently,
    /// Agent access is off.
    Off,
    /// A standing a newer Wormhole gave.
    #[serde(other)]
    Other,
}

/// The caller's own terminal, as the app sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caller {
    pub terminal: i64,
    /// Found by where the command runs (its place in the process tree), not by what it says
    /// (`WORMHOLE_TAB_ID`): only then are the rules about "your own terminal" in force for it.
    pub verified: bool,
    /// Whether the shell has reported command marks (OSC 133): `run` and `wait --block-done` need
    /// them. `None` when the terminal's shell has ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_integration: Option<bool>,
}

impl Standing {
    /// The caller's standing in a few words.
    pub fn words(self) -> &'static str {
        match self {
            Standing::Open => "everyone is allowed while Wormhole is open",
            Standing::Session => "allowed while this program runs",
            Standing::UntilQuit => "allowed until Wormhole quits",
            Standing::Always => "allowed for good",
            Standing::NotAsked => "not asked yet; the first request that needs it asks the user",
            Standing::DeclinedRecently => "told no a moment ago; not asked again for two minutes",
            Standing::Off => "agent access is off",
            Standing::Other => "unknown",
        }
    }
}
