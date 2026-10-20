//! Writing and erasing cells of the live screen, and the marks on its rows.
//!
//! Each call is one row (or a block of whole rows); the terminal state decides what a sequence
//! means and where the cursor goes, these only change cells. A call outside the screen does
//! nothing. Writes never split a wide character: overwriting half of one blanks the other half.

use std::ops::Range;

use crate::types::Color;

use super::{Grid, Pen, Spare};

impl Grid {
    /// Write printable ASCII `bytes` (0x20..=0x7E) into row `y` from column `x`, in `pen`'s style.
    /// The run stops at the last column; the number of bytes written. This is the hot path: a bound
    /// check, a one-pass test of whether the cells it replaces hold anything to give back, and a tight
    /// copy.
    #[inline]
    pub fn write_ascii(&mut self, y: usize, x: usize, bytes: &[u8], pen: &Pen) -> usize {
        let n = bytes.len().min(self.cols.saturating_sub(x));
        if y >= self.rows || n == 0 {
            return 0;
        }
        let (page, row) = self.page_row_mut(y);
        page.write_ascii(row, x, &bytes[..n], pen);
        n
    }

    /// Write the cluster `c` + `marks` of `width` 1 or 2 cells at `x` of row `y`. False, and nothing
    /// written, when it doesn't fit: a wide character at the last column. The terminal then wraps
    /// the line ([`Self::write_spacer_head`], [`Self::set_wrapped`]) and writes it on the next row.
    pub fn write_cluster(
        &mut self,
        y: usize,
        x: usize,
        c: char,
        marks: &[char],
        width: u8,
        pen: &Pen,
    ) -> bool {
        let width = width.clamp(1, 2);
        if y >= self.rows || x >= self.cols || width as usize > self.cols - x {
            return false;
        }
        let (page, row) = self.page_row_mut(y);
        page.write_char(row, x, c, marks, width, pen);
        true
    }

    /// Leave the last cell of row `y` as the blank a wide character that didn't fit leaves behind.
    pub fn write_spacer_head(&mut self, y: usize, pen: &Pen) {
        if y < self.rows {
            let (page, row) = self.page_row_mut(y);
            page.write_spacer_head(row, pen);
        }
    }

    /// Add a zero-width character `c` to the cluster in the cell at `x` of row `y` (the cell of the
    /// character before the cursor; a wide character's second cell counts as its first). False when
    /// there is no such cell or it can hold no more.
    pub fn push_mark(&mut self, y: usize, x: usize, c: char) -> bool {
        if y >= self.rows || x >= self.cols {
            return false;
        }
        let (page, row) = self.page_row_mut(y);
        page.push_mark(row, x, c)
    }

    /// Make the character at `x` of row `y` one cell wide or two: a variation selector changed how
    /// it is drawn. Widening takes the next cell over; false when it can't (no next cell, nothing
    /// there to widen).
    pub fn set_width(&mut self, y: usize, x: usize, width: u8) -> bool {
        if y >= self.rows || x >= self.cols {
            return false;
        }
        let (page, row) = self.page_row_mut(y);
        page.set_width(row, x, width)
    }

    /// Blank the cells `x` of row `y` with `bg` as their background (background-color erase).
    /// `spare` is what stays (`true` for DECSEL's protected cells). Erasing up to the last column ends the row's
    /// soft wrap; erasing all of it also forgets its block.
    /// Pictures covering any of the cells are deleted, unless the erase is selective.
    pub fn erase(&mut self, y: usize, x: Range<usize>, bg: Color, spare: impl Into<Spare>) {
        let spare = spare.into();
        let end = x.end.min(self.cols);
        if y >= self.rows || x.start >= end {
            return;
        }
        if spare != Spare::Selective && !self.placements.is_empty() {
            self.placements_erased(y..y + 1, x.start..end);
        }
        self.erase_cells(y, x, bg, spare);
    }

