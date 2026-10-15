//! The grid's text, copied for a search that runs somewhere else.
//!
//! A search over scrollback must not hold the lock the parser needs: the host copies a window of
//! rows here, lets go, and searches the copy (with whatever matcher it likes; this crate has
//! none). A match is a range of bytes of a line's text; [`CopiedLine::cells`] turns it back into
//! the cells it covers, across the soft wraps the line was joined over.
//!
//! # Shape of the copy
//!
//! - **Lines are the program's**, not the screen's: rows soft-wrapped into each other are one line
//!   of text, and a match may span the wrap but never a line break. The blanks at the end of a
//!   line are cut, so a search for spaces doesn't find the rest of a row.
//! - **Text and cells.** A wide character is one character (its second cell is not text), the
//!   characters that combine with a cell's are in the text after it, an empty cell is a space.
//!   Mapping a byte back to a cell needs a table, except for the common line that is plain ASCII
//!   where byte `n` is cell `n`: such a line costs no table at all.
//! - **A window**, not the whole history: [`Grid::copy_text`] takes about `rows` rows and says
//!   where the next window starts, so the lock is held for a bounded time per call.
//! - **Allocation.** The copy owns its buffers and [`TextCopy`] keeps them between calls, so a
//!   search that walks the history in windows allocates once.

use std::ops::Range;

use crate::types::{Column, Line, Point};

use super::Grid;
use super::text::text_len;

/// A window takes rows past the requested count only to finish a line; a line wrapped over more
/// rows than this many times the request is cut there (a hostile program can make one of any
/// length).
const WINDOW_SLACK: usize = 4;

/// The most rows one window takes (so that cell numbers fit in 31 bits), and the most text: a
/// program that gives every cell dozens of combining marks must not make a copy of gigabytes. A
/// window past either ends at the line it is in.
const MAX_WINDOW_ROWS: usize = 1 << 14;
const MAX_WINDOW_BYTES: usize = 1 << 28;

/// The high bit of [`CharAt::cell`]: the character is wide, so its match ends one cell later.
const WIDE: u32 = 1 << 31;

/// Where one character of a line's text is: its first byte, and its first cell counted from the
/// line's first row (a row holds `cols` cells), with the high bit saying the character is wide.
#[derive(Clone, Copy, Debug)]
struct CharAt {
    byte: u32,
    cell: u32,
}

/// How to find a character's cell from its byte.
#[derive(Clone, Copy, Debug)]
enum Map {
    /// Every byte is a cell of its own: plain ASCII from the line's first cell on.
    Identity,
    /// Entries `from..to` of the copy's table.
    Table { from: u32, to: u32 },
}

#[derive(Clone, Copy, Debug)]
struct Record {
    /// The grid line of the line's first row.
    first: i32,
    start: u32,
    end: u32,
    map: Map,
}

/// Rows of the grid copied as text; see the module docs.
#[derive(Clone, Debug, Default)]
pub struct TextCopy {
    cols: usize,
    history: usize,
    text: String,
    lines: Vec<Record>,
    table: Vec<CharAt>,
}

/// One line of a [`TextCopy`].
#[derive(Clone, Copy, Debug)]
pub struct CopiedLine<'a> {
    copy: &'a TextCopy,
    record: Record,
}

impl TextCopy {
    /// An empty copy.
    pub fn new() -> TextCopy {
        TextCopy::default()
    }

    /// The lines copied, top to bottom.
    pub fn lines(&self) -> impl ExactSizeIterator<Item = CopiedLine<'_>> {
        self.lines.iter().map(|&record| CopiedLine { copy: self, record })
    }

    /// The history's length when this was copied: output since moves the lines up by as many as
    /// it grew.
    pub fn history(&self) -> usize {
        self.history
    }

    /// What the buffers can hold without growing: text bytes, table entries, lines.
    #[cfg(test)]
    pub(super) fn capacities(&self) -> (usize, usize, usize) {
        (self.text.capacity(), self.table.capacity(), self.lines.capacity())
    }

    fn clear(&mut self) {
        self.text.clear();
        self.lines.clear();
        self.table.clear();
    }
}

