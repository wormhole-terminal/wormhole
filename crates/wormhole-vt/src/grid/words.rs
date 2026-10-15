//! Words and lines around a cell: what a double and a triple click select.
//!
//! A word is the run of cells around a point that no separator character interrupts; it goes on
//! across a soft wrap. A line is the program's line: every row it was wrapped over. Both walks
//! are bounded, because the text is a program's and a line of megabytes without a space must not
//! make a click slow.

use crate::types::{Column, Line, Point};

use super::Grid;
use super::cell::Cell;

/// The most cells a word is searched over in each direction.
const MAX_WORD_CELLS: usize = 1 << 16;
/// The most cells a bracket is searched over for its partner.
const MAX_BRACKET_CELLS: usize = 1 << 16;
/// The most rows a line is searched over in each direction.
const MAX_LINE_ROWS: usize = 1 << 12;

/// Whether `cell` ends a word. A wide character's second half never does: it is part of the
/// character.
#[inline]
fn is_separator(cell: Cell, separator: &impl Fn(char) -> bool) -> bool {
    !cell.is_spacer_tail() && separator(cell.character().unwrap_or(' '))
}

impl Grid {
    /// The first cell of the word around `point`: the cell itself is never what stops the walk,
    /// so on a separator the word is the separator and the word before it. `separator` says which
    /// characters end a word (an empty cell is a space).
    pub fn word_start(&self, point: Point, separator: impl Fn(char) -> bool) -> Point {
        let mut at = self.clamp_point(point);
        for _ in 0..MAX_WORD_CELLS {
            let Some(before) = self.step_left(at) else { break };
            if self.cell(before).is_none_or(|cell| is_separator(cell, &separator)) {
                break;
            }
            at = before;
        }
        at
    }

    /// The last cell of the word around `point`; see [`Grid::word_start`].
    pub fn word_end(&self, point: Point, separator: impl Fn(char) -> bool) -> Point {
        let mut at = self.clamp_point(point);
        for _ in 0..MAX_WORD_CELLS {
            let Some(after) = self.step_right(at) else { break };
            if self.cell(after).is_none_or(|cell| is_separator(cell, &separator)) {
                break;
            }
            at = after;
        }
        at
    }

    /// The cell before `at` on the same program line, over a soft wrap. The blank a wide character
    /// left at the end of a row is not a cell of the line: the walk steps over it.
    fn step_left(&self, at: Point) -> Option<Point> {
        let before = if at.column.0 > 0 {
            Point::new(at.line, Column(at.column.0 - 1))
        } else if at.line > self.topmost() && self.is_wrapped(at.line - 1) {
            Point::new(at.line - 1, Column(self.cols - 1))
        } else {
            return None;
        };
        if self.cell(before).is_some_and(|cell| cell.is_spacer_head()) {
            return Some(Point::new(before.line, before.column - 1));
        }
        Some(before)
    }

    /// The cell after `at` on the same program line; see [`Grid::step_left`].
    fn step_right(&self, at: Point) -> Option<Point> {
        let after = if at.column.0 + 1 < self.cols {
            Point::new(at.line, Column(at.column.0 + 1))
        } else if at.line < self.bottommost() && self.is_wrapped(at.line) {
            Point::new(at.line + 1, Column(0))
        } else {
            return None;
        };
        if self.cell(after).is_some_and(|cell| cell.is_spacer_head()) {
            return self.step_right(after);
        }
        Some(after)
    }

    /// The bracket that closes the one at `point`, or opens the closing one there: `()`, `[]`,
    /// `{}` and `<>`, nested ones of the same kind skipped, searched in reading order (rows need
    /// not be wrapped) over [`MAX_BRACKET_CELLS`] cells. `None` when the cell is no bracket or has
    /// no partner.
    pub fn bracket_match(&self, point: Point) -> Option<Point> {
        let at = self.clamp_point(point);
        let c = self.cell(at)?.character()?;
        let (partner, forward) = match c {
            '(' => (')', true),
            '[' => (']', true),
            '{' => ('}', true),
            '<' => ('>', true),
            ')' => ('(', false),
            ']' => ('[', false),
            '}' => ('{', false),
            '>' => ('<', false),
            _ => return None,
        };
        let (mut depth, mut at) = (0usize, at);
        for _ in 0..MAX_BRACKET_CELLS {
            at = if forward {
                if at.column.0 + 1 < self.cols {
                    Point::new(at.line, at.column + 1)
                } else if at.line < self.bottommost() {
                    Point::new(at.line + 1, Column(0))
                } else {
                    return None;
                }
            } else if at.column.0 > 0 {
                Point::new(at.line, at.column - 1)
            } else if at.line > self.topmost() {
                Point::new(at.line - 1, Column(self.cols - 1))
            } else {
                return None;
            };
            match self.cell(at)?.character() {
                Some(found) if found == partner => {
                    if depth == 0 {
                        return Some(at);
                    }
                    depth -= 1;
                }
                Some(found) if found == c => depth += 1,
                _ => {}
            }
        }
        None
    }

    /// The first and last row of the program's line that holds `line`: the rows it was soft-wrapped
    /// over. A line beyond the grid's edge is taken as the nearest row.
    pub fn line_extent(&self, line: Line) -> (Line, Line) {
        let line = Line(line.0.clamp(self.topmost().0, self.bottommost().0));
        let mut first = line;
        for _ in 0..MAX_LINE_ROWS {
            if first > self.topmost() && self.is_wrapped(first - 1) {
                first -= 1;
            } else {
                break;
            }
        }
        let mut last = line;
        for _ in 0..MAX_LINE_ROWS {
            if last < self.bottommost() && self.is_wrapped(last) {
                last += 1;
            } else {
                break;
            }
        }
        (first, last)
    }

    /// `point` moved inside the grid.
    fn clamp_point(&self, point: Point) -> Point {
        Point::new(
            Line(point.line.0.clamp(self.topmost().0, self.bottommost().0)),
            Column(point.column.0.min(self.cols - 1)),
        )
    }

    /// The oldest line of the history (line 0 when there is none).
    pub(crate) fn topmost(&self) -> Line {
        Line(-(self.history_len() as i32))
    }

    /// The bottom row of the live screen.
    pub(crate) fn bottommost(&self) -> Line {
        Line(self.rows as i32 - 1)
    }
}