    /// [`Self::erase`] without the pictures: for the scrolls, which place them themselves.
    pub(super) fn erase_cells(&mut self, y: usize, x: Range<usize>, bg: Color, spare: Spare) {
        let end = x.end.min(self.cols);
        if y >= self.rows || x.start >= end {
            return;
        }
        let (page, row) = self.page_row_mut(y);
        page.erase(row, x.start..end, bg, spare);
    }

    /// Blank whole rows `ys` (ED, and what a scroll's new lines are). Their soft wraps, blocks
    /// and sizes (`line_size`) are forgotten, but not by an erase that spares cells (selective, or
    /// guarded areas in use). Pictures covering any of the rows are deleted, unless the erase is
    /// selective.
    pub fn erase_rows(&mut self, ys: Range<usize>, bg: Color, spare: impl Into<Spare>) {
        let spare = spare.into();
        if spare != Spare::Selective && !self.placements.is_empty() {
            self.placements_erased(ys.start..ys.end.min(self.rows), 0..self.cols);
        }
        let whole_screen = ys.start == 0 && ys.end >= self.rows;
        self.erase_rows_cells(ys, bg, spare);
        if whole_screen && spare == Spare::Nothing {
            self.recount_line_sizes();
        }
    }

    /// [`Self::erase_rows`] without the pictures: for the scrolls, which place them themselves.
    pub(super) fn erase_rows_cells(&mut self, ys: Range<usize>, bg: Color, spare: Spare) {
        let cols = self.cols;
        for y in ys.start..ys.end.min(self.rows) {
            let (page, row) = self.page_row_mut(y);
            if spare != Spare::Nothing {
                page.erase(row, 0..cols, bg, spare);
            } else {
                page.clear_row(row, bg);
            }
        }
    }

    /// ICH: insert `n` blank cells at `x` of row `y`, pushing the cells up to column `right` along.
    /// What is pushed past `right` is lost. Pictures covering the cells from `x` to `right` are
    /// deleted: the cells under them moved.
    pub fn insert_cells(&mut self, y: usize, x: usize, right: usize, n: usize, bg: Color) {
        let right = right.min(self.cols - 1);
        if y < self.rows && n > 0 {
            self.placements_erased(y..y + 1, x..right + 1);
            let (page, row) = self.page_row_mut(y);
            page.insert_cells(row, x, right, n, bg);
        }
    }

    /// DCH: delete `n` cells at `x` of row `y`, pulling the cells up to column `right` left; blank
    /// cells come in at `right`. Pictures covering the cells from `x` to `right` are deleted.
    pub fn delete_cells(&mut self, y: usize, x: usize, right: usize, n: usize, bg: Color) {
        let right = right.min(self.cols - 1);
        if y < self.rows && n > 0 {
            self.placements_erased(y..y + 1, x..right + 1);
            let (page, row) = self.page_row_mut(y);
            page.delete_cells(row, x, right, n, bg);
        }
    }

    /// HT started on column `x` of row `y`: remember it on the cell, if it is blank, for text
    /// copies (see [`TextShape`](super::TextShape)'s tabs).
    pub fn mark_tab(&mut self, y: usize, x: usize) {
        if y < self.rows && x < self.cols {
            let (page, row) = self.page_row_mut(y);
            page.mark_tab(row, x);
        }
    }

    /// Mark the live row `y` as holding block `block` and nothing else known: the terminal calls
    /// this for a prompt it sees begin on a row that has no text yet.
    pub fn set_row_block(&mut self, y: usize, block: u32) {
        if y < self.rows {
            let (page, row) = self.page_row_mut(y);
            page.set_block(row, block);
        }
    }

    /// Set or clear the soft-wrap flag of live row `y`: its line goes on in the next row.
    pub fn set_wrapped(&mut self, y: usize, wrapped: bool) {
        if y < self.rows {
            let (page, row) = self.page_row_mut(y);
            page.set_wrapped(row, wrapped);
        }
    }
}
