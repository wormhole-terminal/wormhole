//! What a row of the grid carries of a command block: which block and which of its parts. The values
//! `Screen` reads off the cells and `marks`, `blocks` and the snapshot interpret; no code here, so the
//! screen and what reads it do not depend on each other.

/// The three parts of a command block, as the shell's marks divide it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The shell's prompt, a secondary or right prompt included.
    Prompt,
    /// The command line as the shell echoed it while it was typed.
    Command,
    /// What the command printed.
    Output,
}

/// What a row carries of a block: the block's id and which of its parts are on the row. A prompt
/// row carries the prompt and, once something is typed, the command; a long command's wrapped
/// row the command alone; the rest the output. Where two blocks meet on one row (a program that
/// ended without a newline, the prompt right after it), the row is the later block's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowMark {
    pub id: u64,
    pub prompt: bool,
    pub command: bool,
    pub output: bool,
}

impl RowMark {
    /// The row shows the prompt or the command line, so the output starts after it.
    pub fn is_command_line(&self) -> bool {
        self.prompt || self.command
    }
}
