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
use crate::agents::{self, AgentId, Launch};
use crate::platform::codesign::{Signature, SignerKind};

/// What the walk asks of the system.
pub trait Processes {
    fn executable(&self, pid: u32) -> Option<PathBuf>;
    fn command_line(&self, pid: u32) -> Option<(String, Vec<String>)>;
    fn start_time(&self, pid: u32) -> Option<SystemTime>;
    fn signature(&self, pid: u32) -> Signature;
    /// Whether the process's environment makes an interpreter load code of its own before the script it was
    /// given (`NODE_OPTIONS`, `PYTHONPATH`, …). A process whose environment cannot be read says yes.
    fn env_runs_code(&self, pid: u32) -> bool;
    /// Whether `pid` is the shell of one of Wormhole's terminals.
    fn is_terminal_shell(&self, pid: u32) -> bool;
    /// What `script` (as `pid` was started with it, relative to its folder) is and what pins it; `None` when it is
    /// not a regular file inside the package that would pin it.
    fn script_pin(&self, pid: u32, script: &str) -> Option<ScriptPin>;
    /// A file's size and modification time (Unix seconds), which pin an unsigned program.
    fn stamp(&self, path: &Path) -> Option<(u64, i64)>;
}

/// A script as the system resolves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptPin {
    /// The regular file it is, links followed.
    pub file: PathBuf,
    /// What an approval is pinned to: the folder of the nearest `package.json` the file is inside of, else the
    /// file itself.
    pub pin: PathBuf,
}

/// A program, as the question shows it. Never trusted: it is what the system said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub pid: u32,
    /// Its executable, or the script an interpreter runs (the file it really is).
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

/// How many processes above the agent are looked at for a second agent ("started by"): a few levels, not the
/// whole chain.
const STARTED_BY_LEVELS: usize = 4;

/// Programs that pass their arguments on and are never the identity.
const WRAPPERS: &[&str] =
    &["env", "command", "exec", "nice", "nohup", "time", "timeout", "xargs", "caffeinate", "script", "sudo"];

const SHELLS: &[&str] =
    &["sh", "bash", "zsh", "fish", "dash", "ksh", "tcsh", "csh", "nu", "xonsh", "pwsh", "elvish"];

/// Names `wh` runs as: the command, and the app's executable it is a link to.
const WH_NAMES: &[&str] = &["wh", "wormhole"];

/// What the walk found at the agent's pid.
struct Found {
    agent: AgentId,
    pid: u32,
    exe: PathBuf,
    /// For a script: what it is and what pins it.
    script: Option<ScriptPin>,
}

