//! The command blocks and prompts the shell's integration marks on the grid.

use crate::Pty;
use wormhole_vt::Line;

use crate::{Block, Direction, Ended, blocks, marks};

impl Pty {
    /// The lines where prompts start (OSC 133 `A`), oldest first: negative lines are in the
    /// history. A scan of the grid under the lock, for a key press rather than every frame.
    pub fn prompt_lines(&self) -> Vec<Line> {
        marks::prompt_lines(&self.term.lock())
    }

    /// The nearest prompt start above or below `from` (a grid line; the top of the view is
    /// `-display_offset`), for jumping between prompts.
    pub fn prompt_line(&self, from: Line, direction: Direction) -> Option<Line> {
        marks::prompt_line(&self.term.lock(), from, direction)
    }

    /// Whether the shell has integration (a prompt mark has arrived), so there are blocks.
    pub fn has_blocks(&self) -> bool {
        self.shared.at_prompt().is_some()
    }

    /// Every command block on the grid, oldest first: a scan of the whole grid under the lock,
    /// for a key press or a menu, not for every frame.
    pub fn blocks(&self) -> Vec<Block> {
        let term = self.term.lock();
        blocks::blocks(&term, &self.shared.commands())
    }

    /// The block whose text `line` is in.
    pub fn block_at(&self, line: Line) -> Option<Block> {
        let term = self.term.lock();
        blocks::block_at(&term, line, &self.shared.commands())
    }

    /// The block with this id (a `BlockSpan`'s), while its text is on the grid.
    pub fn block(&self, id: u64) -> Option<Block> {
        let term = self.term.lock();
        blocks::block(&term, id, &self.shared.commands())
    }

    /// The cells of `block`'s command line as they are drawn, for [`command_to_prompt`].
    pub fn command_cells(&self, block: &Block) -> Vec<wormhole_vt::Cell> {
        blocks::command_cells(&self.term.lock(), block)
    }

    /// The newest command that ran (it may still run): what a mark that has just arrived is about.
    /// Looks only at the newest blocks, so it costs a few lines of the grid.
    pub fn newest_block(&self) -> Option<Block> {
        let term = self.term.lock();
        blocks::newest_ran(&term, &self.shared.commands())
    }

    /// The command that most recently finished with exit code `code`: what it was, how long it ran
    /// and what it last printed. Looks only at the newest blocks, so it costs a few lines of the
    /// grid, not a scan of the history; for the moment a command is found to have failed.
    pub fn ended(&self, code: i32) -> Option<Ended> {
        let term = self.term.lock();
        blocks::ended(&term, &self.shared.commands(), code)
    }
}
