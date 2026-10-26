//! `read` and `blocks`: what a terminal shows and what its shell ran, as the wire has them.
//!
//! The requests say which lines or which blocks; the results carry the lines with the screen's
//! size, the cursor and the modes the program has set, so a caller that drives a program (keys,
//! mouse) knows what state it is in. Reading text is `control_read`'s ([`super::read_check`]); the
//! copying is `wormhole_term::ScreenReader`'s, done on the socket thread (`ipc::read_job`).

use serde::{Deserialize, Serialize};
use wormhole_term::types::{CursorShape, Modes, MouseEncoding, MouseTracking};
use wormhole_term::{BlockCapture, BlocksCapture, Capture, CaptureRange, CaptureSpec, Hue, StyleRun};

use super::{Error, ErrorCode, Target};

/// The most lines one `read` may ask for.
pub const READ_LINES_MAX: u32 = 5000;
/// The most text one `read` returns; over it the oldest lines go and `truncated` says so.
pub const READ_BYTES_MAX: usize = 256 * 1024;
/// The most rows `styles` may cover.
pub const STYLE_ROWS_MAX: u32 = 500;
/// The most blocks one `blocks` may ask for.
pub const BLOCKS_MAX: u32 = 100;
/// Blocks asked for when the request says no number.
pub const BLOCKS_DEFAULT: u32 = 10;
/// The most output returned for one block.
pub const BLOCK_OUTPUT_MAX_BYTES: usize = 64 * 1024;
/// The most output returned for all the blocks of one request.
pub const BLOCKS_OUTPUT_MAX_BYTES: usize = 512 * 1024;
/// The most a reply may serialize to: a read over it drops its oldest lines.
pub const REPLY_MAX_BYTES: usize = 1 << 20;

/// `read`'s request: which lines of which terminal (the caller's own without a target).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    #[serde(default)]
    pub range: Range,
    /// Put soft-wrapped rows back together into the lines the program printed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub join: bool,
    /// Add each line's colours and attributes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub styles: bool,
}

/// Which lines. Numbers are the engine's: 0 is the live screen's top row, history is negative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Range {
    /// The live screen, whatever the person has scrolled to: the alternate screen while a program
    /// has it.
    #[default]
    Screen,
    /// The last `lines` rows of history and screen.
    Last { lines: u32 },
    /// Rows `from` to `to`, inclusive, cut to what exists.
    Lines { from: i32, to: i32 },
    /// A command block's prompt, command and output.
    Block {
        #[serde(default)]
        id: BlockRef,
    },
}

/// Which block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlockRef {
    Id(u64),
    Newest(Newest),
}

impl Default for BlockRef {
    fn default() -> Self {
        BlockRef::Newest(Newest::Last)
    }
}

/// The word that names the newest block that ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Newest {
    Last,
}

impl ReadSpec {
    /// What to ask the screen for, or what is wrong with the request.
    pub fn capture_spec(&self) -> Result<CaptureSpec, Error> {
        let bad = |message: String| Err(Error::new(ErrorCode::BadRequest, message));
        let too_large = |message: String| Err(Error::new(ErrorCode::TooLarge, message));
        let (range, rows) = match self.range {
            Range::Screen => (CaptureRange::Screen, None),
            Range::Last { lines: 0 } => return bad("read at least one line".into()),
            Range::Last { lines } => (CaptureRange::Last(lines as usize), Some(lines)),
            Range::Lines { from, to } => {
                if from > to {
                    return bad(format!("the range {from}:{to} ends before it starts"));
                }
                let count = i64::from(to) - i64::from(from) + 1;
                let count = u32::try_from(count).unwrap_or(u32::MAX);
                (CaptureRange::Lines { from, to }, Some(count))
            }
            Range::Block { id } => {
                let id = match id {
                    BlockRef::Id(id) => Some(id),
                    BlockRef::Newest(Newest::Last) => None,
                };
                (CaptureRange::Block(id), None)
            }
        };
        if rows.is_some_and(|rows| rows > READ_LINES_MAX) {
            return too_large(format!("read takes at most {READ_LINES_MAX} lines"));
        }
        if self.styles && self.join {
            return bad("styles and join can't be combined: joined lines have no columns".into());
        }
        if self.styles && rows.is_some_and(|rows| rows > STYLE_ROWS_MAX) {
            return too_large(format!("styles cover at most {STYLE_ROWS_MAX} rows"));
        }
        Ok(CaptureSpec { range, join: self.join, styles: self.styles, max_bytes: READ_BYTES_MAX })
    }
}

