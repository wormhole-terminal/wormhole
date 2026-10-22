//! Coding agents Wormhole knows about: how to recognise them in a
//! terminal, and how (if at all) they report their status.
//!
//! Detection works for every agent in the registry. Exact status needs the
//! agent to call back into Wormhole; `integrations` installs that for the
//! agents with a documented hook mechanism, the rest fall back to the
//! terminal-output heuristic.

mod icons;
pub mod integrations;
mod path;

pub use icons::{ICON_PREFIX, icon_bytes};
pub(crate) use path::find_on_path;
pub use path::{AgentRow, AgentsSnapshot, installed};

// The registry and the detection live with the terminal layer, which needs them (the `Pty`'s
// foreground queries, the program status), and are re-exported here: from the terminal crate itself,
// not core's `terminal`, which starts shells and so sits above this module.
pub use wormhole_term::agents::{
    AGENTS, Agent, AgentId, Foreground, TileColor, agent, command_label, find, foreground_agent,
    foreground_command, identify, shell_in_foreground,
};

#[cfg(test)]
mod tests;
