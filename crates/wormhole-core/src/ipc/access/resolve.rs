//! Who is behind a `wh` (`agent-permissions.md`, 4.1 and 4.3): walk the caller's process chain,
//! nearest first, to the first program the agent registry knows, and say how it is told from another.
//!
//! A pure function over a [`Processes`] the app implements with the system's, so it is tested against a
//! made-up process table. Nothing the caller says counts: not `from`, not the name an agent gives
//! itself. The chain and the signature do, and a pid is checked to be the same process before and after
//! it is read.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::grants::ProcessRef;
use super::identity::{Identity, Kind};
use crate::agents::{self, AgentId};
use crate::platform::codesign::{Signature, SignerKind};

/// What the walk asks of the system.
pub trait Processes {
    fn executable(&self, pid: u32) -> Option<PathBuf>;
    fn command_line(&self, pid: u32) -> Option<(String, Vec<String>)>;
    fn start_time(&self, pid: u32) -> Option<SystemTime>;
    fn signature(&self, pid: u32) -> Signature;
    /// Whether `pid` is the shell of one of Wormhole's terminals.
    fn is_terminal_shell(&self, pid: u32) -> bool;
    /// The folder of the nearest `package.json` at or above a script, else the script's own folder.
    fn package_folder(&self, script: &Path) -> PathBuf;
    /// A file's size and modification time (Unix seconds), which pin an unsigned program.
    fn stamp(&self, path: &Path) -> Option<(u64, i64)>;
}

/// A program, as the question shows it. Never trusted: it is what the system said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub pid: u32,
    /// Its executable, or the script an interpreter runs.
    pub path: PathBuf,
    /// The signature of the executable (the interpreter's, for a script).
    pub signature: Signature,
    /// For a script, the interpreter it runs on ("node").
    pub interpreter: Option<String>,
    pub started: SystemTime,
}

/// Who is asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub identity: Identity,
    /// What the user knows it as: the agent's name, or "Scripts".
    pub label: String,
    /// The agent's process, for a grant "for this session". `None` for Scripts.
    pub process: Option<ProcessRef>,
    /// The agent itself, or for Scripts the program that made the request (the first one up the chain
    /// that is not `wh`, a shell or a wrapper), when there is one.
    pub program: Option<Program>,
    /// A `sudo` stands between the request and the agent: it runs as root. Shown, and never makes a
    /// grant broader.
    pub elevated: bool,
    /// Another agent is above it ("started by Codex CLI").
    pub started_by: Option<String>,
    /// The agent's registry names who usually signs it, and this is not that.
    pub unusual_signer: bool,
}

/// The process changed hands while it was being read: ask again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Changed;

/// The chain is cut at this many processes: a bound on what one request costs.
const CHAIN_MAX: usize = crate::ipc::ANCESTORS_MAX + 1;

/// Programs that pass their arguments on and are never the identity.
const WRAPPERS: &[&str] =
    &["env", "command", "exec", "nice", "nohup", "time", "timeout", "xargs", "caffeinate", "script", "sudo"];

const SHELLS: &[&str] =
    &["sh", "bash", "zsh", "fish", "dash", "ksh", "tcsh", "csh", "nu", "xonsh", "pwsh", "elvish"];

/// Names `wh` runs as: the command, and the app's executable it is a link to.
const WH_NAMES: &[&str] = &["wh", "wormhole"];

/// Who is behind `chain` (the caller first, then its parents): see the module's notes.
pub fn resolve(chain: &[u32], procs: &dyn Processes) -> Result<Resolved, Changed> {
    let mut elevated = false;
    let mut requester: Option<Program> = None;
    let mut found: Option<(AgentId, u32, PathBuf, Vec<String>)> = None;
    let mut second: Option<AgentId> = None;
    for &pid in chain.iter().take(CHAIN_MAX) {
        let Some(exe) = procs.executable(pid) else { continue };
        let (exe_text, argv) =
            procs.command_line(pid).unwrap_or_else(|| (exe.to_string_lossy().into_owned(), Vec::new()));
        if let Some(agent) = agents::identify(&exe_text, &argv) {
            if found.is_some() {
                second = Some(agent);
                break;
            }
            found = Some((agent, pid, exe, argv));
            continue;
        }
        if found.is_some() {
            continue;
        }
        let name = base_name(&exe);
        if name == "sudo" {
            elevated = true;
        }
        if procs.is_terminal_shell(pid) {
            break;
        }
        if WH_NAMES.contains(&name.as_str())
            || WRAPPERS.contains(&name.as_str())
            || SHELLS.contains(&name.as_str())
        {
            continue;
        }
        if requester.is_none() {
            requester = program_of(pid, &exe, &argv, procs).ok();
        }
    }
    let Some((agent, pid, exe, argv)) = found else {
        return Ok(Resolved {
            identity: Identity::scripts(),
            label: "Scripts".to_string(),
            process: None,
            program: requester,
            elevated,
            started_by: None,
            unusual_signer: false,
        });
    };
    let before = procs.start_time(pid).ok_or(Changed)?;
    let program = program_of(pid, &exe, &argv, procs)?;
    if procs.start_time(pid) != Some(before) {
        return Err(Changed);
    }
    let (identity, unusual_signer) = identity_of(agent, &program, procs);
    Ok(Resolved {
        identity,
        label: agents::agent(agent).name.to_string(),
        process: Some(ProcessRef { pid, started: before }),
        program: Some(program),
        elevated,
        started_by: second.map(|other| agents::agent(other).name.to_string()),
        unusual_signer,
    })
}