/// `blocks`' request: the newest blocks of a terminal (the caller's own without a target).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlocksSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    /// How many, newest last; [`BLOCKS_DEFAULT`] without a number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<u32>,
    /// Add each block's output.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub output: bool,
}

impl BlocksSpec {
    /// What to ask the screen for, or what is wrong with the request.
    pub fn blocks_spec(&self) -> Result<wormhole_term::BlocksSpec, Error> {
        let last = self.last.unwrap_or(BLOCKS_DEFAULT);
        if last == 0 {
            return Err(Error::new(ErrorCode::BadRequest, "ask for at least one block"));
        }
        if last > BLOCKS_MAX {
            return Err(Error::new(ErrorCode::TooLarge, format!("blocks takes at most {BLOCKS_MAX}")));
        }
        Ok(wormhole_term::BlocksSpec {
            last: last as usize,
            output: self.output,
            output_bytes: BLOCK_OUTPUT_MAX_BYTES,
            total_bytes: BLOCKS_OUTPUT_MAX_BYTES,
        })
    }
}

/// `read`'s result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Read {
    pub terminal: i64,
    /// The title the program set, when it set one (terminal text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// `main`, or `alt` while a full-screen program has the alternate screen.
    pub screen: String,
    pub size: Size,
    /// Lines of history above the live screen.
    pub history: usize,
    pub cursor: CursorInfo,
    pub modes: ModesInfo,
    /// The grid line the first of `lines` starts on.
    pub first_line: i32,
    pub lines: Vec<String>,
    /// One list of runs per line, when asked for; cells in the default style have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<Vec<RunInfo>>>,
    /// Lines were left out: the history dropped them while the read went on, or the text was over
    /// the cap (the oldest go first).
    pub truncated: bool,
    /// Compare two reads' revisions to tell whether the terminal printed in between.
    pub revision: u64,
    /// The block a `--block` read covered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub rows: usize,
    pub cols: usize,
}

/// Where the cursor is: a row of the live screen and a column, 0-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorInfo {
    pub row: i32,
    pub col: usize,
    pub visible: bool,
    /// What the program asked for; absent while the cursor is hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
}

/// The modes that decide how keys and the mouse reach the program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModesInfo {
    /// DECCKM: arrow keys send `ESC O x`.
    pub app_cursor: bool,
    /// DECKPAM.
    pub app_keypad: bool,
    pub bracketed_paste: bool,
    /// The program asked for the DEC locator (a pointer it is told of in its own reports).
    pub locator: bool,
    /// What mouse events the program asked for: `off`, `x10`, `click`, `drag` or `motion`.
    pub mouse: String,
    /// How it wants them written: `x10`, `utf8`, `urxvt`, `sgr` or `sgr_pixels`.
    pub mouse_encoding: String,
    /// The kitty keyboard protocol's flags in force (0: the legacy encoding).
    pub kitty_keyboard: u8,
}

/// Cells `col..col + len` of a line, drawn the same way. Colours are `#rrggbb` or a palette index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunInfo {
    pub col: u16,
    pub len: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fg: Option<ColorInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg: Option<ColorInfo>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub inverse: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorInfo {
    Palette(u8),
    Rgb(String),
}

impl From<Hue> for ColorInfo {
    fn from(hue: Hue) -> Self {
        match hue {
            Hue::Indexed(index) => ColorInfo::Palette(index),
            Hue::Rgb(rgb) => ColorInfo::Rgb(format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)),
        }
    }
}

impl From<&StyleRun> for RunInfo {
    fn from(run: &StyleRun) -> Self {
        let style = run.style;
        Self {
            col: run.col,
            len: run.len,
            fg: style.fg.map(ColorInfo::from),
            bg: style.bg.map(ColorInfo::from),
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            inverse: style.inverse,
            dim: style.dim,
        }
    }
}

