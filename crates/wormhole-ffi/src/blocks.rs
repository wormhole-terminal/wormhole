//! Command blocks (OSC 133) as Swift draws and drives them: the viewport's spans for the gutter, the
//! blocks on the grid, jumps between prompts and the four actions on a block (`wormhole-term`
//! `blocks`, `marks`, `command_to_prompt`; the view's own state stays in Swift). Lines are grid
//! lines (0 the top of the live screen, the history above it negative), rows are viewport rows.

use wormhole_term::types::{Color, Line, Rgb};
use wormhole_term::{Block, BlockState, Direction, Refusal, command_to_prompt};

use crate::terminal::Terminal;

/// Where a command stands, from the shell's own marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum BlockStatus {
    /// The shell is at this prompt; nothing has run.
    Prompt,
    Running,
    /// `code` is `None` when the shell gave none (after an empty line), where nothing ran.
    Finished {
        code: Option<i32>,
        took_ms: Option<u64>,
    },
}

/// A block's rows in the viewport of the last frame, for the gutter beside them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct BlockSpanRecord {
    pub id: u64,
    pub first_row: u32,
    /// One past the block's last row.
    pub end_row: u32,
    /// `first_row` is the block's own first row; otherwise it began above the view.
    pub starts: bool,
    pub status: BlockStatus,
}

/// A block on the grid with its command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BlockRecord {
    pub id: u64,
    pub start: i32,
    pub output_start: Option<i32>,
    pub end: i32,
    /// The command line as the shell echoed it; empty when the history dropped its start.
    pub command: String,
    pub status: BlockStatus,
}

/// What `block_put_at_prompt` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PutResult {
    /// ⌃U and the command were typed; the Return is the person's.
    Typed,
    /// No such block any more, or it has no command.
    Gone,
    /// A program holds the terminal: it would get the text as input.
    NotAtPrompt,
    /// Some of the command was drawn concealed (SGR 8).
    Concealed,
    /// Some of it was drawn in the color of its own background.
    SameColor,
    /// It holds characters that draw nothing, or blanks that are not spaces.
    Invisible,
    /// Several lines in a shell that does not take a paste as one: each would run.
    Lines,
}

fn status(state: BlockState) -> BlockStatus {
    match state {
        BlockState::Prompt => BlockStatus::Prompt,
        BlockState::Running { .. } => BlockStatus::Running,
        BlockState::Finished { code, took } => BlockStatus::Finished {
            code,
            took_ms: took.map(|t| u64::try_from(t.as_millis()).unwrap_or(u64::MAX)),
        },
    }
}

fn record(block: Block) -> BlockRecord {
    BlockRecord {
        id: block.id,
        start: block.start.0,
        output_start: block.output_lines().map(|(first, _)| first.0),
        end: block.end.0,
        command: block.command,
        status: status(block.state),
    }
}

#[uniffi::export]
impl Terminal {
    /// Whether the shell has integration (a prompt mark arrived), so there are blocks.
    pub fn blocks_available(&self) -> bool {
        self.with_pty(|pty| pty.has_blocks()).unwrap_or(false)
    }

    /// The shell is at its prompt (its marks say so), which is when a command can be typed there.
    pub fn shell_at_prompt(&self) -> bool {
        self.with_pty(|pty| pty.at_prompt() == Some(true)).unwrap_or(false)
    }

    /// The blocks in view as of the last frame, top to bottom (none without integration, or while a
    /// full-screen program has the screen).
    pub fn block_spans(&self) -> Vec<BlockSpanRecord> {
        self.with_snapshot(|snapshot| {
            snapshot
                .block_spans()
                .iter()
                .map(|span| BlockSpanRecord {
                    id: span.id,
                    first_row: span.rows.start as u32,
                    end_row: span.rows.end as u32,
                    starts: span.starts,
                    status: status(span.state),
                })
                .collect()
        })
    }

    /// The block with this id, while its text is on the grid.
    pub fn block_by_id(&self, id: u64) -> Option<BlockRecord> {
        self.with_pty(|pty| pty.block(id)).flatten().map(record)
    }

