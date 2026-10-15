//! Reading the grid, history included.
//!
//! Reads take a [`Line`](crate::types::Line) (0 is the live screen's top row, the history is
//! negative) and answer `None` or nothing for a line the grid doesn't have.

use crate::types::{Line, Point};

use super::cell::Cell;
use super::style::Style;
use super::{Grid, RowMark};

impl Grid {
    // ---- reading ----

    /// Whether the line at `line` is soft-wrapped into the one below it.
    pub fn is_wrapped(&self, line: Line) -> bool {
        self.locate_line(line).is_some_and(|(page, row)| self.pages[page].rows[row].wrapped())
    }

    /// The cell at `x` of live row `y`.
    pub fn live_cell(&self, y: usize, x: usize) -> Option<Cell> {
        (y < self.rows && x < self.cols).then(|| {
            let (page, row) = self.locate_y(y);
            self.pages[page].cells(row)[x]
        })
    }

    /// The cell at `point` (history included); `None` outside the grid.
    pub fn cell(&self, point: Point) -> Option<Cell> {
        if point.column.0 >= self.cols {
            return None;
        }
        let (page, row) = self.locate_line(point.line)?;
        Some(self.pages[page].cells(row)[point.column.0])
    }

    /// The style of the cell at `point`.
    pub fn style_at(&self, point: Point) -> Option<Style> {
        let cell = self.cell(point)?;
        let (page, _) = self.locate_line(point.line)?;
        Some(*self.pages[page].styles.get(cell.style_id()))
    }

    /// The characters after the first of the cluster in the cell at `point`.
    pub fn marks_at(&self, point: Point) -> &[char] {
        if !self.cell(point).is_some_and(|cell| cell.has_grapheme()) {
            return &[];
        }
        let Some((page, row)) = self.locate_line(point.line) else { return &[] };
        let page = &self.pages[page];
        page.graphemes.get(page.offset(row, point.column.0))
    }

    /// The hyperlink the cell at `point` is under.
    pub fn link_at(&self, point: Point) -> Option<crate::types::Hyperlink> {
        if !self.cell(point)?.has_hyperlink() {
            return None;
        }
        let (page, row) = self.locate_line(point.line)?;
        let page = &self.pages[page];
        page.links.at(page.offset(row, point.column.0)).cloned()
    }

    /// Column of the last cell of `line` that holds a character or a style, plus one; 0 for a
    /// blank row.
    pub fn content_len(&self, line: Line) -> usize {
        let Some((page, row)) = self.locate_line(line) else { return 0 };
        self.pages[page].cells(row).iter().rposition(|cell| !cell.is_blank()).map_or(0, |last| last + 1)
    }

    /// The text of `line`: characters and their marks, wide characters once, empty cells as
    /// spaces, with trailing blanks cut when `trim` is set.
    pub fn row_text(&self, line: Line, trim: bool) -> String {
        let mut out = String::new();
        self.push_row_text(line, trim, &mut out);
        out
    }

    pub(crate) fn push_row_text(&self, line: Line, trim: bool, out: &mut String) {
        let Some((page_ix, row)) = self.locate_line(line) else { return };
        let page = &self.pages[page_ix];
        let cells = page.cells(row);
        let end = if trim {
            // A blank that carries a mark (a combining character with nothing to combine with) is text.
            cells
                .iter()
                .rposition(|c| c.has_grapheme() || c.character().is_some_and(|c| c != ' '))
                .map_or(0, |p| p + 1)
        } else {
            cells.len()
        };
        for (x, cell) in cells[..end].iter().enumerate() {
            if cell.is_spacer_tail() || cell.is_spacer_head() {
                continue;
            }
            out.push(cell.character().unwrap_or(' '));
            if cell.has_grapheme() {
                out.extend(page.graphemes.get(page.offset(row, x)));
            }
        }
    }

    /// The block mark of the row at `line`, read from its cells.
    pub fn row_mark(&self, line: Line) -> Option<RowMark> {
        let (page, row) = self.locate_line(line)?;
        self.pages[page].row_mark(row)
    }
}
