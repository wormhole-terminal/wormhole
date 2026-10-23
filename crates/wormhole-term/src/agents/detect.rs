//! Which coding agent runs in a terminal: look at the terminal's foreground
//! process group (via the shell's `e_tpgid`) and match each member's
//! executable and arguments against the registry.
//!
//! Called every couple of seconds per session, so it sticks to a few cheap
//! process queries (`sys::process`) and never spawns anything.

use std::time::SystemTime;

use super::{AgentId, identify};
use crate::sys::process;

/// The agent in the foreground of the terminal whose shell is `shell_pid`.
pub fn foreground_agent(shell_pid: u32) -> Option<AgentId> {
    let pgid = process::foreground_group(shell_pid)?;
    let mut members = process::group_members(pgid);
    if members.is_empty() {
        members.push(pgid);
    }
    // The group leader first: wrappers (npx, node shims) usually lead.
    members.sort_unstable_by_key(|&pid| (pid != pgid, pid));
    for &pid in &members {
        if let Some(agent) = process::command_line(pid).and_then(|(exe, argv)| identify(&exe, &argv)) {
            return Some(agent);
        }
    }
    // An agent that handed the terminal to a child (an editor, a pager) is
    // still what the session is running: walk up towards the shell, never past
    // it or out of this terminal (that would find whatever launched the app).
    let tty = process::tty(shell_pid)?;
    let mut pid = pgid;
    for _ in 0..8 {
        pid = process::parent(pid).filter(|&p| p > 1 && p != shell_pid && process::tty(p) == Some(tty))?;
        if let Some(agent) = process::command_line(pid).and_then(|(exe, argv)| identify(&exe, &argv)) {
            return Some(agent);
        }
    }
    None
}

/// The program in a terminal's foreground when it isn't the shell: what a row without an agent
/// shows on its second line, and since when it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Foreground {
    /// As [`command_label`] spells it: "cargo test -p wormhole-core".
    pub command: String,
    pub started: SystemTime,
}

/// The program in the foreground of the terminal whose shell is `shell_pid` (the process group
/// leader, which is what the shell started), or `None` while the shell is at its prompt.
pub fn foreground_command(shell_pid: u32) -> Option<Foreground> {
    let pgid = process::foreground_group(shell_pid).filter(|&pgid| pgid != shell_pid)?;
    let (exe, argv) = process::command_line(pgid)?;
    Some(Foreground { command: command_label(&exe, &argv), started: process::start_time(pgid)? })
}

/// Programs that run a script named by their first argument: the script is the command.
const INTERPRETERS: &[&str] = &["node", "python", "python3", "ruby", "perl"];

/// Runtimes whose first argument is usually a subcommand (`bun run dev`, `deno task start`): it
/// names a script only when it looks like a file, so `bun test` stays `bun test`.
const RUNTIMES: &[&str] = &["bun", "deno"];

/// Longest command a row keeps (it truncates on screen long before this).
const COMMAND_MAX: usize = 200;

/// A command line as a row shows it: the program's name (not its path), then its arguments, so
/// `/opt/homebrew/bin/node /opt/homebrew/bin/npm run dev` reads `npm run dev` and a login
/// shell's `-zsh` reads `zsh`.
pub fn command_label(exe: &str, argv: &[String]) -> String {
    let name = |path: &str| {
        let path = path.strip_prefix('-').unwrap_or(path);
        path.rsplit('/').next().unwrap_or(path).to_string()
    };
    let mut words: Vec<String> = match argv.split_first() {
        Some((first, rest)) => std::iter::once(name(first)).chain(rest.iter().cloned()).collect(),
        None => vec![name(exe)],
    };
    let runtime = RUNTIMES.contains(&words[0].as_str());
    if (runtime || INTERPRETERS.contains(&words[0].as_str()))
        && let Some(script) =
            words.get(1).filter(|arg| !arg.starts_with('-') && (!runtime || arg.contains(['.', '/'])))
    {
        let script = name(script);
        words.splice(0..2, [script]);
    }
    let line = words.iter().flat_map(|word| word.split_whitespace()).collect::<Vec<_>>().join(" ");
    match line.char_indices().nth(COMMAND_MAX) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line,
    }
}

