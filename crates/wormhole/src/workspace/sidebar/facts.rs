//! What each terminal's row shows, gathered once per frame from its terminal, git status and
//! notes: the facts core's `Listing` and the rows are built from.
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use wormhole_core::notify::Source;
use wormhole_core::sidebar::command::{OutputFacts, ends_by_itself, has_new_output};

use super::*;

/// What a terminal's row shows, gathered once per frame.
pub(super) struct RowInfo {
    pub(super) ix: usize,
    pub(super) id: i64,
    pub(super) name: SharedString,
    /// The agent in the terminal, if any.
    pub(super) agent: Option<AgentId>,
    /// What tells the row's state: an agent, a program that reports its own status (it takes an
    /// agent's kind of row, without the logo), or neither (a plain terminal).
    pub(super) source: Source,
    pub(super) state: RowState,
    pub(super) message: Option<String>,
    /// How long the terminal has been in its status.
    pub(super) since: Duration,
    pub(super) cwd: Option<PathBuf>,
    /// The repository its folder is a worktree of.
    pub(super) worktree: Option<PathBuf>,
    /// The command in front of a plain terminal, and how long it has run: only for one that ends
    /// by itself (a build, a test run), since the time an editor or a dev server has been open says
    /// nothing.
    pub(super) command: Option<String>,
    pub(super) running: Option<Duration>,
    pub(super) bell: bool,
    /// The last command failed (a failing exit code) and the next one hasn't started: the row's
    /// cue, whether or not anyone has seen it yet.
    pub(super) failure: Option<RowFailure>,
    /// The command is done and left output you haven't seen ("new output").
    pub(super) unread: bool,
    pub(super) pinned: bool,
    /// Notes with something in them and tasks not done.
    pub(super) open_items: usize,
    /// The changed files the row counts.
    pub(super) changes: Option<usize>,
}

/// A plain terminal's last failed command, as its row says it ("cargo build · exit 101").
pub(super) struct RowFailure {
    pub(super) code: i32,
    pub(super) command: Option<String>,
    /// How long ago the shell said so.
    pub(super) since: Duration,
}

impl RowInfo {
    /// A row whose status something tells (an agent or a program that reports) says it in words;
    /// a plain terminal says what runs in it.
    pub(super) fn told(&self) -> bool {
        self.source.told()
    }
}

/// What a terminal's status makes of its row. A terminal whose status is told (`told`: an
/// agent's, or a program's own report) shows it; one without is working while a command runs,
/// needs you while its bell counts, and has failed when its last command did and nobody has seen.
fn row_state(told: bool, status: Status, command: bool) -> RowState {
    match (told, status) {
        (_, Status::Exited) => RowState::Exited,
        (_, Status::Attention) => RowState::NeedsYou,
        (true, Status::Busy) => RowState::Working,
        (true, Status::Done) => RowState::Finished,
        (true, Status::Failed) => RowState::Failed,
        (true, Status::Idle) => RowState::Idle,
        (false, _) if command => RowState::Working,
        (false, Status::Failed) => RowState::Failed,
        (false, _) => RowState::Idle,
    }
}

impl Workspace {
    /// Every terminal's row facts, in the window's (manual) order.
    pub(super) fn row_infos(&self, cx: &App) -> Vec<RowInfo> {
        let now = SystemTime::now();
        self.sessions
            .iter()
            .enumerate()
            .map(|(ix, session)| {
                let terminal = session.terminal().read(cx);
                let status = terminal.status();
                let foreground = terminal.foreground();
                let agent = terminal.agent;
                let source = terminal.source();
                let told = source.told();
                let command = foreground.map(|f| f.command.clone()).filter(|_| !told);
                let program_running = command.is_some();
                let git = session.git.status.as_ref().filter(|_| session.git.cwd == terminal.cwd);
                let root = terminal.cwd.as_deref().map(|cwd| session.repo_root.of(cwd));
                RowInfo {
                    ix,
                    id: session.id,
                    name: session.name(cx),
                    agent,
                    source,
                    state: row_state(told, status, command.is_some()),
                    message: terminal.agent_message.clone(),
                    since: terminal.status_since().elapsed(),
                    cwd: terminal.cwd.clone(),
                    worktree: root.as_deref().and_then(|root| self.worktree_of(root)),
                    running: foreground
                        .filter(|f| !told && ends_by_itself(&f.command))
                        .and_then(|f| now.duration_since(f.started).ok()),
                    command,
                    bell: !told && terminal.bell_rang(),
                    failure: terminal.failure().filter(|_| !told).map(|failure| RowFailure {
                        code: failure.code,
                        command: failure.command.clone(),
                        since: failure.at.elapsed(),
                    }),
                    unread: has_new_output(OutputFacts {
                        agent: told,
                        // Seen on screen: the selected terminal and the panes shown with it.
                        selected: self.on_screen(ix),
                        program_running,
                        unseen: matches!(status, Status::Busy | Status::Done),
                    }),
                    pinned: self.split_pinned(ix),
                    open_items: session.open_items(cx),
                    changes: git.and_then(|status| {
                        let n = status.files.len();
                        (n > 0).then_some(n)
                    }),
                }
            })
            .collect()
    }
}

/// What each row's times read now (compact rows, without a second line, show none).
pub(super) fn row_times(infos: &[RowInfo], cx: &App) -> Vec<Option<String>> {
    let compact = !settings(cx).density.two_lines();
    infos.iter().map(|info| rows::time_text(info, compact)).collect()
}

#[cfg(test)]
mod tests {
    use wormhole_core::sidebar::RowState;

    use super::row_state;
    use crate::terminal_view::Status;

    #[test]
    fn a_failure_is_a_row_state_for_agents_programs_and_plain_terminals() {
        assert_eq!(row_state(true, Status::Failed, false), RowState::Failed);
        assert_eq!(row_state(false, Status::Failed, false), RowState::Failed);
        // A plain terminal that runs a command again is working, whatever its last one did.
        assert_eq!(row_state(false, Status::Failed, true), RowState::Working);
        // Seen, it is a result nobody has left to look at.
        assert_eq!(row_state(false, Status::Idle, false), RowState::Idle);
        assert_eq!(row_state(true, Status::Attention, false), RowState::NeedsYou, "asking outranks");
    }

    #[test]
    fn an_agents_status_is_its_rows_state() {
        assert_eq!(row_state(true, Status::Busy, false), RowState::Working);
        assert_eq!(row_state(true, Status::Attention, false), RowState::NeedsYou);
        assert_eq!(row_state(true, Status::Done, false), RowState::Finished);
        assert_eq!(row_state(true, Status::Idle, true), RowState::Idle, "the agent itself is the command");
        assert_eq!(row_state(true, Status::Exited, false), RowState::Exited);
    }

    #[test]
    fn a_plain_terminal_works_while_a_command_runs_and_needs_you_for_its_bell() {
        assert_eq!(row_state(false, Status::Idle, true), RowState::Working);
        assert_eq!(row_state(false, Status::Busy, false), RowState::Idle, "output alone is unread, not work");
        assert_eq!(row_state(false, Status::Done, false), RowState::Idle);
        assert_eq!(row_state(false, Status::Attention, true), RowState::NeedsYou);
        assert_eq!(row_state(false, Status::Exited, true), RowState::Exited);
    }
}
