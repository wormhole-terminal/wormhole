//! The grid's cells as text: a range to copy, a line to restore, a span to compare.
//!
//! What is copied is what a reader sees: a wide character once (its spacer cells are not text), the
//! characters that combine with a cell's own after it, an empty cell as a space. Lines are the
//! program's, not the screen's: a row that was soft-wrapped continues in the next one without a
//! newline, and the blanks at the end of a row that was not (nothing was written there) are cut.
//! Everything here reads; the output is a `String` the caller owns (the one allocation) or one it
//! passes in.

use crate::types::{Line, PLACEHOLDER, Point};

use super::Grid;
use super::cell::{Cell, Content};
use super::page::Page;

/// How [`Grid::text_between`] reads the range between two points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextShape {
    /// In reading order from the first point to the second: the rows in between whole, the first
    /// from its column, the last up to its (a mouse selection that is not a block).
    Stream,
    /// The rectangle the points span: every row between them, the same columns on each (a block
    /// selection).
    Block,
}

/// Whether the blanks at the end of a row are part of its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trim {
    /// Cut them: nothing was written there.
    Blanks,
    /// Keep them: the row was soft-wrapped, so they are spaces of the line.
    Keep,
}

/// Whether a gap a tab (HT) left is copied as a `\t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tabs {
    /// A cell HT started on, and the blanks after it up to the next tab stop, are one `\t` while
    /// every cell of the gap is still blank; once a program wrote any of them, what is there is
    /// copied. The text of a copy only: a search needs one character per cell.
    Kept,
    /// Every cell is its own text.
    Spaces,
}

/// The tab stop after column `x` that HT starts at, as a reset terminal has them (every eighth
/// column): where a gap ends. Stops a program moved (HTS, TBC) are not remembered, so a gap that
/// ends at one of those is copied as spaces if the default stop is not where it ends.
fn stop_after(x: usize) -> usize {
    (x / 8 + 1) * 8
}

/// One cell's text: its character (a space for an empty cell), then the characters that combine
/// with it. Nothing for a wide character's second half and for a spacer head.
#[inline]
fn push_cell(page: &Page, row: usize, x: usize, cell: Cell, out: &mut String) {
    if cell.is_spacer_tail() || cell.is_spacer_head() {
        return;
    }
    out.push(cell.character().unwrap_or(' '));
    if cell.has_grapheme() {
        out.extend(page.graphemes.get(page.offset(row, x)));
    }
}

/// The columns of `cells` that hold text: up to and not past the last cell with a character that
/// is not a blank, or with marks (a combining character with nothing to combine with is text).
#[inline]
pub(super) fn text_len(cells: &[Cell]) -> usize {
    cells
        .iter()
        .rposition(|c| c.has_grapheme() || c.character().is_some_and(|c| c != ' '))
        .map_or(0, |p| p + 1)
}

/// The text of columns `from..=to` of `row`. A range that starts on a wide character's second half
/// starts at its first, so the character isn't lost; `trim` decides about the blanks at the end of
/// the row.
fn push_slice(page: &Page, row: usize, from: usize, to: usize, (trim, tabs): (Trim, Tabs), out: &mut String) {
    let cells = page.cells(row);
    let end = match trim {
        Trim::Blanks => text_len(cells).min(to + 1),
        Trim::Keep => (to + 1).min(cells.len()),
    };
    let from =
        if cells.get(from).is_some_and(|c| c.is_spacer_tail()) { from.saturating_sub(1) } else { from };
    let mut x = from;
    while x < end {
        let cell = cells[x];
        // A picture printed as placeholders is not text, nor are the marks that say which tile.
        if tabs == Tabs::Kept && cell.character() == Some(PLACEHOLDER) {
            x += 1;
            continue;
        }
        if tabs == Tabs::Kept && cell.is_tab_start() {
            let gap_end = stop_after(x);
            if gap_end <= end && cells[x + 1..gap_end].iter().all(|c| c.codepoint() == 0 && c.flags() == 0) {
                out.push('\t');
                x = gap_end;
                continue;
            }
        }
        push_cell(page, row, x, cell, out);
        x += 1;
    }
}

/// The whole of `row` as text, one character per cell (no tabs): its trailing blanks cut when
/// `trim` is set.
pub(super) fn push_row_text(page: &Page, row: usize, trim: bool, out: &mut String) {
    let trim = if trim { Trim::Blanks } else { Trim::Keep };
    push_slice(page, row, 0, page.cols().saturating_sub(1), (trim, Tabs::Spaces), out);
}

impl Grid {
    /// The text of the cells from `start` to `end`, as copying them with the mouse gives it.
    /// The points may come in either order; lines outside the grid are clamped to its edges and
    /// columns to its width.
    ///
    /// A `Stream` joins soft-wrapped rows without a newline; every other row end that the range
    /// reaches gets one, except the one at the very end. Blanks at the end of a row that was not
    /// wrapped are cut. A `Block` has a newline between rows and none after the last, and each
    /// of its rows loses the blanks at its end.
    ///
    /// Allocates its result; see [`Grid::push_text_between`] to reuse a buffer.
    pub fn text_between(&self, start: Point, end: Point, shape: TextShape) -> String {
        let mut out = String::new();
        self.push_text_between(start, end, shape, &mut out);
        out
    }

