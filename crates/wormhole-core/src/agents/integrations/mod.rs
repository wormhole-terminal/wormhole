//! Status integrations: hooks Wormhole installs into an agent's own
//! config so the agent reports when it's working, waiting or done.
//!
//! Rules for every integration: merge into the existing config, only ever
//! add or remove entries recognisable as ours (they all call
//! `$WORMHOLE_BIN hook`), keep a backup of the previous file, be
//! idempotent, and do nothing outside Wormhole (the command checks
//! `$WORMHOLE_BIN` first). All functions take the home directory so tests
//! run against temp dirs.

mod files;
mod json_hooks;
mod own_files;
mod payload;
mod signals;
mod status_line;

use std::path::{Path, PathBuf};

use super::AgentId;

pub use payload::describe;
pub use signals::{Signal, answer_key, signal};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    NotInstalled,
    Installed,
    /// Some of our entries are missing (e.g. after an upgrade added events).
    Partial,
}

/// Where an integration lives and how it's written.
pub struct Spec {
    pub agent: AgentId,
    kind: Kind,
    /// Something the user must do in the agent afterwards, for the UI.
    pub note: Option<&'static str>,
}

enum Kind {
    /// Entries merged into a shared JSON file (path relative to home).
    Json(&'static str, json_hooks::Format),
    /// Files of our own in a folder the agent loads from.
    Files(own_files::Files),
}

static SPECS: &[Spec] = &[
    Spec {
        agent: AgentId::Claude,
        kind: Kind::Json(".claude/settings.json", json_hooks::CLAUDE),
        note: None,
    },
    Spec {
        agent: AgentId::Codex,
        kind: Kind::Json(".codex/hooks.json", json_hooks::CODEX),
        note: Some("Codex asks you to review and trust new hooks once (/hooks)."),
    },
    Spec {
        agent: AgentId::Gemini,
        kind: Kind::Json(".gemini/settings.json", json_hooks::GEMINI),
        note: None,
    },
    Spec { agent: AgentId::Copilot, kind: Kind::Files(own_files::copilot), note: None },
    Spec { agent: AgentId::OpenCode, kind: Kind::Files(own_files::opencode), note: None },
    Spec { agent: AgentId::Goose, kind: Kind::Files(own_files::goose), note: None },
    Spec { agent: AgentId::Qwen, kind: Kind::Json(".qwen/settings.json", json_hooks::QWEN), note: None },
    Spec { agent: AgentId::Droid, kind: Kind::Json(".factory/hooks.json", json_hooks::DROID), note: None },
];

pub fn spec(agent: AgentId) -> Option<&'static Spec> {
    SPECS.iter().find(|s| s.agent == agent)
}

/// Where agent configs live: `$WORMHOLE_AGENT_HOME` (for isolated test
/// instances) or the user's home.
pub fn home() -> PathBuf {
    std::env::var_os("WORMHOLE_AGENT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
}

impl Spec {
    /// The file this integration edits or adds (the first of them).
    pub fn path(&self, home: &Path) -> PathBuf {
        let rel = match &self.kind {
            Kind::Json(rel, _) => *rel,
            Kind::Files(files) => files().first().map(|(rel, _)| *rel).unwrap_or_default(),
        };
        home.join(rel)
    }

    /// [`path`](Self::path) for the UI (`~/.claude/settings.json`).
    pub fn display_path(&self, home: &Path) -> String {
        files::display(&self.path(home))
    }

    pub fn state(&self, home: &Path) -> Result<State, String> {
        match &self.kind {
            Kind::Json(rel, format) => json_hooks::state(format, &home.join(rel)),
            Kind::Files(files) => own_files::state(*files, home),
        }
    }

    /// Whether hooks an earlier version installed under its old names are still here.
    pub fn has_legacy(&self, home: &Path) -> bool {
        match &self.kind {
            Kind::Json(rel, format) => json_hooks::has_legacy(format, &home.join(rel)),
            Kind::Files(files) => own_files::has_legacy(*files, home),
        }
    }

    pub fn install(&self, home: &Path) -> Result<(), String> {
        match &self.kind {
            Kind::Json(rel, format) => json_hooks::install(format, self.agent, &home.join(rel)),
            Kind::Files(files) => own_files::install(*files, home),
        }
    }

    pub fn uninstall(&self, home: &Path) -> Result<(), String> {
        match &self.kind {
            Kind::Json(rel, format) => json_hooks::uninstall(format, &home.join(rel)),
            Kind::Files(files) => own_files::uninstall(*files, home),
        }
    }
}

/// Marker in every command we install; how we recognise our own entries.
const MARKER: &str = "WORMHOLE_BIN\" hook";
/// The marker of the hooks versions up to 0.2 installed, when the app went by its codename. Still
/// ours: they are replaced by today's on install (and at launch, [`upgrade_legacy`]) and removed on
/// uninstall.
const LEGACY_MARKER: &str = "ANDROMEDA_BIN\" hook";

/// The shell command a hook runs. A no-op outside Wormhole, and it never
/// blocks or fails the agent. Claude's stays in its original form so hooks
/// installed by earlier versions keep being recognised.
fn hook_command(agent: AgentId, event: &str) -> String {
    let args = match agent {
        AgentId::Claude => event.to_string(),
        other => format!("--agent {} {event}", agent_arg(other)),
    };
    let command = format!("[ -n \"$WORMHOLE_BIN\" ] && \"$WORMHOLE_BIN\" hook {args} || true");
    // Qwen Code expects a JSON object on stdout from a successful hook.
    if agent == AgentId::Qwen { format!("{command}; printf '{{}}'") } else { command }
}

fn agent_arg(agent: AgentId) -> String {
    serde_json::to_value(agent).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

fn is_our_command(command: &str) -> bool {
    command.contains(MARKER) || is_legacy_command(command)
}

fn is_legacy_command(command: &str) -> bool {
    command.contains(LEGACY_MARKER)
}

/// Replace the hooks an earlier version installed under its old names with today's, for every
/// agent that has any; the others are left alone. What happened to each, for the log.
pub fn upgrade_legacy(home: &Path) -> Vec<(AgentId, Result<(), String>)> {
    SPECS.iter().filter(|spec| spec.has_legacy(home)).map(|spec| (spec.agent, spec.install(home))).collect()
}

#[cfg(test)]
mod tests;
