//! Agents that load every hook file or plugin from a folder (Copilot CLI,
//! Goose, OpenCode): we add files of our own there instead of editing the
//! user's, and remove only files that still carry our marker.

use std::path::Path;

use serde_json::json;

use super::{AgentId, LEGACY_MARKER, MARKER, State, files, hook_command};

/// A folder that belongs entirely to us.
const OWN_DIR: &str = ".agents/plugins/wormhole";
/// It and every file of ours were named after the app's old codename up to 0.2.
const LEGACY_OWN_DIR: &str = ".agents/plugins/andromeda";

/// Where an earlier version put each of these files: the same paths under the old name.
fn legacy_paths(files: Files) -> Vec<String> {
    files().into_iter().map(|(rel, _)| rel.replace("wormhole", "andromeda")).collect()
}

/// A file an earlier version installed: in its own folder, or carrying its marker.
fn is_legacy_ours(home: &Path, path: &Path) -> bool {
    path.is_file()
        && (path.starts_with(home.join(LEGACY_OWN_DIR))
            || std::fs::read_to_string(path).is_ok_and(|text| {
                text.contains(LEGACY_MARKER) || text.contains(&LEGACY_MARKER.replace('"', "\\\""))
            }))
}

pub fn has_legacy(files: Files, home: &Path) -> bool {
    legacy_paths(files).iter().any(|rel| is_legacy_ours(home, &home.join(rel)))
}

/// Remove the files an earlier version installed, and its folder once that is empty.
fn remove_legacy(files: Files, home: &Path) -> Result<(), String> {
    for rel in legacy_paths(files) {
        let path = home.join(&rel);
        if is_legacy_ours(home, &path) {
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", files::display(&path)))?;
            prune(&path, &home.join(LEGACY_OWN_DIR));
        }
    }
    Ok(())
}

/// Remove the empty folders between `path` and `own` (inclusive), never above it.
fn prune(path: &Path, own: &Path) {
    let mut dir = path.parent();
    while let Some(d) = dir.filter(|d| d.starts_with(own)) {
        if std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

/// Files (relative to the home directory) and their contents.
pub type Files = fn() -> Vec<(&'static str, String)>;

pub fn copilot() -> Vec<(&'static str, String)> {
    let hook = |event: &str| json!([{ "type": "command", "bash": hook_command(AgentId::Copilot, event), "timeoutSec": 5 }]);
    let hooks = json!({
        "version": 1,
        "hooks": {
            "userPromptSubmitted": hook("userPromptSubmitted"),
            "postToolUse": hook("postToolUse"),
            "notification": hook("notification"),
            "agentStop": hook("agentStop"),
            "sessionEnd": hook("sessionEnd"),
        }
    });
    vec![(".copilot/hooks/wormhole.json", pretty(&hooks))]
}

pub fn goose() -> Vec<(&'static str, String)> {
    let manifest = json!({
        "name": "wormhole",
        "version": "1.0.0",
        "description": "Reports goose's status to the Wormhole sidebar (does nothing outside Wormhole)"
    });
    let hook = |event: &str| json!([{ "hooks": [{ "type": "command", "command": hook_command(AgentId::Goose, event), "timeout": 5 }] }]);
    let hooks = json!({
        "hooks": {
            "UserPromptSubmit": hook("UserPromptSubmit"),
            "PostToolUse": hook("PostToolUse"),
            "Stop": hook("Stop"),
            "SessionEnd": hook("SessionEnd"),
        }
    });
    vec![
        (".agents/plugins/wormhole/plugin.json", pretty(&manifest)),
        (".agents/plugins/wormhole/hooks/hooks.json", pretty(&hooks)),
    ]
}

pub fn opencode() -> Vec<(&'static str, String)> {
    vec![(".config/opencode/plugins/wormhole.js", OPENCODE_PLUGIN.replace("{MARKER}", MARKER))]
}

/// Our marker sits in a comment so the file is recognisable as ours.
const OPENCODE_PLUGIN: &str = r#"// Installed by Wormhole ({MARKER}): reports OpenCode's status to the
// Wormhole sidebar. Does nothing outside Wormhole. Remove it in
// Wormhole → Settings → Coding agents (or delete this file).
import { spawn } from "node:child_process";

export const WormholeStatus = async () => {
  const bin = process.env.WORMHOLE_BIN;
  if (!bin) return {};
  const send = (event) => {
    try {
      spawn(bin, ["hook", "--agent", "opencode", event], { stdio: "ignore" }).unref();
    } catch {}
  };
  return {
    event: async ({ event }) => {
      if (event.type === "session.status") {
        if (event.properties?.status?.type === "busy") send("session.busy");
      } else if (["session.idle", "permission.asked", "permission.replied"].includes(event.type)) {
        send(event.type);
      }
    },
  };
};
"#;

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default() + "\n"
}

/// Files carry our marker; everything in our own folder is ours anyway.
fn is_ours(home: &Path, path: &Path) -> bool {
    path.is_file()
        && (path.starts_with(home.join(OWN_DIR))
            // In JSON files the marker's quote is escaped.
            || std::fs::read_to_string(path)
                .is_ok_and(|text| text.contains(MARKER) || text.contains(&MARKER.replace('"', "\\\""))))
}

pub fn state(files: Files, home: &Path) -> Result<State, String> {
    let list = files();
    if has_legacy(files, home) {
        // An earlier version's files still work but are due to be replaced.
        return Ok(State::Partial);
    }
    let present = list.iter().filter(|(rel, _)| is_ours(home, &home.join(rel))).count();
    Ok(match present {
        0 => State::NotInstalled,
        n if n == list.len() => State::Installed,
        _ => State::Partial,
    })
}

pub fn install(files: Files, home: &Path) -> Result<(), String> {
    let list = files();
    // Check everything first so a conflict never leaves half an install.
    for (rel, _) in &list {
        let path = home.join(rel);
        if path.exists() && !is_ours(home, &path) {
            return Err(format!("{} already exists and isn't Wormhole's", files::display(&path)));
        }
    }
    for (rel, text) in list {
        files::write_text(&home.join(rel), &text)?;
    }
    remove_legacy(files, home)
}

pub fn uninstall(files: Files, home: &Path) -> Result<(), String> {
    for (rel, _) in files() {
        let path = home.join(rel);
        if is_ours(home, &path) {
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", files::display(&path)))?;
            let _ = std::fs::remove_file(files::with_suffix(&path, files::BACKUP_SUFFIX));
            // Our own plugin folder (Goose) goes too, once nothing else is in it.
            prune(&path, &home.join(OWN_DIR));
        }
    }
    remove_legacy(files, home)
}
