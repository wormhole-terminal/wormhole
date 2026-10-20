//! Double-width and double-height lines: DECSWL, DECDWL and DECDHL (`ESC # 5`, `# 6`, `# 3`, `# 4`).
//!
//! Spec: the VT510 manual's line attributes and xterm's ctlseqs ("ESC # 3" to "ESC # 6"). The
//! attribute belongs to the row the cursor is on (the grid keeps it with the row, `grid/line_size.rs`).
//! On a double-width or double-height row every character takes two columns, so only the first half
//! of the screen's columns can be reached (`LineSize::visible_columns`): the cursor moves, tabs
//! and the right margin stop there, and a character that would pass it wraps (or, with autowrap
//! off, overwrites the last column) like at the edge of a normal row. DECDHL is two rows: the
//! program sets `ESC # 3` on one and `ESC # 4` on the row below it, and writes the same text on both.
//!
//! Choices:
//!
//! - Setting a size changes nothing in the row's cells: the ones past the half are hidden, not
//!   erased, and show again if the row becomes single (the VT510 manual leaves the characters'
//!   fate open, and keeping them loses nothing). A cursor past the new half moves to its last column,
//!   and a pending wrap ends.
//! - Erasing cells inside a row (EL, ECH) leaves its size; blanking a row whole (ED over it, a
//!   line scrolled or inserted in, IL and DL's new lines, RIS, DECALN) makes it single. ED 2 resets
//!   every row; EL 2 of a row does not.
//! - A double-width row has no soft wrap: text that runs past its half goes on in the next row,
//!   and the two are two lines (the cells the row hides would otherwise come between them when a
//!   copy joins them, and reflow cannot rejoin them). Giving a wrapped row a size ends its wrap.
//! - A resize keeps the row's cells and size and never reflows it (`grid/rewrap.rs`).

use crate::types::LineSize;

use crate::terminal::Terminal;

impl Terminal {
    /// `ESC # 3`, `4`, `5` and `6`: the cursor's row gets `size`.
    pub(in crate::terminal) fn set_line_size(&mut self, size: LineSize) {
        self.grid.set_line_size(self.cursor.y, size);
        self.cursor.pending = false;
        self.fit_cursor();
    }

    /// The last column of live row `y` the cursor can be in: the last of the screen, or of the
    /// half a double-width row shows. A test of one flag while no row has a size.
    #[inline]
    pub(in crate::terminal) fn row_right(&self, y: usize) -> usize {
        if !self.grid.has_line_sizes() {
            return self.cols - 1;
        }
        self.grid.live_line_size(y).visible_columns(self.cols) - 1
    }

    /// Where going right from column `x` of row `y` stops: the right margin, or the row's last
    /// column for a cursor beyond the margin.
    #[inline]
    pub(in crate::terminal) fn right_edge(&self, x: usize, y: usize) -> usize {
        let edge = if x <= self.margins.right { self.margins.right } else { self.cols - 1 };
        edge.min(self.row_right(y))
    }

    /// Bring the cursor inside its row: after a move to another row, or a scroll that put another
    /// row under it, which may show fewer columns.
    #[inline]
    pub(in crate::terminal) fn fit_cursor(&mut self) {
        if self.grid.has_line_sizes() {
            self.cursor.x = self.cursor.x.min(self.row_right(self.cursor.y));
        }
    }
}
