//! The command blocks of a terminal, with their output, read from any thread.

use wormhole_vt::Line;

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

/// How many rows of a block's output are looked at, at most: the text of the rest would be cut
/// off anyway, and a block can be as long as the history.
const OUTPUT_ROWS: i32 = 2000;

impl ScreenReader {
    /// The newest blocks of the grid, in one hold of the lock. None while a full-screen program
    /// has the screen (the engine keeps the main screen's marks out of reach until it exits).
    pub fn blocks(&self, spec: &BlocksSpec) -> BlocksCapture {
        let integration = self.shared.at_prompt().is_some();
        if !integration {
            return BlocksCapture { integration, blocks: Vec::new() };
        }
        let term = self.term.lock();
        let log = self.shared.commands();
        let all = blocks::blocks(&term, &log);
        let skip = all.len().saturating_sub(spec.last);
        let mut out: Vec<BlockCapture> = all
            .iter()
            .skip(skip)
            .map(|block| BlockCapture {
                id: block.id,
                command: block.command.clone(),
                state: block.state,
                first_line: block.start.0,
                last_line: block.end.0,
                output: None,
                truncated: false,
            })
            .collect();
        if spec.output {
            let mut budget = spec.total_bytes;
            for (block, read) in all.iter().skip(skip).zip(out.iter_mut()).rev() {
                let Some((first, last)) = block.output_lines() else { continue };
                let window = Line(first.0.max(last.0 - OUTPUT_ROWS + 1));
                let mut text = blocks::lines_text(&term, window, last);
                let cap = spec.output_bytes.min(budget);
                read.truncated = window > first;
                if text.len() > cap {
                    let mut start = text.len() - cap;
                    while !text.is_char_boundary(start) {
                        start += 1;
                    }
                    text.drain(..start);
                    read.truncated = true;
                }
                budget -= text.len();
                read.output = Some(text);
            }
        }
        BlocksCapture { integration, blocks: out }
    }
}