fn base_name(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// What the system says about the program at `pid`.
fn program_of(pid: u32, exe: &Path, argv: &[String], procs: &dyn Processes) -> Result<Program, Changed> {
    let started = procs.start_time(pid).ok_or(Changed)?;
    let exe_text = exe.to_string_lossy();
    let script = agents::interpreted_script(&exe_text, argv);
    let (path, interpreter) = match script {
        Some(script) => (PathBuf::from(script), Some(base_name(exe))),
        None => (exe.to_path_buf(), None),
    };
    Ok(Program { pid, path, signature: procs.signature(pid), interpreter, started })
}

/// The identity of `agent` running as `program`, and whether its signature is not the usual one.
fn identity_of(agent: AgentId, program: &Program, procs: &dyn Processes) -> (Identity, bool) {
    let registry = agents::agent(agent);
    let base = Identity { kind: Kind::Unsigned, agent: Some(agent), ..Identity::scripts() };
    let signature = &program.signature;
    if program.interpreter.is_some() {
        let folder = procs.package_folder(&program.path);
        let identity = Identity { kind: Kind::Script, pin: folder.to_string_lossy().into_owned(), ..base };
        return (identity, false);
    }
    let identifier = signature.identifier.clone().unwrap_or_default();
    let team = signature.team_id.clone().unwrap_or_default();
    match signature.kind {
        SignerKind::DeveloperId => {
            let usual = registry.signers.is_empty()
                || registry.signers.iter().any(|(id, team_id)| *id == identifier && *team_id == team);
            (Identity { kind: Kind::DeveloperId, identifier, team_id: team, ..base }, !usual)
        }
        SignerKind::AppleSystem => (Identity { kind: Kind::Apple, identifier, ..base }, false),
        SignerKind::AdHoc | SignerKind::Other => {
            let pin = signature.cdhash.clone().unwrap_or_default();
            (Identity { kind: Kind::AdHoc, pin, ..base }, !registry.signers.is_empty())
        }
        SignerKind::Unsigned | SignerKind::Unreadable => {
            let stamp = procs.stamp(&program.path).map(|(size, time)| format!("|{size}|{time}"));
            let pin = format!("{}{}", program.path.display(), stamp.unwrap_or_default());
            (Identity { kind: Kind::Unsigned, pin, ..base }, !registry.signers.is_empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    use super::{Changed, Processes, Resolved, resolve};
    use crate::agents::AgentId;
    use crate::ipc::access::identity::Kind;
    use crate::platform::codesign::{Signature, SignerKind};

    #[derive(Clone)]
    struct Proc {
        exe: &'static str,
        argv: Vec<&'static str>,
        signature: Signature,
        started: u64,
        shell: bool,
    }

    #[derive(Default)]
    struct Table {
        procs: HashMap<u32, Proc>,
        /// A pid whose start time changes on the second read.
        reused: std::cell::RefCell<Option<u32>>,
        reads: std::cell::RefCell<HashMap<u32, u32>>,
    }

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    impl Table {
        fn with(mut self, pid: u32, exe: &'static str, argv: &[&'static str]) -> Self {
            let signature = Signature::of(SignerKind::Unsigned);
            self.procs.insert(pid, Proc { exe, argv: argv.to_vec(), signature, started: 10, shell: false });
            self
        }
        fn signed(mut self, pid: u32, signature: Signature) -> Self {
            self.procs.get_mut(&pid).expect("a process").signature = signature;
            self
        }
        fn shell(mut self, pid: u32) -> Self {
            self.procs.get_mut(&pid).expect("a process").shell = true;
            self
        }
    }

    impl Processes for Table {
        fn executable(&self, pid: u32) -> Option<PathBuf> {
            self.procs.get(&pid).map(|p| PathBuf::from(p.exe))
        }
        fn command_line(&self, pid: u32) -> Option<(String, Vec<String>)> {
            self.procs.get(&pid).map(|p| (p.exe.to_string(), p.argv.iter().map(|a| a.to_string()).collect()))
        }
        fn start_time(&self, pid: u32) -> Option<SystemTime> {
            let process = self.procs.get(&pid)?;
            let mut reads = self.reads.borrow_mut();
            let count = reads.entry(pid).or_default();
            *count += 1;
            let again = *self.reused.borrow() == Some(pid) && *count > 1;
            Some(at(process.started + u64::from(again)))
        }
        fn signature(&self, pid: u32) -> Signature {
            self.procs
                .get(&pid)
                .map(|p| p.signature.clone())
                .unwrap_or_else(|| Signature::of(SignerKind::Unreadable))
        }
        fn is_terminal_shell(&self, pid: u32) -> bool {
            self.procs.get(&pid).is_some_and(|p| p.shell)
        }
        fn package_folder(&self, script: &Path) -> PathBuf {
            script.parent().map(Path::to_path_buf).unwrap_or_default()
        }
        fn stamp(&self, _path: &Path) -> Option<(u64, i64)> {
            Some((100, 5))
        }
    }

    fn developer_id(identifier: &str, team: &str) -> Signature {
        Signature {
            kind: SignerKind::DeveloperId,
            identifier: Some(identifier.into()),
            team_id: Some(team.into()),
            subject: Some("Anthropic PBC".into()),
            cdhash: Some("abcd".into()),
        }
    }

    fn who(chain: &[u32], table: &Table) -> Resolved {
        resolve(chain, table).expect("resolves")
    }

    #[test]
    fn an_agent_behind_a_shell_behind_wh_is_the_identity_and_its_signer_is_what_counts() {
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh", "list"])
            .with(2, "/bin/bash", &["bash", "-c", "wh list"])
            .with(3, "/Users/me/.local/bin/claude", &["claude"])
            .signed(3, developer_id("com.anthropic.claude-code", "Q6L2SF6YDW"))
            .with(4, "/bin/zsh", &["-zsh"])
            .shell(4);
        let resolved = who(&[1, 2, 3, 4], &table);
        assert_eq!(resolved.label, "Claude Code");
        assert_eq!(resolved.identity.kind, Kind::DeveloperId);
        assert_eq!(resolved.identity.agent, Some(AgentId::Claude));
        assert_eq!(resolved.identity.team_id, "Q6L2SF6YDW");
        assert!(!resolved.unusual_signer, "that is the signer the registry names");
        assert_eq!(resolved.process.map(|p| p.pid), Some(3));
        assert_eq!(
            resolved.program.as_ref().map(|p| p.path.as_path()),
            Some(Path::new("/Users/me/.local/bin/claude"))
        );
        assert!(!resolved.elevated && resolved.started_by.is_none());
    }

    #[test]
    fn a_program_called_claude_signed_by_someone_else_is_flagged_and_is_not_the_developer_id_identity() {
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(3, "/tmp/claude", &["claude"])
            .signed(3, developer_id("com.evil.claude", "EVIL123456"));
        let resolved = who(&[1, 3], &table);
        assert!(resolved.unusual_signer);
        assert_eq!(resolved.identity.team_id, "EVIL123456", "approved, if at all, as what it is");
        // An ad-hoc one of the same name is a different identity again, pinned by its hash.
        let adhoc = Signature { cdhash: Some("ff00".into()), ..Signature::of(SignerKind::AdHoc) };
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(3, "/tmp/claude", &["claude"])
            .signed(3, adhoc);
        let resolved = who(&[1, 3], &table);
        assert_eq!((resolved.identity.kind, resolved.identity.pin.as_str()), (Kind::AdHoc, "ff00"));
        assert!(resolved.unusual_signer);
    }

    #[test]
    fn an_agent_with_no_expected_signer_is_not_flagged_for_its_ad_hoc_signature() {
        let adhoc = Signature { cdhash: Some("ff00".into()), ..Signature::of(SignerKind::AdHoc) };
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(3, "/opt/homebrew/bin/opencode", &["opencode"])
            .signed(3, adhoc);
        let resolved = who(&[1, 3], &table);
        assert_eq!(resolved.identity.agent, Some(AgentId::OpenCode));
        assert!(!resolved.unusual_signer, "the registry names no signer for it, so nothing is unusual");
    }

    #[test]
    fn a_node_script_is_pinned_by_its_package_folder_not_by_node() {
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(
                2,
                "/opt/homebrew/bin/node",
                &["node", "/opt/homebrew/lib/node_modules/@google/gemini-cli/dist/gemini"],
            )
            .signed(2, developer_id("node", "HX7739G8FX"));
        let resolved = who(&[1, 2], &table);
        assert_eq!(resolved.identity.agent, Some(AgentId::Gemini));
        assert_eq!(resolved.identity.kind, Kind::Script);
        assert_eq!(resolved.identity.pin, "/opt/homebrew/lib/node_modules/@google/gemini-cli/dist");
        let program = resolved.program.expect("shown");
        assert_eq!(program.interpreter.as_deref(), Some("node"));
        assert_eq!(program.signature.team_id.as_deref(), Some("HX7739G8FX"), "shown, not part of the key");
        assert!(resolved.identity.team_id.is_empty());
    }

    #[test]
    fn an_unsigned_agent_is_pinned_by_its_file() {
        let table = Table::default().with(1, "/usr/local/bin/wh", &["wh"]).with(
            3,
            "/opt/goose",
            &["goose", "session"],
        );
        let resolved = who(&[1, 3], &table);
        assert_eq!(
            (resolved.identity.kind, resolved.identity.pin.as_str()),
            (Kind::Unsigned, "/opt/goose|100|5")
        );
    }

    #[test]
    fn the_nearest_of_two_agents_is_the_identity_and_the_other_is_said() {
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(2, "/bin/bash", &["bash"])
            .with(3, "/opt/homebrew/bin/codex", &["codex", "exec"])
            .with(4, "/Users/me/.local/bin/claude", &["claude"]);
        let resolved = who(&[1, 2, 3, 4], &table);
        assert_eq!(resolved.identity.agent, Some(AgentId::Codex));
        assert_eq!(resolved.started_by.as_deref(), Some("Claude Code"));
    }

    #[test]
    fn a_sudo_in_between_is_remembered_and_changes_nobody() {
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(2, "/usr/bin/sudo", &["sudo", "wh"])
            .with(3, "/opt/homebrew/bin/codex", &["codex"]);
        let resolved = who(&[1, 2, 3], &table);
        assert!(resolved.elevated);
        assert_eq!(resolved.identity.agent, Some(AgentId::Codex));
    }

    #[test]
    fn a_wh_typed_at_a_terminals_prompt_is_scripts_like_any_other_script() {
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh", "list"])
            .with(2, "/bin/zsh", &["-zsh"])
            .shell(2)
            .with(3, "/opt/homebrew/bin/codex", &["codex"]);
        // The agent above the terminal's shell is not reached: the shell is where the walk stops.
        let resolved = who(&[1, 2, 3], &table);
        assert_eq!(resolved.identity.kind, Kind::Scripts);
        assert_eq!(resolved.label, "Scripts");
        assert!(resolved.process.is_none() && resolved.program.is_none());
    }

    #[test]
    fn a_script_names_the_program_that_asked() {
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(2, "/bin/bash", &["bash", "build.sh"])
            .with(3, "/usr/bin/make", &["make"])
            .with(4, "/bin/zsh", &["-zsh"])
            .shell(4);
        let resolved = who(&[1, 2, 3, 4], &table);
        assert_eq!(resolved.identity.kind, Kind::Scripts);
        assert_eq!(resolved.program.map(|p| p.path), Some(PathBuf::from("/usr/bin/make")));
    }

    #[test]
    fn a_caller_whose_parents_are_gone_is_scripts_and_an_unknown_caller_too() {
        let table = Table::default().with(1, "/usr/local/bin/wh", &["wh"]);
        assert_eq!(who(&[1], &table).identity.kind, Kind::Scripts);
        assert_eq!(who(&[], &table).identity.kind, Kind::Scripts);
        assert_eq!(who(&[99, 98], &table).identity.kind, Kind::Scripts, "processes that cannot be read");
    }

    #[test]
    fn an_agent_whose_pid_changes_hands_while_it_is_read_is_asked_again() {
        let table = Table::default().with(1, "/usr/local/bin/wh", &["wh"]).with(
            3,
            "/opt/homebrew/bin/codex",
            &["codex"],
        );
        *table.reused.borrow_mut() = Some(3);
        assert_eq!(resolve(&[1, 3], &table), Err(Changed));
    }
}
