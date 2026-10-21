//! One row, read in place: what a serializer needs without a lookup per cell.
//!
//! The snapshot copy (`copy.rs`) turns a row into the types a host draws and leaves out what
//! only the engine itself acts on (the cell's own flags, the page-local style ids, the row's
//! block). The repaint writer (`terminal::repaint`) and the state dump need all of it, and a row at
//! a time: [`Grid::row_view`] finds the page once, and the view then answers for the whole row.

use crate::types::{Hyperlink, Line, LineSize};

use super::cell::Cell;
use super::page::Page;
use super::style::Style;
use super::{Grid, StyleId};

/// A row of the grid, borrowed.
#[derive(Clone, Copy)]
pub struct RowView<'a> {
    page: &'a Page,
    row: usize,
}

impl Grid {
    /// The row at `line` (history included), `None` for a line the grid does not have.
    pub fn row_view(&self, line: Line) -> Option<RowView<'_>> {
        let (page, row) = self.locate_line(line)?;
        Some(RowView { page: &self.pages[page], row })
    }
}

impl<'a> RowView<'a> {
    /// The row's cells, one per column.
    #[inline]
    pub fn cells(&self) -> &'a [Cell] {
        self.page.cells(self.row)
    }

    /// The style a cell of this row names (ids are the page's, so only this row's cells can ask).
    #[inline]
    pub fn style(&self, id: StyleId) -> &'a Style {
        self.page.styles.get(id)
    }

    /// The characters after the first of the cluster in column `col`.
    #[inline]
    pub fn marks(&self, col: usize) -> &'a [char] {
        self.page.graphemes.get(self.page.offset(self.row, col))
    }

    /// The hyperlinks over the row as `(first column, column after the last, link)`, in column
    /// order, one entry per run of cells under the same link.
    pub fn links(&self) -> impl Iterator<Item = (usize, usize, &'a Hyperlink)> + use<'a> {
        let (page, first) = (self.page, self.page.offset(self.row, 0));
        let end = first + self.page.cells(self.row).len() as u32;
        page.links
            .within(first, end)
            .map(move |(lo, hi, id)| ((lo - first) as usize, (hi - first) as usize, page.links.link(id)))
    }

    /// Whether the line goes on in the next row (a soft wrap).
    #[inline]
    pub fn wrapped(&self) -> bool {
        self.page.rows[self.row].wrapped()
    }

    /// How big the row is drawn.
    #[inline]
    pub fn line_size(&self) -> LineSize {
        self.page.rows[self.row].size()
    }

    /// The command block (OSC 133) number the row was marked with, 0 for none. The mark a host
    /// reads (`RowMark`) is this and the parts the cells carry.
    #[inline]
    pub fn block(&self) -> u32 {
        self.page.rows[self.row].block
    }
}
