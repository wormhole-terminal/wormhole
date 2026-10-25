//! `[permissions]`: the checks that its entries are well formed, and the summary a person reads
//! before enabling a Probe: what it may do, how that is confirmed, which pairs are dangerous
//! together, and which programs run code (section 2 of the design).

use std::collections::BTreeSet;

use super::Findings;
use super::argv;
use super::paths;
use crate::probe::manifest::{AgentStatus, TerminalAccess};
use crate::probe::read::ProbeFolder;

/// How the app asks the user to approve a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    /// A click: for what harms little alone.
    Click,
    /// Touch ID, which neither a script nor an agent in a terminal can press.
    TouchId,
}

/// One thing the manifest asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredPermission {
    /// The `[permissions]` key, or `main` for code.
    pub key: &'static str,
    /// What exactly: a program, a host, a variable name.
    pub detail: String,
    pub confirmation: Confirmation,
}

/// A pair of permissions that is more than its parts: what is on the screen can be sent out. Approving
/// one needs Touch ID even when each part alone would be a click.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combination {
    pub description: String,
}

/// What a program in `permissions.process` can do besides what its argv says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramClass {
    /// `git`, which the app runs itself, read-only.
    ReadOnlyGit,
    /// A script of the Probe's own folder, inside the approved fingerprint.
    OwnScript,
    /// A shell, an interpreter or a launcher: it can run anything.
    RunsAnything,
    /// A tool that runs code the folder it starts in brings (a Makefile, a build script, a config).
    RunsFolderCode,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramNote {
    pub program: String,
    pub class: ProgramClass,
}

/// What a Probe would be allowed to do.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Summary {
    /// The Probe has code (E2) and would run it inside the app.
    pub has_code: bool,
    pub permissions: Vec<DeclaredPermission>,
    pub combinations: Vec<Combination>,
    /// One per entry of `permissions.process`.
    pub programs: Vec<ProgramNote>,
}

/// Shells, interpreters and launchers: whatever they are given, they run.
const RUNS_ANYTHING: &[&str] = &[
    "sh",
    "bash",
    "zsh",
    "dash",
    "fish",
    "ksh",
    "csh",
    "tcsh",
    "node",
    "deno",
    "bun",
    "ruby",
    "perl",
    "php",
    "lua",
    "awk",
    "gawk",
    "osascript",
    "env",
    "xargs",
    "open",
    "find",
    "nohup",
    "sudo",
    "su",
    "doas",
    "exec",
    "eval",
    "time",
    "timeout",
    "nice",
    "caffeinate",
    "script",
    "expect",
    "tmux",
    "screen",
    "xcrun",
];
/// Tools that read the folder they start in and run what it says.
const RUNS_FOLDER_CODE: &[&str] = &[
    "make", "gmake", "ninja", "just", "rake", "npm", "npx", "pnpm", "yarn", "cargo", "ssh", "scp", "sftp",
    "rsync", "gradle", "mvn", "go", "pip",
];

pub(crate) fn classify(program: &str) -> ProgramClass {
    if program.starts_with("./") {
        return ProgramClass::OwnScript;
    }
    if program == "git" {
        return ProgramClass::ReadOnlyGit;
    }
    let name = program.to_ascii_lowercase();
    if RUNS_ANYTHING.contains(&name.as_str()) || name.starts_with("python") || name.starts_with("pypy") {
        ProgramClass::RunsAnything
    } else if RUNS_FOLDER_CODE.contains(&name.as_str()) {
        ProgramClass::RunsFolderCode
    } else {
        ProgramClass::Other
    }
}

/// A host as `permissions.open` and `net` spell it: lower-case labels, no scheme, port or path.
fn is_host(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 253
        && text.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && !text.starts_with(['.', '-'])
        && !text.ends_with(['.', '-'])
        && !text.contains("..")
}

