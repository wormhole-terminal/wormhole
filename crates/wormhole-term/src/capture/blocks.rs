//! The command blocks of a terminal, with their output, read from any thread.

use wormhole_vt::Line;

use super::{CHUNK, Lines, ReadScratch, copy_rows};
use crate::ScreenReader;
use crate::blocks::{self, BlockState};

/// Which blocks to read and how much of their output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlocksSpec {
    /// The newest this many blocks.
    pub last: usize,
    /// Add each block's output.
    pub output: bool,
    /// Most output bytes for one block (its newest lines are kept).
    pub output_bytes: usize,
    /// Most output bytes for all of them: the newest blocks get it first.
    pub total_bytes: usize,
}

/// One block as read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockCapture {
    pub id: u64,
    pub command: String,
    pub state: BlockState,
    /// The grid lines it spans: the prompt's first, the last that holds something of it.
    pub first_line: i32,
    pub last_line: i32,
    /// Its output, when asked for and it has some.
    pub output: Option<String>,
    /// The output was cut: only its newest lines are here.
    pub truncated: bool,
}

/// What [`ScreenReader::blocks`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlocksCapture {
    /// The shell has integration; without it there are no blocks to read.
    pub integration: bool,
    /// Oldest first.
    pub blocks: Vec<BlockCapture>,
}

/// How many rows of a block's output are read, at most: the newest. One chunk, copied in one hold of
/// the lock; the rest of a long output is `truncated`.
const OUTPUT_ROWS: i32 = CHUNK as i32;

impl ScreenReader {
    /// The newest blocks of the grid. The list (ids, ranges and states, no text) is one hold of the
    /// lock; each block's output is another, newest block first, a chunk of rows copied and turned
    /// into text outside it, and none is read once the total is spent. None while a full-screen
    /// program has the screen (the engine keeps the main screen's marks out of reach until it exits).
    pub fn blocks(&self, spec: &BlocksSpec, scratch: &mut ReadScratch) -> BlocksCapture {
        let integration = self.shared.at_prompt().is_some();
        if !integration {
            return BlocksCapture { integration, blocks: Vec::new() };
        }
        let mut out: Vec<BlockCapture> = {
            let mut term = self.term.lock();
            term.reap();
            let log = self.shared.commands();
            let all = blocks::blocks(&term, &log);
            let skip = all.len().saturating_sub(spec.last);
            all[skip..]
                .iter()
                .map(|block| BlockCapture {
                    id: block.id,
                    command: block.command.clone(),
                    state: block.state,
                    first_line: block.start.0,
                    last_line: block.end.0,
                    output: None,
                    truncated: false,
                })
                .collect()
        };
        if !spec.output {
            return BlocksCapture { integration, blocks: out };
        }
        let mut budget = spec.total_bytes;
        for read in out.iter_mut().rev() {
            if budget == 0 {
                break;
            }
            let term = self.term.lock();
            // Found again by its id: output arriving between two holds moves its rows.
            let found = blocks::block(&term, read.id, &self.shared.commands());
            let Some((first, last)) = found.as_ref().and_then(|block| block.output_lines()) else {
                continue;
            };
            let window = Line(first.0.max(last.0 - OUTPUT_ROWS + 1));
            let n = copy_rows(&term, window.0, last.0, scratch);
            let cols = term.columns();
            drop(term);
            let mut text = Lines::joined(cols);
            text.add(scratch.rows(n));
            let capped = text.into_capped(spec.output_bytes.min(budget));
            let mut output = capped.lines.join("\n");
            output.truncate(output.trim_end().len());
            budget = budget.saturating_sub(output.len());
            read.truncated = window > first || capped.cut;
            read.output = Some(output);
        }
        BlocksCapture { integration, blocks: out }
    }
}
