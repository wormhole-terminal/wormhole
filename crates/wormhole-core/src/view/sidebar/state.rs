//! What a terminal is doing, as the list sees it, and how its status makes it.

/// A terminal's status (idle, busy, done, failed, needs input, exited), from its output, its
/// agent's hooks and what its programs report.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Idle,
    Busy,
    /// Finished producing output while you weren't looking.
    Done,
    /// A command ended with a failing exit code (the shell's marks), or an agent or program
    /// stopped on an error (its hooks, or its own report), while you weren't looking. Seen like
    /// `Done`: opening the terminal clears it.
    Failed,
    /// Rang the bell while you weren't looking, or its agent asks something.
    Attention,
    Exited,
}

/// What a terminal is doing. The app maps its own status into this (an agent's, or for a terminal
/// without one its bell and foreground command).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowState {
    /// An agent asks something, or a terminal without an agent rang its bell.
    NeedsYou,
    /// An agent works, or a command runs.
    Working,
    /// An agent finished and nobody has looked yet.
    Finished,
    /// A command ended with a non-zero exit code, or an agent or program stopped on an error, and
    /// nobody has looked yet.
    Failed,
    /// Nothing happening: an agent waiting for a prompt, a shell at its prompt.
    Idle,
    /// The shell ended.
    Exited,
}

impl RowState {
    /// What a terminal's status makes of its row. A terminal whose status is told (`told`: an
    /// agent's, or a program's own report) shows it; one without is working while a command runs,
    /// needs input while its bell counts, and has failed when its last command did and nobody has seen.
    pub fn of(told: bool, status: Status, command: bool) -> Self {
        match (told, status) {
            (_, Status::Exited) => Self::Exited,
            (_, Status::Attention) => Self::NeedsYou,
            (true, Status::Busy) => Self::Working,
            (true, Status::Done) => Self::Finished,
            (true, Status::Failed) => Self::Failed,
            (true, Status::Idle) => Self::Idle,
            (false, _) if command => Self::Working,
            (false, Status::Failed) => Self::Failed,
            (false, _) => Self::Idle,
        }
    }

    /// Idle: nothing to look at (an ended shell included).
    pub fn is_idle(self) -> bool {
        matches!(self, RowState::Idle | RowState::Exited)
    }
}

#[cfg(test)]
mod tests {
    use super::{RowState, Status};

    #[test]
    fn a_failure_is_a_row_state_for_agents_programs_and_plain_terminals() {
        assert_eq!(RowState::of(true, Status::Failed, false), RowState::Failed);
        assert_eq!(RowState::of(false, Status::Failed, false), RowState::Failed);
        // A plain terminal that runs a command again is working, whatever its last one did.
        assert_eq!(RowState::of(false, Status::Failed, true), RowState::Working);
        // Seen, it is a result nobody has left to look at.
        assert_eq!(RowState::of(false, Status::Idle, false), RowState::Idle);
        assert_eq!(RowState::of(true, Status::Attention, false), RowState::NeedsYou, "asking outranks");
    }

    #[test]
    fn an_agents_status_is_its_rows_state() {
        assert_eq!(RowState::of(true, Status::Busy, false), RowState::Working);
        assert_eq!(RowState::of(true, Status::Attention, false), RowState::NeedsYou);
        assert_eq!(RowState::of(true, Status::Done, false), RowState::Finished);
        assert_eq!(RowState::of(true, Status::Idle, true), RowState::Idle, "the agent itself is the command");
        assert_eq!(RowState::of(true, Status::Exited, false), RowState::Exited);
    }

    #[test]
    fn a_plain_terminal_works_while_a_command_runs_and_needs_you_for_its_bell() {
        assert_eq!(RowState::of(false, Status::Idle, true), RowState::Working);
        assert_eq!(
            RowState::of(false, Status::Busy, false),
            RowState::Idle,
            "output alone is unread, not work"
        );
        assert_eq!(RowState::of(false, Status::Done, false), RowState::Idle);
        assert_eq!(RowState::of(false, Status::Attention, true), RowState::NeedsYou);
        assert_eq!(RowState::of(false, Status::Exited, true), RowState::Exited);
    }
}