fn is_env_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(super) fn check(folder: &ProbeFolder, out: &mut Findings) {
    let m = &folder.manifest;
    let p = &m.permissions;
    for (ix, program) in p.process.iter().enumerate() {
        let at = format!("permissions.process[{ix}]");
        if program.is_empty() || program.chars().any(|c| c.is_control() || c.is_whitespace()) {
            out.error(
                "permission-process",
                at,
                "must be a program name without spaces or control characters",
            );
        } else if let Some(script) = program.strip_prefix("./") {
            match paths::inside_folder(script) {
                Err(why) => out.error("permission-process", at, format!("`{program}` {why}")),
                Ok(plain) if folder.file(&plain).is_none() => out.error(
                    "permission-process-missing",
                    at,
                    format!("`{program}` is not a file of the Probe's folder"),
                ),
                Ok(_) => {}
            }
        } else if program.contains('/') {
            out.error(
                "permission-process",
                at,
                format!("`{program}`: a program is a bare name that Wormhole finds, or `./path` inside the Probe's folder"),
            );
        }
    }
    // A listed program nothing runs is an ask without a reason; with code, `$.process.run` may use it.
    if m.probe.main.is_none() {
        let used: BTreeSet<&str> = m
            .contributes
            .commands
            .iter()
            .filter_map(|c| c.run.as_deref())
            .chain(m.contributes.status.iter().filter_map(|s| s.run.as_deref()))
            .filter_map(argv::program_of)
            .collect();
        for (ix, program) in p.process.iter().enumerate() {
            if !used.contains(program.as_str()) {
                out.warning(
                    "permission-process-unused",
                    format!("permissions.process[{ix}]"),
                    format!("`{program}` is listed but no command or status item runs it; ask for the least"),
                );
            }
        }
    }
    for (ix, name) in p.env.iter().enumerate() {
        let at = format!("permissions.env[{ix}]");
        if !is_env_name(name) {
            out.error("permission-env", at, format!("`{name}` is not a variable name"));
        } else if name.starts_with("WORMHOLE_") {
            out.error("permission-env-reserved", at, "variables starting with WORMHOLE_ are set by the app");
        }
    }
    if !p.env.is_empty() && p.process.is_empty() && m.probe.main.is_none() {
        out.warning(
            "permission-env-unused",
            "permissions.env",
            "no program is permitted to receive these variables",
        );
    }
    for (ix, host) in p.open.iter().enumerate() {
        if !is_host(host) {
            out.error(
                "permission-open",
                format!("permissions.open[{ix}]"),
                format!("`{host}` is not a host: lower-case labels only, no scheme, port or path"),
            );
        }
    }
    for (ix, host) in p.net.iter().enumerate() {
        if !is_host(host) {
            out.error("permission-net", format!("permissions.net[{ix}]"), format!("`{host}` is not a host"));
        }
    }
    for (ix, entry) in p.agents.iter().enumerate() {
        let own = entry.strip_prefix("install-hooks:").is_some_and(|id| {
            m.contributes.agents.iter().any(|a| a.id == id && a.status == AgentStatus::Hooks)
        });
        if !own {
            out.error(
                "permission-agents",
                format!("permissions.agents[{ix}]"),
                format!("`{entry}` must be `install-hooks:<id>` of one of this Probe's agents with `status = \"hooks\"`"),
            );
        }
    }
    for (ix, entry) in p.fs.iter().enumerate() {
        if !(entry.starts_with("read:") || entry.starts_with("write:")) {
            out.error(
                "permission-fs",
                format!("permissions.fs[{ix}]"),
                format!("`{entry}` must be `read:<path>` or `write:<path>`"),
            );
        }
    }
}

