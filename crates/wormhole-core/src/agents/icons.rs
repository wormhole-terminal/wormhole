//! The bundled agent logos.

use super::AgentId;

/// Bundled logos (monochrome, tinted by the UI), served by the app's asset
/// source under `agent-icons/`.
pub const ICON_PREFIX: &str = "agent-icons/";

macro_rules! icon {
    ($name:literal) => {
        (
            concat!("agent-icons/", $name, ".svg"),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent-icons/", $name, ".svg"))
                as &[u8],
        )
    };
}

static ICONS: &[(AgentId, (&str, &[u8]))] = &[
    (AgentId::Claude, icon!("claude")),
    (AgentId::Gemini, icon!("gemini")),
    (AgentId::Copilot, icon!("copilot")),
    (AgentId::Cursor, icon!("cursor")),
    (AgentId::OpenCode, icon!("opencode")),
    (AgentId::Qwen, icon!("qwen")),
    (AgentId::Cline, icon!("cline")),
];

pub(super) fn icon_asset(id: AgentId) -> Option<&'static str> {
    ICONS.iter().find(|(agent, _)| *agent == id).map(|(_, (key, _))| *key)
}

/// The SVG bytes behind an `agent-icons/…` asset key.
pub fn icon_bytes(key: &str) -> Option<&'static [u8]> {
    ICONS.iter().find(|(_, (k, _))| *k == key).map(|(_, (_, bytes))| *bytes)
}
