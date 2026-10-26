//! The bundled agent logos.

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

static ICONS: &[(&str, &[u8])] = &[
    icon!("claude"),
    icon!("codex"),
    icon!("gemini"),
    icon!("copilot"),
    icon!("cursor"),
    icon!("opencode"),
    icon!("qwen"),
    icon!("cline"),
    icon!("amp"),
    icon!("goose"),
    icon!("kiro"),
    icon!("pi"),
];

/// The SVG bytes behind an `agent-icons/…` asset key.
pub fn icon_bytes(key: &str) -> Option<&'static [u8]> {
    ICONS.iter().find(|(k, _)| *k == key).map(|(_, bytes)| *bytes)
}

/// The keys of the bundled logos (a test checks each belongs to an agent).
#[cfg(test)]
pub(super) fn keys() -> impl Iterator<Item = &'static str> {
    ICONS.iter().map(|(key, _)| *key)
}