impl ModesInfo {
    pub fn of(modes: Modes) -> Self {
        let mouse = match modes.mouse_tracking() {
            MouseTracking::Off => "off",
            MouseTracking::X10 => "x10",
            MouseTracking::Click => "click",
            MouseTracking::Drag => "drag",
            MouseTracking::Motion => "motion",
            // A tracking level a newer engine knows.
            _ => "other",
        };
        let mouse_encoding = match modes.mouse_encoding() {
            MouseEncoding::X10 => "x10",
            MouseEncoding::Utf8 => "utf8",
            MouseEncoding::Urxvt => "urxvt",
            MouseEncoding::Sgr => "sgr",
            MouseEncoding::SgrPixels => "sgr_pixels",
            _ => "other",
        };
        Self {
            app_cursor: modes.contains(Modes::APP_CURSOR),
            app_keypad: modes.contains(Modes::APP_KEYPAD),
            bracketed_paste: modes.contains(Modes::BRACKETED_PASTE),
            locator: modes.contains(Modes::LOCATOR),
            mouse: mouse.into(),
            mouse_encoding: mouse_encoding.into(),
            kitty_keyboard: modes.kitty_flags().bits(),
        }
    }
}

fn shape_name(shape: CursorShape) -> &'static str {
    match shape {
        CursorShape::Block => "block",
        CursorShape::Underline => "underline",
        CursorShape::Beam => "beam",
        CursorShape::HollowBlock => "hollow_block",
        CursorShape::Hidden => "hidden",
    }
}

impl Read {
    /// A capture of terminal `terminal` on the wire.
    pub fn of(terminal: i64, title: Option<String>, capture: Capture) -> Self {
        Self {
            terminal,
            title,
            screen: if capture.alt_screen { "alt" } else { "main" }.into(),
            size: Size { rows: capture.rows, cols: capture.cols },
            history: capture.history,
            cursor: CursorInfo {
                row: capture.cursor.row,
                col: capture.cursor.col,
                visible: capture.cursor.visible,
                shape: capture.cursor.shape.map(|shape| shape_name(shape).to_string()),
            },
            modes: ModesInfo::of(capture.modes),
            first_line: capture.first_line,
            lines: capture.lines,
            styles: capture
                .styles
                .map(|rows| rows.iter().map(|runs| runs.iter().map(RunInfo::from).collect()).collect()),
            truncated: capture.truncated,
            revision: capture.revision,
            block: capture.block.map(|block| i64::try_from(block.id).unwrap_or(i64::MAX)),
        }
    }

    /// About how many bytes the reply will be, cheaply and never under: when this is well under
    /// [`REPLY_MAX_BYTES`] it need not be measured.
    pub fn size_estimate(&self) -> usize {
        // A character may be written as a `\uXXXX` escape, six bytes for at most three.
        let text: usize = self.lines.iter().map(|line| line.len() * 2 + 4).sum();
        let runs: usize = self.styles.iter().flatten().map(|row| row.len() * 96 + 2).sum();
        text + runs + 1024
    }

    /// Drop the oldest quarter of the lines (and their styles), saying so.
    pub fn drop_oldest(&mut self) {
        let drop = (self.lines.len() / 4).max(1).min(self.lines.len());
        self.lines.drain(..drop);
        if let Some(styles) = &mut self.styles {
            styles.drain(..drop.min(styles.len()));
        }
        // Without `join` a line is a row; with it the first line moves by at least that.
        self.first_line = self.first_line.saturating_add(i32::try_from(drop).unwrap_or(i32::MAX));
        self.truncated = true;
    }
}

/// `blocks`' result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blocks {
    pub terminal: i64,
    /// The shell has integration; without it there are no blocks to read.
    pub integration: bool,
    pub blocks: Vec<BlockInfo>,
}

/// One command block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockInfo {
    pub id: i64,
    pub command: String,
    /// `prompt` (nothing ran), `running` or `finished`.
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_ms: Option<u64>,
    pub first_line: i32,
    pub last_line: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    pub truncated: bool,
}

impl From<&BlockCapture> for BlockInfo {
    fn from(block: &BlockCapture) -> Self {
        use wormhole_term::BlockState;
        let (state, exit_code, took_ms) = match block.state {
            BlockState::Prompt => ("prompt", None, None),
            BlockState::Running { .. } => ("running", None, None),
            BlockState::Finished { code, took } => {
                ("finished", code, took.map(|took| u64::try_from(took.as_millis()).unwrap_or(u64::MAX)))
            }
        };
        Self {
            id: i64::try_from(block.id).unwrap_or(i64::MAX),
            command: block.command.clone(),
            state: state.into(),
            exit_code,
            took_ms,
            first_line: block.first_line,
            last_line: block.last_line,
            output: block.output.clone(),
            truncated: block.truncated,
        }
    }
}

impl Blocks {
    pub fn of(terminal: i64, capture: &BlocksCapture) -> Self {
        Self {
            terminal,
            integration: capture.integration,
            blocks: capture.blocks.iter().map(BlockInfo::from).collect(),
        }
    }

