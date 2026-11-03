//! Coding agents: one row per agent on this Mac with how its status reaches Wormhole and the control that fits
//! its hooks (Add, Repair, or a menu of Repair, Remove and Reveal), the agents that aren't on PATH behind one
//! row that opens them, the command line, who may drive Wormhole, the skill, and a last line that points to
//! Notifications and opens how status works.
//!
//! Hooks are written only by a row's own button, never by opening the page. The page is a module per
//! concern: `snapshot` reads the disk, `installed` draws the agents on this Mac, `others` the ones that
//! aren't, `footer` the way to Notifications and `how` how status works.

use super::{agent_access, agent_skills, cli_tool};
use crate::settings_window::registry::{Setting, toggle};

mod footer;
mod how;
mod installed;
mod others;
mod snapshot;

pub(crate) use snapshot::{AgentRow, AgentsSnapshot};

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("agents-preview", |_, cx| super::draw_agents_preview(cx)),
    Setting::rows("agents-installed", installed::draw),
    Setting::rows("agents-other", others::draw),
    cli_tool::SETTING,
    // Shown while the Automation flag is off (the index row says so); with it on, Agent access decides.
    toggle!("control-write", control_write),
    // Shown while the Automation flag is on, like the rows below.
    agent_access::MODE,
    agent_access::PROGRAMS,
    // Shown while the Automation flag is on, like the row above.
    agent_skills::SKILLS,
    agent_skills::MCP,
    Setting::block("agents-footer", footer::draw),
];