pub(super) fn summarize(folder: &ProbeFolder) -> Summary {
    let m = &folder.manifest;
    let p = &m.permissions;
    let mut permissions = Vec::new();
    let mut add = |key, detail: &str, confirmation| {
        permissions.push(DeclaredPermission { key, detail: detail.to_string(), confirmation });
    };
    if m.probe.main.is_some() {
        add("main", "run code inside Wormhole", Confirmation::TouchId);
    }
    for program in &p.process {
        add("process", program, Confirmation::TouchId);
    }
    for name in &p.env {
        add("env", name, Confirmation::TouchId);
    }
    for host in &p.open {
        add("open", host, Confirmation::Click);
    }
    if p.notify {
        add("notify", "show notifications", Confirmation::Click);
    }
    for entry in &p.agents {
        add("agents", entry, Confirmation::TouchId);
    }
    for access in &p.terminal {
        match access {
            TerminalAccess::Read => add("terminal", "read what is on your terminals", Confirmation::TouchId),
            TerminalAccess::Write => add("terminal", "type into your terminals", Confirmation::TouchId),
        }
    }
    for host in &p.net {
        add("net", host, Confirmation::Click);
    }
    for entry in &p.fs {
        let confirmation =
            if entry.starts_with("write:") { Confirmation::TouchId } else { Confirmation::Click };
        add("fs", entry, confirmation);
    }

    let mut combinations = Vec::new();
    let reads: Vec<&str> = [
        p.terminal.contains(&TerminalAccess::Read).then_some("what is on your terminals"),
        p.fs.iter().any(|f| f.starts_with("read:")).then_some("the files it may read"),
    ]
    .into_iter()
    .flatten()
    .collect();
    for what in reads {
        if !p.net.is_empty() {
            combinations
                .push(Combination { description: format!("can send {what} to {}", p.net.join(", ")) });
        }
        if !p.open.is_empty() {
            combinations.push(Combination {
                description: format!("can send {what} out in the links it opens on {}", p.open.join(", ")),
            });
        }
        if !p.process.is_empty() {
            combinations.push(Combination {
                description: format!("can send {what} to anything, through {}", p.process.join(", ")),
            });
        }
    }
    if !p.env.is_empty() && !p.process.is_empty() {
        combinations.push(Combination {
            description: format!(
                "can hand these secrets to the programs it may run ({}): {}",
                p.process.join(", "),
                p.env.join(", ")
            ),
        });
    }
    let programs = p
        .process
        .iter()
        .map(|program| ProgramNote { program: program.clone(), class: classify(program) })
        .collect();
    Summary { has_code: m.probe.main.is_some(), permissions, combinations, programs }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_that_run_code_are_told_apart() {
        for (program, class) in [
            ("git", ProgramClass::ReadOnlyGit),
            ("./scripts/x.sh", ProgramClass::OwnScript),
            ("sh", ProgramClass::RunsAnything),
            ("bash", ProgramClass::RunsAnything),
            ("python3.12", ProgramClass::RunsAnything),
            ("Python", ProgramClass::RunsAnything),
            ("osascript", ProgramClass::RunsAnything),
            ("env", ProgramClass::RunsAnything),
            ("xargs", ProgramClass::RunsAnything),
            ("open", ProgramClass::RunsAnything),
            ("find", ProgramClass::RunsAnything),
            ("make", ProgramClass::RunsFolderCode),
            ("npm", ProgramClass::RunsFolderCode),
            ("npx", ProgramClass::RunsFolderCode),
            ("cargo", ProgramClass::RunsFolderCode),
            ("ssh", ProgramClass::RunsFolderCode),
            ("jq", ProgramClass::Other),
            ("say", ProgramClass::Other),
        ] {
            assert_eq!(classify(program), class, "{program}");
        }
    }

    #[test]
    fn hosts_are_plain_lower_case_names() {
        assert!(is_host("acme.atlassian.net"));
        assert!(is_host("localhost"));
        for bad in
            ["", "Acme.net", "https://acme.net", "acme.net/x", "acme.net:80", "-a.net", "a..net", "a.net."]
        {
            assert!(!is_host(bad), "{bad}");
        }
    }

    #[test]
    fn variable_names_are_identifiers() {
        assert!(is_env_name("ACME_TOKEN"));
        assert!(is_env_name("_x1"));
        for bad in ["", "1X", "A-B", "A B", "A=B"] {
            assert!(!is_env_name(bad), "{bad}");
        }
    }
}
