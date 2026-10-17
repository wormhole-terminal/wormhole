//! What a terminal is doing, as the list sees it.

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
    /// Idle: nothing to look at (an ended shell included).
    pub fn is_idle(self) -> bool {
        matches!(self, RowState::Idle | RowState::Exited)
    }
}
