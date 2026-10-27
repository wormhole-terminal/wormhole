//! The Wormhole skill: a folder an AI coding agent loads to learn to drive Wormhole through `wh`
//! (the Agent Skills format: `SKILL.md` with a name and a description, and reference files it opens
//! when it needs them). The text ships inside the app; a person puts it in an agent's skill folder by
//! pressing Install (Settings → Coding agents, or `wh skill install`), never automatically.
//!
//! - `targets`: where each agent that reads skills keeps them;
//! - `install`: its state there, Install, Update and Uninstall, in a folder of our own that carries a
//!   marker, so a skill somebody else wrote under the same name is never touched.
//!
//! The text is public documentation: it names products and commands, nothing private. Its commands are
//! checked against the parser and its tools against the MCP server (`ipc`'s guard tests), so it cannot
//! drift from what `wh` does.

mod install;
mod targets;
#[cfg(test)]
mod tests;

pub use install::{Done, State, install, state, states, uninstall};
pub use targets::{TARGETS, Target, by_word, target};

/// The skill's name: its folder, and `name` in its frontmatter.
pub const NAME: &str = "wormhole";

/// What marks a `SKILL.md` as installed by Wormhole: a line in the body (not the frontmatter, which
/// agents read each in their own way), by which Update and Uninstall know the folder is ours.
pub const MARKER: &str = "managed-by: wormhole";

/// One file of the skill, with its path inside the skill's folder.
#[derive(Debug, Clone, Copy)]
pub struct File {
    pub path: &'static str,
    pub text: &'static str,
}

/// Every file of the skill; `SKILL.md` first.
pub const FILES: &[File] = &[
    File { path: "SKILL.md", text: include_str!("SKILL.md") },
    File { path: "references/commands.md", text: include_str!("references/commands.md") },
    File { path: "references/recipes.md", text: include_str!("references/recipes.md") },
    File { path: "references/errors.md", text: include_str!("references/errors.md") },
    File { path: "references/mcp.md", text: include_str!("references/mcp.md") },
];

/// The file called `path` (as `FILES` spells it).
pub fn file(path: &str) -> Option<&'static File> {
    FILES.iter().find(|file| file.path == path)
}
