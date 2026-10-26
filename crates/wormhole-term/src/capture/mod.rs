//! A terminal's screen, history or one of its command blocks as text, read from any thread: what
//! `wh read` returns, with the size, the cursor and the modes the program has set.
//!
//! The rows are copied under the screen's lock in chunks of [`rows::CHUNK`] and turned into
//! strings after it is let go of, into buffers the caller keeps ([`ReadScratch`]), so a read of
//! the screen is one short hold, the same order of work as a frame's snapshot. A range longer than
//! one chunk anchors its next chunk and its end with pins, which follow their rows while output
//! scrolls the history between two holds: the lines come back as the range was when it began, or
//! the read says it was cut short (`truncated`) when the history dropped what it was reading.

mod blocks;
mod rows;
#[cfg(test)]
mod tests;

use wormhole_vt::{Column, CursorShape, Line, Modes, Point};

pub use blocks::{BlockCapture, BlocksCapture, BlocksSpec};
pub(crate) use rows::{CHUNK, RowBuf, copy_rows};
pub use rows::{Hue, ReadScratch, Style, StyleRun};

use crate::ScreenReader;

/// What to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureSpec {
    pub range: CaptureRange,
    /// Put soft-wrapped rows back together into the line the program printed.
    pub join: bool,
    /// Add each row's colours and attributes (not with `join`: columns would not mean anything).
    pub styles: bool,
    /// Keep at most this many bytes of text, the newest.
    pub max_bytes: usize,
}

/// Which lines. Grid lines: 0 is the live screen's top row, history is negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureRange {
    /// The live screen, whatever the person has scrolled to: the alternate screen while a program
    /// has it.
    Screen,
    /// The last `n` rows of history and screen.
    Last(usize),
    /// `from..=to`, cut to what exists.
    Lines { from: i32, to: i32 },
    /// A command block's prompt, command and output: this one, or the newest that ran.
    Block(Option<u64>),
}

/// Why nothing was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureError {
    /// The shell has no integration, a full-screen program has the screen, or no command has run.
    NoBlock,
    /// The block's rows are no longer on the grid.
    BlockGone,
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CaptureError::NoBlock => "no command block to read",
            CaptureError::BlockGone => "that block has left the history",
        })
    }
}

impl std::error::Error for CaptureError {}

/// Where the cursor is and whether it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureCursor {
    /// The grid line (a row of the live screen).
    pub row: i32,
    pub col: usize,
    pub visible: bool,
    /// What the program asked for; `None` while it is hidden.
    pub shape: Option<CursorShape>,
}

/// The command block a read covered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedBlock {
    pub id: u64,
    pub command: String,
}

/// What a read found.
#[derive(Clone, Debug)]
pub struct Capture {
    /// A full-screen program has the alternate screen.
    pub alt_screen: bool,
    pub rows: usize,
    pub cols: usize,
    /// Lines of history above the live screen.
    pub history: usize,
    pub cursor: CaptureCursor,
    pub modes: Modes,
    /// The grid line the first returned line starts on.
    pub first_line: i32,
    pub lines: Vec<String>,
    /// One list of runs per line, when asked for.
    pub styles: Option<Vec<Vec<StyleRun>>>,
    /// Lines were left out: the history dropped them while the read went on, or the text was over
    /// the byte cap (the oldest go first).
    pub truncated: bool,
    /// Compare two reads' revisions to tell whether the terminal printed in between.
    pub revision: u64,
    pub block: Option<CapturedBlock>,
}

/// The text of a read as it is put together: lines, how many rows each took and their styles.
struct Lines {
    join: bool,
    styles: bool,
    cols: usize,
    lines: Vec<String>,
    rows_per_line: Vec<u32>,
    runs: Vec<Vec<StyleRun>>,
    open: String,
    open_rows: u32,
}

impl Lines {
    fn new(spec: &CaptureSpec, cols: usize) -> Self {
        Self {
            join: spec.join && !spec.styles,
            styles: spec.styles,
            cols,
            lines: Vec::new(),
            rows_per_line: Vec::new(),
            runs: Vec::new(),
            open: String::new(),
            open_rows: 0,
        }
    }

    fn add(&mut self, rows: &[RowBuf]) {
        for row in rows {
            row.push_text(&mut self.open, self.cols);
            self.open_rows += 1;
            if self.styles {
                self.runs.push(row.style_runs());
            }
            if !(self.join && row.wrapped) {
                self.finish();
            }
        }
    }

    fn finish(&mut self) {
        let kept = self.open.trim_end().len();
        self.open.truncate(kept);
        self.lines.push(std::mem::take(&mut self.open));
        self.rows_per_line.push(std::mem::take(&mut self.open_rows));
    }

