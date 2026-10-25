//! `[[contributes.agents]]`: a coding agent Wormhole should recognise (section 3.4 of the design).
//! Detection and look need no permission; only the icon (a file of the folder) and the way the
//! agent reports its status carry rules.

use wormhole_term::agents::AGENTS;

use super::Findings;
use super::paths;
use super::svg;
use crate::probe::manifest::{AgentDef, AgentStatus};
use crate::probe::read::ProbeFolder;

pub(super) fn check(folder: &ProbeFolder, out: &mut Findings) {
    for (ix, agent) in folder.manifest.contributes.agents.iter().enumerate() {
        check_agent(folder, agent, &format!("contributes.agents[{ix}]"), out);
    }
}

fn bare_name(text: &str) -> bool {
    !text.is_empty() && !text.contains('/') && !text.chars().any(|c| c.is_control() || c.is_whitespace())
}

fn check_agent(folder: &ProbeFolder, agent: &AgentDef, at: &str, out: &mut Findings) {
    if agent.name.trim().is_empty() || agent.name.chars().any(char::is_control) {
        out.error("agent-name", format!("{at}.name"), "must be a name without control characters");
    }
    if !bare_name(&agent.command) {
        out.error(
            "agent-command",
            format!("{at}.command"),
            "must be the program's name, as typed: no path, spaces or control characters",
        );
    }
    if agent.executables.is_empty() {
        out.error(
            "agent-executables",
            format!("{at}.executables"),
            "needs the names the agent runs as, so it can be recognised",
        );
    }
    for (ix, name) in agent.executables.iter().enumerate() {
        if !bare_name(name) {
            out.error(
                "agent-executables",
                format!("{at}.executables[{ix}]"),
                format!("`{name}` must be a program name: no path, spaces or control characters"),
            );
        }
    }
    for (ix, package) in agent.packages.iter().enumerate() {
        if package.is_empty() || package.chars().any(char::is_control) {
            out.error(
                "agent-packages",
                format!("{at}.packages[{ix}]"),
                "must not be empty or hold control characters",
            );
        }
    }
    let monogram_ok = agent.monogram.len() == 2 && agent.monogram.bytes().all(|b| b.is_ascii_uppercase());
    if !monogram_ok {
        out.error(
            "agent-monogram",
            format!("{at}.monogram"),
            format!("`{}` must be two capital letters, such as `AA`", agent.monogram),
        );
    }
    // A built-in name cannot be taken: a Probe adds agents, it does not redirect the app's own.
    for builtin in AGENTS {
        let clash = std::iter::once(&agent.command)
            .chain(&agent.executables)
            .any(|name| builtin.command == name || builtin.executables.contains(&name.as_str()));
        if clash {
            out.error(
                "agent-builtin-name",
                format!("{at}.executables"),
                format!("`{}` is a name the built-in agent {} already has", agent.command, builtin.name),
            );
            break;
        }
    }
    if let Some(icon) = &agent.icon {
        check_icon(folder, icon, &format!("{at}.icon"), out);
    }
    if agent.status == AgentStatus::Hooks {
        let wanted = format!("install-hooks:{}", agent.id);
        if !folder.manifest.permissions.agents.contains(&wanted) {
            out.error(
                "agent-hooks-permission",
                format!("{at}.status"),
                format!(
                    "`status = \"hooks\"` writes hook entries; add `agents = [\"{wanted}\"]` to [permissions]"
                ),
            );
        }
        out.warning(
            "agent-hooks-unsupported",
            format!("{at}.status"),
            "the file and dialect of the hooks are not part of the format yet (step E1.4), so nothing would be installed",
        );
    }
}

fn check_icon(folder: &ProbeFolder, icon: &str, path: &str, out: &mut Findings) {
    let Some(file) = paths::file_of(folder, icon, path, ("agent-icon-path", "agent-icon-missing"), out)
    else {
        return;
    };
    if file.bytes().len() > svg::MAX_BYTES {
        out.error(
            "agent-icon-size",
            path,
            format!("{} bytes; an icon may have {} KiB", file.bytes().len(), svg::MAX_BYTES / 1024),
        );
        return;
    }
    match file.text() {
        None => out.error("agent-icon-forbidden", path, "is not valid UTF-8 text"),
        Some(text) => {
            if let Some(why) = svg::problem(text) {
                out.error("agent-icon-forbidden", path, format!("`{icon}` {why}"));
            }
        }
    }
}