impl<'a> CopiedLine<'a> {
    /// The line's text.
    pub fn text(&self) -> &'a str {
        &self.copy.text[self.record.start as usize..self.record.end as usize]
    }

    /// The grid line of the line's first row.
    pub fn first_line(&self) -> Line {
        Line(self.record.first)
    }

    /// The first and last cell of the characters in `bytes` of [`CopiedLine::text`] (a wide
    /// character's last cell is its second), or `None` for an empty range or one that is not
    /// inside the text. A range that starts or ends inside a character covers that character.
    pub fn cells(&self, bytes: Range<usize>) -> Option<(Point, Point)> {
        if bytes.start >= bytes.end || bytes.end > self.text().len() {
            return None;
        }
        let (first, last) = (self.char_at(bytes.start), self.char_at(bytes.end - 1));
        let end = (last.cell & !WIDE) + u32::from(last.cell & WIDE != 0);
        Some((self.point(first.cell & !WIDE), self.point(end)))
    }

    /// The character the byte `byte` of the text is in.
    fn char_at(&self, byte: usize) -> CharAt {
        match self.record.map {
            Map::Identity => CharAt { byte: byte as u32, cell: byte as u32 },
            Map::Table { from, to } => {
                let table = &self.copy.table[from as usize..to as usize];
                let ix = table.partition_point(|c| c.byte as usize <= byte).saturating_sub(1);
                table[ix]
            }
        }
    }

    fn point(&self, cell: u32) -> Point {
        let cols = self.copy.cols.max(1);
        let cell = cell as usize;
        Point::new(Line(self.record.first + (cell / cols) as i32), Column(cell % cols))
    }
}

/// A line being copied: whether its text is still plain ASCII from the first cell, and where its
/// table starts.
struct Building {
    start: usize,
    table_from: usize,
    plain: bool,
    /// Cells of the rows before this one.
    base: usize,
}

impl Building {
    /// Note that the character about to be pushed is at `cell` of the row and `ascii` single byte,
    /// and make the table if it is not an identity any more.
    fn character(&mut self, copy: &mut TextCopy, cell: usize, plain_char: bool, wide: bool) {
        let byte = copy.text.len() - self.start;
        let at = self.base + cell;
        if self.plain {
            if plain_char && byte == at {
                return;
            }
            // The text stops being one byte per cell: make the table from what is already there.
            self.plain = false;
            copy.table.extend((0..byte as u32).map(|i| CharAt { byte: i, cell: i }));
        }
        copy.table.push(CharAt { byte: byte as u32, cell: at as u32 | if wide { WIDE } else { 0 } });
    }
}

impl Grid {
    /// Copy about `rows` rows from `from` down into `out` (emptied first), whole lines: it stops
    /// at the first line end at or after `rows`, so a line is never split between windows.
    /// Returns the line the next window starts at, `None` when the copy reached the bottom of
    /// the screen. A `from` above the history starts at its top.
    ///
    /// Everything is read under one borrow of the grid and takes time in proportion to `rows`,
    /// so a host that must not hold its lock long asks for a few hundred rows at a time.
    pub fn copy_text(&self, from: Line, rows: usize, out: &mut TextCopy) -> Option<Line> {
        out.clear();
        out.cols = self.cols;
        out.history = self.history_len();
        let (top, bottom) = (self.topmost().0, self.bottommost().0);
        let rows = rows.clamp(1, MAX_WINDOW_ROWS);
        let limit = rows.saturating_mul(WINDOW_SLACK);
        let mut y = from.0.max(top);
        let mut copied = 0usize;
        while y <= bottom && copied < rows && out.text.len() < MAX_WINDOW_BYTES {
            let mut line =
                Building { start: out.text.len(), table_from: out.table.len(), plain: true, base: 0 };
            let first = y;
            while let Some((page_ix, row)) = self.locate_line(Line(y)) {
                let page = &self.pages[page_ix];
                let wrapped = page.rows[row].wrapped();
                let cells = page.cells(row);
                // A row that goes on keeps its blanks: they are spaces of the line.
                let len = if wrapped { cells.len() } else { text_len(cells) };
                for (x, &cell) in cells.iter().enumerate().take(len) {
                    if cell.is_spacer_tail() || cell.is_spacer_head() {
                        continue;
                    }
                    let c = cell.character().unwrap_or(' ');
                    let marks =
                        if cell.has_grapheme() { page.graphemes.get(page.offset(row, x)) } else { &[] };
                    line.character(out, x, c.is_ascii() && marks.is_empty(), cell.is_wide());
                    out.text.push(c);
                    out.text.extend(marks);
                }
                copied += 1;
                y += 1;
                line.base += self.cols;
                if !wrapped || y > bottom || copied >= limit || out.text.len() >= MAX_WINDOW_BYTES {
                    break;
                }
            }
            finish_line(out, &line, first);
        }
        (y <= bottom).then_some(Line(y))
    }
}

/// Cut the blanks off the end of the line just copied and record it.
fn finish_line(copy: &mut TextCopy, line: &Building, first: i32) {
    let kept = copy.text[line.start..].trim_end_matches(' ').len();
    copy.text.truncate(line.start + kept);
    let map = if line.plain {
        Map::Identity
    } else {
        let from = line.table_from;
        let in_text = copy.table[from..].partition_point(|c| (c.byte as usize) < kept);
        copy.table.truncate(from + in_text);
        Map::Table { from: from as u32, to: copy.table.len() as u32 }
    };
    copy.lines.push(Record { first, start: line.start as u32, end: (line.start + kept) as u32, map });
}
