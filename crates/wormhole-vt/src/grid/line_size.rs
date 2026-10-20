//! Rows' sizes: DECDWL, DECDHL and DECSWL as the grid keeps them.
//!
//! A row's [`LineSize`] is two bits of its record (`row.rs`), so it moves with the row: scrolling
//! rotates records, the copies that move a row across pages take it along (`Meta`), and the history
//! keeps it. A row that is blanked whole (a new line entering, ED over a row, a reset) loses it;
//! erasing cells inside a row (EL, ECH) does not. A resize never reflows such a row: it keeps its
//! cells, cropped or padded to the new width (`rewrap.rs`).
//!
//! The terminal limits where the cursor and the text can go on such a row to the first half of the
//! columns (`LineSize::visible_columns`), which it asks for on every write. So that a screen
//! that never used the feature pays one test of a flag for that, not a row lookup, the grid
//! remembers whether a live row may have a size ([`Grid::has_line_sizes`]): set when one is given,
//! recounted after a resize and when the whole screen is blanked.

use crate::types::{Line, LineSize};

use super::Grid;

impl Grid {
    /// The size of the row at `line` (history included); `Single` for a line the grid does not have.
    pub fn line_size(&self, line: Line) -> LineSize {
        self.locate_line(line).map_or(LineSize::Single, |(page, row)| self.pages[page].line_size(row))
    }

    /// The size of live row `y`. Looks no row up while no row of the screen has a size.
    #[inline]
    pub fn live_line_size(&self, y: usize) -> LineSize {
        if !self.sized || y >= self.rows {
            return LineSize::Single;
        }
        let (page, row) = self.locate_y(y);
        self.pages[page].line_size(row)
    }

    /// [`Self::live_line_size`] without the hint, for the checker that tests the hint.
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn live_line_size_unchecked(&self, y: usize) -> LineSize {
        let (page, row) = self.locate_y(y);
        self.pages[page].line_size(row)
    }

    /// Whether a live row may have a size other than `Single`: false means none has, true that one
    /// might (it is not recounted when the lines that had one scroll into the history).
    #[inline]
    pub fn has_line_sizes(&self) -> bool {
        self.sized
    }

    /// Make live row `y` this size (DECSWL, DECDWL, DECDHL); the row is damaged when it changes.
    /// The cells stay: those past `LineSize::visible_columns` are hidden, not lost.
    pub fn set_line_size(&mut self, y: usize, size: LineSize) {
        if y >= self.rows || (size == LineSize::Single && !self.sized) {
            return;
        }
        self.sized |= size.is_double();
        let (page, row) = self.page_row_mut(y);
        page.set_line_size(row, size);
    }

    /// Recount whether a live row has a size: after the rows were rebuilt or the screen blanked.
    pub(super) fn recount_line_sizes(&mut self) {
        if !self.sized {
            return;
        }
        self.sized = (0..self.rows).any(|y| {
            let (page, row) = self.locate_y(y);
            self.pages[page].line_size(row).is_double()
        });
    }
}
