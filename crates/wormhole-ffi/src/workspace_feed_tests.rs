//! The workspace's feeds over real things: a coding agent's process in a real PTY (a script named like
//! the agent, found the way the app finds one), its hooks, and a throwaway git
//! repository read by the workspace itself. Each test runs on the experiment's own data folder and
//! touches only folders it makes under the temp folder.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use wormhole_core::platform::paths;
use wormhole_core::view::sidebar::{RowState, Status};
use wormhole_core::workspace::{Action, SidebarRow};
use wormhole_term::sys::process::command;

use crate::{Terminal, TerminalFactsRecord, TerminalListener, Workspace, WorkspaceConfig, WorkspaceListener};

#[derive(Default)]
struct Wakes(AtomicUsize);

impl WorkspaceListener for Wakes {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl TerminalListener for Wakes {
    fn wake(&self) {}
}

fn workspace() -> (Arc<Workspace>, Arc<Wakes>) {
    crate::isolate_for_tests();
    let wakes = Arc::new(Wakes::default());
    let config = WorkspaceConfig {
        home: None,
        numbers_follow_list: true,
        start_folder: None,
        pane_header_height: 0.0,
    };
    (Workspace::new(config, wakes.clone()), wakes)
}

fn row(ws: &Workspace, id: i64) -> Option<SidebarRow> {
    ws.sidebar().sections.into_iter().flat_map(|section| section.rows).find(|row| row.id == id)
}

/// Wait (bounded) until `done` holds: the feeds run on threads of their own.
fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A scratch folder of the experiment's, new for this test.
fn scratch(name: &str) -> PathBuf {
    let dir = paths::temp().join(format!("Wormhole-swift-ffi-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn put(path: &Path, text: &str) {
    std::fs::File::create(path).and_then(|mut file| file.write_all(text.as_bytes())).expect("a file");
}

/// A shell script named `name` that does `body`. The registry takes a script on an interpreter for the
/// agent its name says (`identify`: `sh claude` is Claude), as it does `node …/codex`. A copy of a system
/// program would not do: macOS kills a platform binary run from another path (exit 137).
fn agent_script(dir: &Path, name: &str, body: &str) -> String {
    let path = dir.join(name);
    put(&path, &format!("#!/bin/sh\n{body}\n"));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    path.to_string_lossy().into_owned()
}

/// An agent that rings its bell for every line it is sent.
const RINGS: &str = "while IFS= read -r line; do printf '\\a\\n'; done";

fn start(ws: &Workspace, wakes: &Arc<Wakes>, program: &str, args: &[&str]) -> i64 {
    ws.apply(Action::NewTerminal);
    let id = ws.selected().expect("the new terminal is selected");
    let args = args.iter().map(ToString::to_string).collect();
    let terminal = Terminal::start(program.into(), args, 80, 24, 8.0, 16.0, wakes.clone())
        .expect("the program starts in an isolated process");
    ws.register_terminal(id, terminal);
    id
}

#[test]
fn an_agent_in_a_real_terminal_is_found_and_its_hooks_are_its_status() {
    let (ws, wakes) = workspace();
    let dir = scratch("agent");
    let claude = agent_script(&dir, "claude", RINGS);
    let agent = start(&ws, &wakes, &claude, &[]);
    start(&ws, &wakes, "/bin/cat", &[]);
    assert_eq!(ws.selected(), Some(2), "the agent's terminal is behind");

    // Found by its process, as the app finds it: nothing told us.
    eventually("the agent in front", || {
        row(&ws, agent).and_then(|r| r.agent).is_some_and(|a| a.short_name == "Claude")
    });
    assert_eq!(row(&ws, agent).map(|r| r.state), Some(RowState::Idle));

    // Its hooks say what it does.
    ws.hook(agent, "claude".into(), "UserPromptSubmit".into(), None);
    assert_eq!(row(&ws, agent).map(|r| r.state), Some(RowState::Working));
    ws.hook(agent, "claude".into(), "PermissionRequest:Bash".into(), Some("Approve cargo test?".into()));
    let asking = row(&ws, agent).expect("row");
    assert_eq!((asking.state, asking.message.as_deref()), (RowState::NeedsYou, Some("Approve cargo test?")));
    assert!(asking.second_line.text.as_deref().is_some_and(|text| text.contains("Approve cargo test?")));
    ws.hook(agent, "claude".into(), "Stop".into(), None);
    assert_eq!(row(&ws, agent).map(|r| r.state), Some(RowState::Finished), "nobody has looked");
    // Looking at it spends the result.
    ws.apply(Action::Select(agent));
    assert_eq!(row(&ws, agent).map(|r| r.state), Some(RowState::Idle));
    // An unknown agent or terminal is ignored.
    assert!(ws.hook(agent, "nobody".into(), "Stop".into(), None).is_empty());
    assert!(ws.hook(99, "claude".into(), "Stop".into(), None).is_empty());
}

#[test]
fn an_agents_bell_asks_for_you_and_its_output_going_quiet_settles_it() {
    let (ws, wakes) = workspace();
    let dir = scratch("quiet");
    let bell = agent_script(&dir, "claude", RINGS);
    // An agent without hooks that prints by itself after a moment, then stays.
    let codex = agent_script(&dir, "codex", "sleep 1.5; echo working; sleep 8");
    let ringing = start(&ws, &wakes, &bell, &[]);
    let printing = start(&ws, &wakes, &codex, &[]);
    start(&ws, &wakes, "/bin/cat", &[]);

    eventually("both agents found", || {
        [ringing, printing].iter().all(|&id| row(&ws, id).is_some_and(|r| r.agent.is_some()))
    });
    // The script writes a BEL for the line it is sent, through the PTY. An agent without hooks rings to ask.
    let terminal = ws.terminal(ringing).expect("registered");
    terminal.write(b"\x07\n".to_vec());
    eventually("the agent asks", || row(&ws, ringing).is_some_and(|r| r.state == RowState::NeedsYou));
    // Heard when looked at.
    ws.apply(Action::Select(ringing));
    assert_eq!(row(&ws, ringing).map(|r| r.state), Some(RowState::Idle));

    // The other prints on its own: working, and once quiet for a while, finished (nobody looks).
    eventually("working", || row(&ws, printing).is_some_and(|r| r.state == RowState::Working));
    let worked = Instant::now();
    eventually("finished", || row(&ws, printing).is_some_and(|r| r.state == RowState::Finished));
    assert!(worked.elapsed() >= Duration::from_millis(1_000), "after the output was quiet for a while");
}

fn git(dir: &Path, args: &[&str]) {
    let status = command("git")
        .current_dir(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid", "-c", "commit.gpgsign=false"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

fn facts_in(dir: &Path, agent: Option<&str>, status: Status) -> TerminalFactsRecord {
    TerminalFactsRecord {
        title: None,
        shell: "zsh".into(),
        agent: agent.map(Into::into),
        reports: false,
        status,
        message: None,
        since_ms: 0,
        cwd: Some(dir.to_string_lossy().into_owned()),
        branch: None,
        command: None,
        running_since_ms: None,
        bell: false,
        failure_code: None,
        failure_command: None,
        failure_at_ms: 0,
        unread: false,
        changes: None,
        notes: 0,
        finished_command: None,
        repo: None,
        dirty: false,
        ahead: 0,
        behind: 0,
        repo_files: None,
    }
}

#[test]
fn a_repository_is_read_by_the_workspace_and_a_working_terminals_files_are_its_count() {
    let (ws, _) = workspace();
    let repo = scratch("repo");
    git(&repo, &["init", "-q", "-b", "main"]);
    put(&repo.join("a.txt"), "one\n");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "first"]);
    let repo_text = std::fs::canonicalize(&repo).expect("real path").to_string_lossy().into_owned();

    // Terminal 1 is an agent at work in the repository; terminal 2 is idle in the same one.
    ws.apply(Action::NewTerminal);
    ws.apply(Action::NewTerminal);
    ws.set_facts(1, facts_in(&repo, Some("claude"), Status::Busy));
    ws.set_facts(2, facts_in(&repo, None, Status::Idle));

    // The first answer: the branch, a clean tree, the repository's name. Nobody owns the baseline.
    eventually("git's first answer", || row(&ws, 1).is_some_and(|r| r.branch.is_some()));
    let first = row(&ws, 1).expect("row");
    assert_eq!(first.branch.as_deref(), Some("main"));
    assert_eq!((first.dirty, first.repo_files, first.changes), (false, Some(0), None), "{first:?}");
    assert_eq!(first.repo.as_deref(), Some(repo_text.as_str()));

    // A file appears while the agent works: the repository changes, and the file is the agent's.
    put(&repo.join("b.txt"), "two\n");
    eventually("the new file", || row(&ws, 1).is_some_and(|r| r.repo_files == Some(1)));
    eventually("the agent's count", || row(&ws, 1).is_some_and(|r| r.changes == Some(1)));
    let (agent, idle) = (row(&ws, 1).expect("row"), row(&ws, 2).expect("row"));
    assert!(agent.dirty && idle.dirty, "the tree's own state, for everyone in it");
    assert_eq!((agent.changes, idle.changes), (Some(1), None), "the idle terminal changed nothing");
    assert_eq!(idle.repo_files, Some(1), "the repository's count is everyone's");

    // A commit leaves the status: the count goes with it.
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "second"]);
    eventually("a clean tree again", || row(&ws, 1).is_some_and(|r| !r.dirty && r.changes.is_none()));

    // A folder outside any repository says so by having none.
    let plain = scratch("plain");
    ws.set_facts(2, facts_in(&plain, None, Status::Idle));
    eventually("no repository", || row(&ws, 2).is_some_and(|r| r.repo.is_none() && r.branch.is_none()));
}