/// Whether the shell itself owns the terminal, i.e. nothing runs in it.
pub fn shell_in_foreground(shell_pid: u32) -> bool {
    shell_holds_terminal(shell_pid) == Some(true)
}

/// [`shell_in_foreground`], or `None` where the platform can't tell.
pub(crate) fn shell_holds_terminal(shell_pid: u32) -> Option<bool> {
    process::foreground_group(shell_pid).map(|pgid| pgid == shell_pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real process that calls itself like an agent: `sleep` started
    /// through a symlink named `gemini` (argv[0] is the link). A copied
    /// system binary would be killed by code signing, a symlink isn't.
    #[test]
    fn detects_a_process_by_its_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fake = dir.path().join("gemini");
        crate::sys::fs::symlink("/bin/sleep", &fake).expect("symlink");
        let mut child = std::process::Command::new(&fake).arg("5").spawn().expect("spawn fake agent");
        let mut found = None;
        // The child may not have exec'd yet right after spawn.
        for _ in 0..50 {
            found = process::command_line(child.id()).and_then(|(exe, argv)| identify(&exe, &argv));
            if found.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(found, Some(AgentId::Gemini));
    }

    #[test]
    fn a_process_knows_when_it_started() {
        let started = process::start_time(std::process::id()).expect("own start time");
        let age = std::time::SystemTime::now().duration_since(started).expect("started in the past");
        assert!(age < std::time::Duration::from_secs(3600), "{age:?}");
    }

    #[test]
    fn the_test_process_itself_is_no_agent() {
        let (exe, argv) = process::command_line(std::process::id()).expect("own command line");
        assert!(!exe.is_empty());
        assert_eq!(identify(&exe, &argv), None);
    }
}

#[cfg(test)]
mod label_tests {
    use super::command_label;

    fn label(exe: &str, argv: &[&str]) -> String {
        command_label(exe, &argv.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn a_command_reads_as_typed_without_paths() {
        assert_eq!(
            label(
                "/Users/me/.cargo/bin/cargo",
                &["/Users/me/.cargo/bin/cargo", "test", "-p", "wormhole-core"]
            ),
            "cargo test -p wormhole-core"
        );
        assert_eq!(label("/usr/bin/vim", &["vim", "src/sidebar.rs"]), "vim src/sidebar.rs");
        assert_eq!(label("/usr/bin/ssh", &["ssh", "deploy@prod-1"]), "ssh deploy@prod-1");
        assert_eq!(label("/bin/zsh", &["-zsh"]), "zsh", "a login shell");
        assert_eq!(label("/usr/bin/top", &[]), "top", "no arguments known");
    }

    #[test]
    fn a_script_run_by_an_interpreter_is_named_after_the_script() {
        assert_eq!(
            label("/opt/homebrew/bin/node", &["node", "/opt/homebrew/bin/npm", "run", "dev"]),
            "npm run dev"
        );
        assert_eq!(label("/usr/bin/python3", &["python3", "manage.py", "runserver"]), "manage.py runserver");
        assert_eq!(label("/usr/bin/python3", &["python3", "-m", "http.server"]), "python3 -m http.server");
    }

    #[test]
    fn a_runtimes_subcommand_is_not_taken_for_a_script() {
        assert_eq!(label("/opt/homebrew/bin/bun", &["bun", "run", "dev"]), "bun run dev");
        assert_eq!(label("/opt/homebrew/bin/bun", &["bun", "test"]), "bun test");
        assert_eq!(label("/opt/homebrew/bin/deno", &["deno", "task", "start"]), "deno task start");
        assert_eq!(label("/opt/homebrew/bin/bun", &["bun", "./scripts/seed.ts"]), "seed.ts");
        assert_eq!(label("/opt/homebrew/bin/deno", &["deno", "main.ts"]), "main.ts");
    }

    #[test]
    fn whitespace_collapses_and_long_lines_are_capped() {
        assert_eq!(label("/bin/echo", &["echo", "a  b", "c\nd"]), "echo a b c d");
        let long = label("/bin/echo", &["echo", &"x".repeat(500)]);
        assert_eq!(long.chars().count(), 201);
        assert!(long.ends_with('…'));
    }
}