    /// The block whose text `line` is in.
    pub fn block_at_line(&self, line: i32) -> Option<BlockRecord> {
        self.with_pty(|pty| pty.block_at(Line(line))).flatten().map(record)
    }

    /// The newest block whose command ran: what the Edit menu acts on with no jump before it.
    pub fn block_newest_ran(&self) -> Option<BlockRecord> {
        self.with_pty(|pty| pty.blocks().into_iter().rev().find(|b| b.state.ran())).flatten().map(record)
    }

    /// The nearest prompt start above (`up`) or below `from`, strictly.
    pub fn prompt_line(&self, from: i32, up: bool) -> Option<i32> {
        let direction = if up { Direction::Up } else { Direction::Down };
        self.with_pty(|pty| pty.prompt_line(Line(from), direction)).flatten().map(|line| line.0)
    }

    /// How many lines the view is scrolled up into the history.
    pub fn display_offset(&self) -> u32 {
        self.with_pty(|pty| pty.display_offset()).unwrap_or(0) as u32
    }

    /// The grid's last line (the live screen's bottom row).
    pub fn bottommost_line(&self) -> i32 {
        self.with_pty(|pty| pty.bottommost_line().0).unwrap_or(0)
    }

    /// Scroll the view so `line` is its top row, as far as the history allows.
    pub fn scroll_line_to_top(&self, line: i32) {
        self.with_pty(|pty| pty.scroll_line_to_top(Line(line)));
    }

    /// Select whole lines `first...=last` (a block's output) and bring them into view, as GPUI's Select Output does.
    pub fn select_lines(&self, first: i32, last: i32) {
        self.with_pty(|pty| {
            pty.select_lines(Line(first), Line(last));
            pty.reveal(Line(first));
        });
    }

    /// The text of lines `first...=last`.
    pub fn lines_text(&self, first: i32, last: i32) -> String {
        self.with_pty(|pty| pty.lines_text(Line(first), Line(last))).unwrap_or_default()
    }

    /// Type block `id`'s command at the prompt after clearing the line (⌃U), never running it, and only
    /// when every cell of it shows what it holds. `palette` is the theme's colors by slot (0xRRGGBB:
    /// the 256 indexed ones, then foreground, background ...), which tells two names for one color apart
    /// from two colors; what the program set at run time (OSC 4, 10, 11) overrides it.
    pub fn block_put_at_prompt(&self, id: u64, palette: Vec<u32>) -> PutResult {
        let Some(block) = self.with_pty(|pty| pty.block(id)).flatten().filter(|b| !b.command.is_empty())
        else {
            return PutResult::Gone;
        };
        let typed = self.with_pty(|pty| {
            if pty.at_prompt() != Some(true) {
                return Err(PutResult::NotAtPrompt);
            }
            let overrides = self.with_snapshot(|snapshot| snapshot.colors);
            let slot = |slot: usize| {
                overrides.get(slot).map(|c| (c.r, c.g, c.b)).or_else(|| {
                    palette.get(slot).map(|rgb| ((rgb >> 16) as u8, (rgb >> 8) as u8, *rgb as u8))
                })
            };
            let cells = pty.command_cells(&block);
            command_to_prompt(&block.command, &cells, pty.mode(), |color| match color {
                Color::Rgb(Rgb { r, g, b }) => Some((r, g, b)),
                Color::Indexed(index) => slot(usize::from(index)),
                Color::Named(named) => slot(named as u16 as usize),
            })
            .map_err(refusal)
        });
        match typed {
            Some(Ok(bytes)) => {
                self.write(bytes);
                PutResult::Typed
            }
            Some(Err(result)) => result,
            None => PutResult::Gone,
        }
    }
}

fn refusal(why: Refusal) -> PutResult {
    match why {
        Refusal::Concealed => PutResult::Concealed,
        Refusal::SameColor => PutResult::SameColor,
        Refusal::Invisible => PutResult::Invisible,
        Refusal::Lines => PutResult::Lines,
    }
}