    /// [`Grid::text_between`], appended to `out`.
    pub fn push_text_between(&self, start: Point, end: Point, shape: TextShape, out: &mut String) {
        let last = self.cols - 1;
        let (top, bottom) = (-(self.history_len() as i32), self.rows as i32 - 1);
        match shape {
            TextShape::Stream => {
                let (a, b) = if start <= end { (start, end) } else { (end, start) };
                if b.line.0 < top || a.line.0 > bottom {
                    return;
                }
                // A point beyond the grid's edge stands for that edge's far end: the first
                // column of the top, the last of the bottom.
                let from_col = if a.line.0 < top { 0 } else { a.column.0.min(last) };
                let to_col = if b.line.0 > bottom { last } else { b.column.0.min(last) };
                let (first, final_line) = (a.line.0.max(top), b.line.0.min(bottom));
                let base = out.len();
                for y in first..=final_line {
                    let Some((page_ix, row)) = self.locate_line(Line(y)) else { continue };
                    let page = &self.pages[page_ix];
                    let wrapped = page.rows[row].wrapped();
                    let from = if y == first { from_col } else { 0 };
                    let to = if y == final_line { to_col } else { last };
                    push_slice(
                        page,
                        row,
                        from,
                        to,
                        (if wrapped { Trim::Keep } else { Trim::Blanks }, Tabs::Kept),
                        out,
                    );
                    if !wrapped && to >= last {
                        out.push('\n');
                    }
                }
                // The line end after the last row is the selection's edge, not text in it.
                if out.len() > base && out.ends_with('\n') {
                    out.pop();
                }
            }
            TextShape::Block => {
                let (first, final_line) = (start.line.min(end.line).0, start.line.max(end.line).0);
                if final_line < top || first > bottom {
                    return;
                }
                let (from, to) =
                    (start.column.min(end.column).0.min(last), start.column.max(end.column).0.min(last));
                for y in first.max(top)..=final_line.min(bottom) {
                    let Some((page_ix, row)) = self.locate_line(Line(y)) else { continue };
                    let at = out.len();
                    push_slice(&self.pages[page_ix], row, from, to, (Trim::Blanks, Tabs::Kept), out);
                    // A rectangle's rows end where their text does, not at its edge.
                    let kept = out[at..].trim_end_matches(' ').len();
                    out.truncate(at + kept);
                    if y != final_line.min(bottom) {
                        out.push('\n');
                    }
                }
            }
        }
    }

    /// The program's lines from `first` to `last`, soft-wrapped rows joined, each without the
    /// blanks at its end: what is saved of a terminal to restore it later. A range that starts or
    /// ends inside a wrapped line gives that line's part.
    pub fn logical_lines(&self, first: Line, last: Line) -> Vec<String> {
        let (top, bottom) = (-(self.history_len() as i32), self.rows as i32 - 1);
        let (first, last) = (first.0.max(top), last.0.min(bottom));
        let mut lines = Vec::new();
        let mut current = String::new();
        for y in first..=last {
            let Some((page_ix, row)) = self.locate_line(Line(y)) else { continue };
            let page = &self.pages[page_ix];
            let wrapped = page.rows[row].wrapped();
            push_slice(
                page,
                row,
                0,
                self.cols - 1,
                (if wrapped { Trim::Keep } else { Trim::Blanks }, Tabs::Spaces),
                &mut current,
            );
            if !wrapped || y == last {
                lines.push(std::mem::take(&mut current));
            }
        }
        lines
    }

    /// The text of the cells of `line` that are `content` of command block `block` (OSC 133):
    /// the prompt, the command line or the output, untrimmed, a blank where a cell holds no
    /// character. A row belongs to one block; where two meet on it, to the later.
    pub fn content_text(&self, line: Line, block: u32, content: Content) -> String {
        let mut out = String::new();
        let Some((page_ix, row)) = self.locate_line(line) else { return out };
        let page = &self.pages[page_ix];
        if block == 0 || page.rows[row].block != block {
            return out;
        }
        for (x, &cell) in page.cells(row).iter().enumerate() {
            if cell.content() == content {
                push_cell(page, row, x, cell, &mut out);
            }
        }
        out
    }

    /// The characters of the cells from `start` to `end` inclusive, rows run together with no
    /// newline and no cutting: what a search saw there, to tell whether the cells still hold it.
    pub fn span_text(&self, start: Point, end: Point) -> String {
        let mut out = String::new();
        self.each_span_row(start, end, |page, row, from, to| {
            push_slice(page, row, from, to, (Trim::Keep, Tabs::Spaces), &mut out);
            true
        });
        out
    }

    /// Whether [`Grid::span_text`] of `start..=end` is `text`, without building it: one row's
    /// text at a time in a buffer that is reused.
    pub fn span_is(&self, start: Point, end: Point, text: &str) -> bool {
        let mut rest = text;
        let mut row_text = String::new();
        let walked = self.each_span_row(start, end, |page, row, from, to| {
            row_text.clear();
            push_slice(page, row, from, to, (Trim::Keep, Tabs::Spaces), &mut row_text);
            match rest.strip_prefix(row_text.as_str()) {
                Some(tail) => {
                    rest = tail;
                    true
                }
                None => false,
            }
        });
        walked && rest.is_empty()
    }

    /// Call `each` with the page, row and column range of every row from `start` to `end`;
    /// `false` when a row is missing or `each` said stop.
    fn each_span_row(
        &self,
        start: Point,
        end: Point,
        mut each: impl FnMut(&Page, usize, usize, usize) -> bool,
    ) -> bool {
        if start > end || start.column.0 >= self.cols || end.column.0 >= self.cols {
            return false;
        }
        let last = self.cols - 1;
        for y in start.line.0..=end.line.0 {
            let Some((page_ix, row)) = self.locate_line(Line(y)) else { return false };
            let from = if Line(y) == start.line { start.column.0 } else { 0 };
            let to = if Line(y) == end.line { end.column.0 } else { last };
            if !each(&self.pages[page_ix], row, from, to) {
                return false;
            }
        }
        true
    }
}
