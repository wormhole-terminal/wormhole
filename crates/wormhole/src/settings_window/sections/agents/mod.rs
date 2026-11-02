//! Coding agents: how many there are and how they report, a pointer to Notifications,
//! one row per installed agent with its hooks' state and the one action that fits (Install,
//! Repair, Remove), the agents that aren't on PATH folded into one row, and how status works.
//!
//! Hooks are written only by a row's own button, never by opening the page. The page is a module per
//! concern: `snapshot` reads the disk, `installed` draws the agents on this Mac, `others` the ones that
//! aren't, `summary` the counts, `how` how status works.

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::*;
use wormhole_core::agents::Agent;
use wormhole_core::config::settings_index::SectionId as Section;

use super::{agent_access, agent_skills, cli_tool};
use crate::settings::UiColors;
use crate::settings_window::registry::{Setting, toggle};
use crate::settings_window::ui;
use crate::style;

mod how;
mod installed;
mod others;
mod snapshot;
mod summary;

pub(crate) use snapshot::AgentsSnapshot;

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("agents-preview", |_, cx| super::draw_agents_preview(cx)),
    Setting::block("agents-summary", summary::draw),
    Setting::custom("agents-notifications", |_, cx| {
        let ui = ui(cx);
        style::link_button("open-notifications", ui, text(SETTINGS_AGENTS_OPEN_NOTIFICATIONS), true)
            .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::Notifications, cx)))
            .into_any_element()
    }),
    Setting::block("agents-installed", installed::draw),
    Setting::block("agents-other", others::draw),
    Setting::block("agents-how", how::draw),
    cli_tool::SETTING,
    // Shown while the Automation flag is off (the index row says so); with it on, Agent access decides.
    toggle!("control-write", control_write),
    // Shown while the Automation flag is on, like the rows below.
    agent_access::MODE,
    agent_access::PROGRAMS,
    // Shown while the Automation flag is on, like the row above.
    agent_skills::SKILLS,
    agent_skills::MCP,
];

/// An agent's logo on its own tile (`agents::Agent::tile`), or its monogram for an agent without a bundled logo.
pub(super) fn mark(ui: UiColors, agent: &Agent) -> AnyElement {
    style::brand_tile(ui, style::TileFace::Agent(agent.id), style::ROW_TILE)
}