    /// The lines, keeping the newest `max_bytes` of text.
    fn into_capped(mut self, max_bytes: usize) -> Capped {
        if self.open_rows > 0 {
            self.finish();
        }
        let mut bytes: usize = self.lines.iter().map(String::len).sum();
        let mut dropped = 0;
        while bytes > max_bytes && dropped + 1 < self.lines.len() {
            bytes -= self.lines[dropped].len();
            dropped += 1;
        }
        let rows_dropped: u32 = self.rows_per_line[..dropped].iter().sum();
        self.lines.drain(..dropped);
        let mut cut = dropped > 0;
        // A single line over the cap is cut from its start too: the newest text is what is wanted.
        if let Some(only) = self.lines.first_mut().filter(|line| line.len() > max_bytes) {
            let mut start = only.len() - max_bytes;
            while !only.is_char_boundary(start) {
                start += 1;
            }
            only.drain(..start);
            cut = true;
        }
        let styles = self.styles.then(|| {
            let skip = (rows_dropped as usize).min(self.runs.len());
            self.runs.drain(..skip);
            self.runs
        });
        Capped { lines: self.lines, styles, rows_dropped: rows_dropped as i32, cut }
    }
}

/// The text of a read with its byte cap applied.
struct Capped {
    lines: Vec<String>,
    styles: Option<Vec<Vec<StyleRun>>>,
    /// How many rows came off the top.
    rows_dropped: i32,
    /// Anything was left out.
    cut: bool,
}

impl ScreenReader {
    /// Read `spec`'s lines with the screen's size, cursor and modes. `scratch` is what the rows are
    /// copied into: keep one per reading thread.
    pub fn capture(&self, spec: &CaptureSpec, scratch: &mut ReadScratch) -> Result<Capture, CaptureError> {
        self.capture_between(spec, scratch, || {})
    }

    /// [`capture`](Self::capture), calling `between` each time the lock is let go of between two
    /// chunks: a test's way to let output arrive there.
    pub(crate) fn capture_between(
        &self,
        spec: &CaptureSpec,
        scratch: &mut ReadScratch,
        mut between: impl FnMut(),
    ) -> Result<Capture, CaptureError> {
        let mut term = self.term.lock();
        let (top, bottom) = (term.topmost_line().0, term.bottommost_line().0);
        let modes = term.mode();
        let alt = modes.contains(Modes::ALT_SCREEN);
        let mut block = None;
        let (first, mut last) = match spec.range {
            CaptureRange::Screen => (0, bottom),
            CaptureRange::Last(n) => {
                let n = i32::try_from(n).unwrap_or(i32::MAX);
                ((bottom - n + 1).max(top), bottom)
            }
            CaptureRange::Lines { from, to } => (from.max(top), to.min(bottom)),
            CaptureRange::Block(id) => {
                let log = self.shared.commands();
                let found = match id {
                    Some(id) => crate::blocks::block(&term, id, &log),
                    None => crate::blocks::newest_past_prompt(&term, &log),
                };
                let found = found.ok_or(if id.is_some() {
                    CaptureError::BlockGone
                } else {
                    CaptureError::NoBlock
                })?;
                block = Some(CapturedBlock { id: found.id, command: found.command.clone() });
                (found.start.0.max(top), found.end.0.min(bottom))
            }
        };
        let cursor = {
            let cursor = term.cursor();
            CaptureCursor {
                row: cursor.point.line.0,
                col: cursor.point.column.0,
                visible: cursor.shape != CursorShape::Hidden,
                shape: (cursor.shape != CursorShape::Hidden).then_some(cursor.shape),
            }
        };
        let (screen_rows, cols, history) = (term.screen_lines(), term.columns(), term.history_size());
        let mut text = Lines::new(spec, cols);
        let mut truncated = false;
        let mut next = first;
        // The pins that carry a range of more than one chunk across the holds of the lock: the
        // first row not read yet, and the range's end.
        let mut pins = None;
        while next <= last {
            let chunk_end = (next + CHUNK as i32 - 1).min(last);
            let n = copy_rows(&term, next, chunk_end, scratch);
            let more = chunk_end < last;
            if more {
                let after = Point::new(Line(chunk_end + 1), Column(0));
                match pins {
                    None => pins = Some((term.pin(after), term.pin(Point::new(Line(last), Column(0))))),
                    Some((ahead, _)) => term.move_pin(ahead, after),
                }
            } else if let Some((ahead, end)) = pins.take() {
                term.unpin(ahead);
                term.unpin(end);
            }
            drop(term);
            text.add(scratch.rows(n));
            if !more {
                break;
            }
            between();
            term = self.term.lock();
            let Some((ahead, end)) = pins else { break };
            // The pins belong to the screen they were made on: if a program swapped screens, they
            // are out of reach (and not ours to remove from the other one).
            let same_screen = term.mode().contains(Modes::ALT_SCREEN) == alt;
            let points =
                if same_screen { (term.pin_point(ahead), term.pin_point(end)) } else { (None, None) };
            match points {
                (Some(ahead_at), Some(end_at)) => (next, last) = (ahead_at.line.0, end_at.line.0),
                _ => {
                    truncated = true;
                    if same_screen {
                        term.unpin(ahead);
                        term.unpin(end);
                    }
                    break;
                }
            }
        }
        let revision = self.shared.revision();
        let Capped { lines, styles, rows_dropped, cut } = text.into_capped(spec.max_bytes);
        truncated |= cut;
        Ok(Capture {
            alt_screen: alt,
            rows: screen_rows,
            cols,
            history,
            cursor,
            modes,
            first_line: first + rows_dropped,
            lines,
            styles,
            truncated,
            revision,
            block,
        })
    }
}
