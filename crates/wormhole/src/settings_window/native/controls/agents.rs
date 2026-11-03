//! The controls of the Coding agents page the native window writes through: how much a program may ask of Wormhole
//! over its control socket.

use wormhole_core::config::AgentAccess;
use wormhole_text::keys::{
    SETTINGS_AGENTS_ACCESS_ASK, SETTINGS_AGENTS_ACCESS_OFF, SETTINGS_AGENTS_ACCESS_OPEN,
};

use super::{MenuOption, Setting, toggle};

pub(super) const SETTINGS: &[Setting] = &[
    toggle!("control-write", control_write),
    Setting::menu(
        "agent-access",
        &[
            MenuOption::text(SETTINGS_AGENTS_ACCESS_OFF),
            MenuOption::text(SETTINGS_AGENTS_ACCESS_ASK),
            MenuOption::text(SETTINGS_AGENTS_ACCESS_OPEN),
        ],
        |s| match s.agent_access {
            AgentAccess::Off => 0,
            AgentAccess::Ask => 1,
            AgentAccess::Open => 2,
        },
        |s, ix| s.agent_access = [AgentAccess::Off, AgentAccess::Ask, AgentAccess::Open][ix.min(2)],
    ),
];
