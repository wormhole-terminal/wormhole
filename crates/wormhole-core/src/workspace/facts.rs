//! What a terminal's row needs to know of the terminal: facts the shell gathers (from the terminal's
//! events, its foreground program, the git status) and hands over whole or in part. The model
//! derives the rest (the row's state, name and words) from these with core's rules.

use std::path::PathBuf;

use crate::agents::AgentId;
use crate::view::sidebar::{RowState, Status};

/// A moment on the shell's monotonic clock, in milliseconds. The model never reads a clock: a fact
/// that began at some moment says which, and a snapshot is asked for at another.
pub type Millis = u64;

/// A plain terminal's last command that failed, as its row says it ("cargo build · exit 101").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub code: i32,
    pub command: Option<String>,
    /// When the shell said so.
    pub at: Millis,
}

/// Everything a sidebar row says of a terminal, as plain values.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalFacts {
    /// The title its program set, as `names::title_name` cleaned it.
    pub title: Option<String>,
    /// The program the terminal started ("zsh").
    pub shell: String,
    /// The coding agent in the foreground, if any.
    pub agent: Option<AgentId>,
    /// A program other than an agent reports its own status (OSC 7501): the row takes an agent's
    /// kind of line, without the logo.
    pub reports: bool,
    pub status: Status,
    /// What the agent or program says it is doing or asking.
    pub message: Option<String>,
    /// When the terminal got into `status`.
    pub since: Millis,
    pub cwd: Option<PathBuf>,
    pub branch: Option<String>,
    /// The command in front of a plain terminal, as a row spells it ("cargo test -p core").
    pub command: Option<String>,
    /// When that command started.
    pub running_since: Option<Millis>,
    pub bell: bool,
    pub failure: Option<Failure>,
    /// The command is done and left output nobody has seen.
    pub unread: bool,
    /// Files this terminal changed in its repository (`git::attribution`), when known.
    pub changes: Option<usize>,
    /// Notes and tasks still open.
    pub notes: usize,
    /// The command that just finished, while its result is unseen (done, failed): it still names the
    /// terminal. Not a running command: `command` is that.
    pub finished_command: Option<String>,
    /// The repository the terminal is in, as git names its root: terminals with the same one share a
    /// repository (`workspace::git_feed`).
    pub repo: Option<PathBuf>,
    /// The working tree has uncommitted changes (the tree's own, whatever `changes` counts).
    pub dirty: bool,
    /// Commits the branch has that its upstream hasn't, and the other way round.
    pub ahead: u32,
    pub behind: u32,
    /// How many files are changed in the repository, whoever changed them.
    pub repo_files: Option<usize>,
}

impl Default for TerminalFacts {
    fn default() -> Self {
        Self {
            title: None,
            shell: String::new(),
            agent: None,
            reports: false,
            status: Status::Idle,
            message: None,
            since: 0,
            cwd: None,
            branch: None,
            command: None,
            running_since: None,
            bell: false,
            failure: None,
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
}

impl TerminalFacts {
    /// Something tells the status: an agent, or a program's own report.
    pub fn told(&self) -> bool {
        self.agent.is_some() || self.reports
    }

    /// What the row shows (`RowState::of`: the one place status becomes a row's state).
    pub fn row_state(&self) -> RowState {
        RowState::of(self.told(), self.status, self.command.is_some())
    }
}