/// Who is behind `chain` (the caller first, then its parents): see the module's notes.
pub fn resolve(chain: &[u32], procs: &dyn Processes) -> Result<Resolved, Changed> {
    let mut elevated = false;
    let mut requester: Option<Program> = None;
    let mut found: Option<Found> = None;
    let mut second: Option<AgentId> = None;
    let mut above_agent = 0;
    for &pid in chain.iter().take(CHAIN_MAX) {
        let Some(exe) = procs.executable(pid) else { continue };
        let (exe_text, argv) =
            procs.command_line(pid).unwrap_or_else(|| (exe.to_string_lossy().into_owned(), Vec::new()));
        if found.is_some() {
            // Past the agent only to say what started it: a few levels, and never beyond a terminal's shell.
            above_agent += 1;
            if above_agent > STARTED_BY_LEVELS || procs.is_terminal_shell(pid) {
                break;
            }
            if let Some(Launch::Native(agent) | Launch::Script { agent, .. }) =
                agents::identify_launch(&exe_text, &argv, false)
            {
                second = Some(agent);
                break;
            }
            continue;
        }
        match agents::identify_launch(&exe_text, &argv, false) {
            Some(Launch::Native(agent)) => {
                found = Some(Found { agent, pid, exe, script: None });
                continue;
            }
            // A script is an agent only when its interpreter was started plainly (nothing in the
            // environment that loads code first) and the script is a regular file inside its package.
            Some(Launch::Script { agent, script }) if !procs.env_runs_code(pid) => {
                if let Some(pin) = procs.script_pin(pid, &script) {
                    found = Some(Found { agent, pid, exe, script: Some(pin) });
                    continue;
                }
            }
            Some(Launch::Script { .. }) | None => {}
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
            requester = program_of(pid, &exe, &argv, None, procs).ok();
        }
    }
    let Some(Found { agent, pid, exe, script }) = found else {
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
    let argv = procs.command_line(pid).map(|(_, argv)| argv).unwrap_or_default();
    let program = program_of(pid, &exe, &argv, script.as_ref(), procs)?;
    if procs.start_time(pid) != Some(before) {
        return Err(Changed);
    }
    let (identity, unusual_signer) = identity_of(agent, &program, &exe, script.as_ref(), procs);
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

/// What the system says about the program at `pid`; for a script, the file it really is and the interpreter.
fn program_of(
    pid: u32,
    exe: &Path,
    _argv: &[String],
    script: Option<&ScriptPin>,
    procs: &dyn Processes,
) -> Result<Program, Changed> {
    let started = procs.start_time(pid).ok_or(Changed)?;
    let (path, interpreter) = match script {
        Some(script) => (script.file.clone(), Some(base_name(exe))),
        None => (exe.to_path_buf(), None),
    };
    Ok(Program { pid, path, signature: procs.signature(pid), interpreter, started })
}

/// The identity of `agent` running as `program`, and whether its signature is not the usual one.
fn identity_of(
    agent: AgentId,
    program: &Program,
    exe: &Path,
    script: Option<&ScriptPin>,
    procs: &dyn Processes,
) -> (Identity, bool) {
    let registry = agents::agent(agent);
    let base = Identity { kind: Kind::Unsigned, agent: Some(agent), ..Identity::scripts() };
    let signature = &program.signature;
    let identifier = signature.identifier.clone().unwrap_or_default();
    let team = signature.team_id.clone().unwrap_or_default();
    // What pins a program by its code when its signer cannot: the hash, else the file as it is.
    let by_code = || match &signature.cdhash {
        Some(hash) => hash.clone(),
        None => {
            let stamp = procs.stamp(exe).map(|(size, time)| format!("|{size}|{time}"));
            format!("{}{}", exe.display(), stamp.unwrap_or_default())
        }
    };
    if let Some(script) = script {
        // The key says which package and which interpreter: another `node` is another program.
        let folder = script.pin.to_string_lossy().into_owned();
        let signed = matches!(signature.kind, SignerKind::DeveloperId | SignerKind::AppleSystem);
        let identity = if signed {
            Identity { kind: Kind::Script, identifier, team_id: team, pin: folder, ..base }
        } else {
            Identity { kind: Kind::Script, pin: format!("{folder}\n{}", by_code()), ..base }
        };
        return (identity, false);
    }
    match signature.kind {
        SignerKind::DeveloperId => {
            let usual = registry.signers.is_empty()
                || registry.signers.iter().any(|(id, team_id)| *id == identifier && *team_id == team);
            (Identity { kind: Kind::DeveloperId, identifier, team_id: team, ..base }, !usual)
        }
        SignerKind::AppleSystem => (Identity { kind: Kind::Apple, identifier, ..base }, false),
        SignerKind::AdHoc | SignerKind::Other => {
            let pin = signature.cdhash.clone().unwrap_or_default();
            (
                Identity { kind: Kind::AdHoc, pin, ..base },
                !registry.signers.is_empty() || signature.injectable,
            )
        }
        SignerKind::Unsigned | SignerKind::Unreadable => {
            (Identity { kind: Kind::Unsigned, pin: by_code(), ..base }, !registry.signers.is_empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    use super::{Changed, Processes, Resolved, ScriptPin, resolve};
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
        /// Processes whose environment loads code before a script (`NODE_OPTIONS` and the like).
        loads_code: std::cell::RefCell<Vec<u32>>,
        /// Scripts that are not a regular file inside their package.
        outside_package: std::cell::RefCell<Vec<String>>,
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
        fn env_runs_code(&self, pid: u32) -> bool {
            self.loads_code.borrow().contains(&pid)
        }
        fn script_pin(&self, _pid: u32, script: &str) -> Option<ScriptPin> {
            if self.outside_package.borrow().iter().any(|s| s == script) {
                return None;
            }
            let file = PathBuf::from(script);
            Some(ScriptPin { pin: file.parent().map(Path::to_path_buf).unwrap_or_default(), file })
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
            injectable: false,
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
        assert_eq!(program.signature.team_id.as_deref(), Some("HX7739G8FX"));
        // The interpreter's signer is part of the key: another `node` is another program.
        assert_eq!(
            (resolved.identity.identifier.as_str(), resolved.identity.team_id.as_str()),
            ("node", "HX7739G8FX")
        );
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

    fn gemini_on(table: Table, node: &'static str, args: &[&'static str]) -> Table {
        table.with(1, "/usr/local/bin/wh", &["wh"]).with(2, node, args)
    }

    #[test]
    fn a_program_that_calls_itself_node_is_not_a_script_agent() {
        // argv[0] says node and the script is Gemini's, but the executable is no interpreter.
        let table = gemini_on(Table::default(), "/tmp/evil", &["node", "/opt/gemini-cli/bin/gemini"]);
        assert_eq!(who(&[1, 2], &table).identity.kind, Kind::Scripts);
    }

    #[test]
    fn flags_that_load_or_run_code_make_a_script_no_agent() {
        for args in [
            &["node", "--require=/tmp/evil.js", "/opt/gemini-cli/bin/gemini"][..],
            &["node", "-r", "/tmp/evil.js", "/opt/gemini-cli/bin/gemini"],
            &["node", "-e", "-x", "/opt/gemini-cli/bin/gemini"],
            &["node", "--import", "/tmp/evil.mjs", "/opt/gemini-cli/bin/gemini"],
        ] {
            let table = gemini_on(Table::default(), "/opt/homebrew/bin/node", args);
            let resolved = who(&[1, 2], &table);
            assert_eq!(resolved.identity.kind, Kind::Scripts, "{args:?}");
            assert_eq!(resolved.label, "Scripts");
        }
    }

    #[test]
    fn an_environment_that_loads_code_makes_a_script_no_agent() {
        let table =
            gemini_on(Table::default(), "/opt/homebrew/bin/node", &["node", "/opt/gemini-cli/bin/gemini"]);
        assert_eq!(who(&[1, 2], &table).identity.kind, Kind::Script);
        table.loads_code.borrow_mut().push(2);
        assert_eq!(who(&[1, 2], &table).identity.kind, Kind::Scripts, "NODE_OPTIONS and the like");
    }

    #[test]
    fn a_script_that_is_not_a_regular_file_inside_its_package_is_no_agent() {
        let table = gemini_on(Table::default(), "/opt/homebrew/bin/node", &["node", "/tmp/gemini"]);
        table.outside_package.borrow_mut().push("/tmp/gemini".into());
        assert_eq!(who(&[1, 2], &table).identity.kind, Kind::Scripts);
    }

    #[test]
    fn another_interpreter_is_another_key() {
        let key = |sig: Signature, exe: &'static str| {
            let table =
                gemini_on(Table::default(), exe, &["node", "/opt/gemini-cli/bin/gemini"]).signed(2, sig);
            who(&[1, 2], &table).identity
        };
        let homebrew = key(developer_id("node", "HX7739G8FX"), "/opt/homebrew/bin/node");
        let other_team = key(developer_id("node", "EVIL123456"), "/opt/homebrew/bin/node");
        assert_ne!(homebrew, other_team);
        // Without a signer: the hash, then the file.
        let hashed = |hash: &str| {
            key(Signature { cdhash: Some(hash.into()), ..Signature::of(SignerKind::AdHoc) }, "/tmp/node")
        };
        assert_ne!(hashed("aa"), hashed("bb"));
        assert_ne!(hashed("aa"), homebrew);
        let unsigned = key(Signature::of(SignerKind::Unsigned), "/tmp/node");
        assert!(unsigned.pin.contains("/tmp/node|100|5"), "{}", unsigned.pin);
        assert_eq!(homebrew.kind, Kind::Script, "and Always stays possible for a script");
        assert!(homebrew.can_be_remembered());
    }

    #[test]
    fn a_developer_id_program_without_the_hardened_runtime_is_pinned_by_its_hash_and_flagged() {
        let injectable = Signature {
            kind: SignerKind::AdHoc,
            identifier: Some("com.anthropic.claude-code".into()),
            team_id: Some("Q6L2SF6YDW".into()),
            cdhash: Some("ab12".into()),
            injectable: true,
            ..Signature::of(SignerKind::AdHoc)
        };
        let table = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(3, "/x/claude", &["claude"])
            .signed(3, injectable);
        let resolved = who(&[1, 3], &table);
        assert_eq!((resolved.identity.kind, resolved.identity.pin.as_str()), (Kind::AdHoc, "ab12"));
        assert!(resolved.unusual_signer);
    }

    #[test]
    fn the_walk_stops_above_the_agent_at_a_terminals_shell_and_after_a_few_levels() {
        let mut table =
            Table::default().with(1, "/usr/local/bin/wh", &["wh"]).with(2, "/x/claude", &["claude"]);
        for pid in 3..12 {
            table = table.with(pid, "/bin/true", &["true"]);
        }
        // A second agent far above (beyond the few levels) is not looked for.
        table = table.with(12, "/opt/homebrew/bin/codex", &["codex"]);
        let chain: Vec<u32> = (1..=12).collect();
        assert_eq!(who(&chain, &table).started_by, None);
        // One within a few levels is said; a terminal's shell in between ends the search.
        let near = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(2, "/x/claude", &["claude"])
            .with(3, "/opt/homebrew/bin/codex", &["codex"]);
        assert_eq!(who(&[1, 2, 3], &near).started_by.as_deref(), Some("Codex CLI"));
        let shell = near.with(4, "/bin/zsh", &["-zsh"]).shell(4);
        let shell = shell.with(5, "/opt/homebrew/bin/codex", &["codex"]);
        let blocked = Table::default()
            .with(1, "/usr/local/bin/wh", &["wh"])
            .with(2, "/x/claude", &["claude"])
            .with(4, "/bin/zsh", &["-zsh"])
            .shell(4)
            .with(5, "/opt/homebrew/bin/codex", &["codex"]);
        let _ = shell;
        assert_eq!(who(&[1, 2, 4, 5], &blocked).started_by, None);
    }
}