    /// About how many bytes the reply will be (see [`Read::size_estimate`]).
    pub fn size_estimate(&self) -> usize {
        self.blocks
            .iter()
            .map(|block| {
                block.output.as_ref().map_or(0, |output| output.len() * 2) + block.command.len() * 2 + 256
            })
            .sum::<usize>()
            + 256
    }

    /// Drop the output of the oldest block that still has some, saying so.
    pub fn drop_oldest_output(&mut self) -> bool {
        match self.blocks.iter_mut().find(|block| block.output.is_some()) {
            Some(block) => {
                block.output = None;
                block.truncated = true;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockRef, BlocksSpec, Newest, Range, ReadSpec};
    use crate::ipc::control::ErrorCode;

    fn spec(range: Range) -> ReadSpec {
        ReadSpec { target: None, range, join: false, styles: false }
    }

    #[test]
    fn a_range_goes_to_the_screen_as_it_is_or_is_refused_with_a_reason() {
        let code = |spec: ReadSpec| spec.capture_spec().map(|_| ()).map_err(|e| e.code);
        assert_eq!(code(spec(Range::Screen)), Ok(()));
        assert_eq!(code(spec(Range::Last { lines: 5000 })), Ok(()));
        assert_eq!(code(spec(Range::Last { lines: 5001 })), Err(ErrorCode::TooLarge));
        assert_eq!(code(spec(Range::Last { lines: 0 })), Err(ErrorCode::BadRequest));
        assert_eq!(code(spec(Range::Lines { from: -4999, to: 0 })), Ok(()));
        assert_eq!(code(spec(Range::Lines { from: -5000, to: 0 })), Err(ErrorCode::TooLarge));
        assert_eq!(code(spec(Range::Lines { from: 3, to: 2 })), Err(ErrorCode::BadRequest));
        assert_eq!(
            code(spec(Range::Lines { from: i32::MIN, to: i32::MAX })),
            Err(ErrorCode::TooLarge),
            "a range as wide as the numbers go"
        );
        let styled = |range| ReadSpec { styles: true, ..spec(range) };
        assert_eq!(code(styled(Range::Last { lines: 500 })), Ok(()));
        assert_eq!(code(styled(Range::Last { lines: 501 })), Err(ErrorCode::TooLarge));
        assert_eq!(
            code(ReadSpec { styles: true, join: true, ..spec(Range::Screen) }),
            Err(ErrorCode::BadRequest)
        );
    }

    #[test]
    fn blocks_are_asked_for_by_number_with_a_default_and_a_cap() {
        let ask = |last| BlocksSpec { target: None, last, output: true }.blocks_spec().map(|s| s.last);
        assert_eq!(ask(None).ok(), Some(10));
        assert_eq!(ask(Some(100)).ok(), Some(100));
        assert_eq!(ask(Some(101)).map_err(|e| e.code), Err(ErrorCode::TooLarge));
        assert_eq!(ask(Some(0)).map_err(|e| e.code), Err(ErrorCode::BadRequest));
    }

    #[test]
    fn the_wire_shapes_are_the_documented_ones() {
        let read: ReadSpec = serde_json::from_str(
            r#"{"target":{"kind":"terminal","id":3},"range":{"kind":"lines","from":-200,"to":39},"join":true}"#,
        )
        .expect("parses");
        assert_eq!(read.range, Range::Lines { from: -200, to: 39 });
        assert!(read.join && !read.styles);
        let block: ReadSpec = serde_json::from_str(r#"{"range":{"kind":"block","id":57}}"#).expect("parses");
        assert_eq!(block.range, Range::Block { id: BlockRef::Id(57) });
        let newest: ReadSpec =
            serde_json::from_str(r#"{"range":{"kind":"block","id":"last"}}"#).expect("parses");
        assert_eq!(newest.range, Range::Block { id: BlockRef::Newest(Newest::Last) });
        let bare: ReadSpec = serde_json::from_str(r#"{"range":{"kind":"block"}}"#).expect("parses");
        assert_eq!(bare.range, newest.range, "no id is the newest");
        let none: ReadSpec = serde_json::from_str("{}").expect("parses");
        assert_eq!(none.range, Range::Screen, "the screen unless told");
        assert!(serde_json::from_str::<ReadSpec>(r#"{"range":{"kind":"block","id":"first"}}"#).is_err());
    }
}
