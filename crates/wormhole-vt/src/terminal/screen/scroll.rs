//! Line feeds, scrolling and the margins that bound it.

use crate::grid::Region;
use crate::terminal::Terminal;
use crate::terminal::modes::Modes;

/// The scrolling region (DECSTBM) and the left and right margins (DECSLRM), always inside the
/// screen and covering it whole until a program says otherwise. Edges included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) struct Margins {
    pub(in crate::terminal) top: usize,
    pub(in crate::terminal) bottom: usize,
    pub(in crate::terminal) left: usize,
    pub(in crate::terminal) right: usize,
}

impl Margins {
    pub(in crate::terminal) fn full(cols: usize, rows: usize) -> Margins {
        Margins { top: 0, bottom: rows - 1, left: 0, right: cols - 1 }
    }
}

impl Terminal {
    /// The scrolling region as the grid wants it.
    #[inline]
    pub(in crate::terminal) fn region(&self) -> Region {
        Region {
            top: self.margins.top,
            bottom: self.margins.bottom,
            left: self.margins.left,
            right: self.margins.right,
        }
    }

    /// The color erased and scrolled-in cells get: the pen's background.
    #[inline]
    pub(in crate::terminal) fn erase_color(&self) -> crate::types::Color {
        self.pen.style.bg
    }

    /// Scroll the region up by `n` lines: blank lines (in the pen's background) enter at the
    /// bottom margin, and the lines that leave the top go into the history when the region is the
    /// whole width from the first row (the grid decides).
    pub(in crate::terminal) fn scroll_up(&mut self, n: usize) {
        self.scroll_region_up(n);
        self.fit_cursor();
    }

    /// [`Self::scroll_up`] for the callers that bring the cursor inside its row themselves.
    #[inline]
    pub(in crate::terminal) fn scroll_region_up(&mut self, n: usize) {
        let region = self.region();
        let bg = self.erase_color();
        self.grid.scroll_up(region, n, bg);
    }

    /// Scroll the region down by `n` lines.
    pub(in crate::terminal) fn scroll_down(&mut self, n: usize) {
        let region = self.region();
        let bg = self.erase_color();
        self.grid.scroll_down(region, n, bg);
        self.fit_cursor();
    }

    /// IND, and LF, VT and FF: down one line, scrolling at the bottom margin. The column stays.
    pub(in crate::terminal) fn index(&mut self) {
        if self.cursor.y == self.margins.bottom {
            // The region scrolls only for a cursor inside its columns; outside them the cursor
            // would have to leave the region to go on, and does not (VT510, DECLRMM).
            if self.inside_columns() {
                self.scroll_up(1);
            }
        } else if self.cursor.y + 1 < self.rows {
            self.cursor.y += 1;
        }
        self.cursor.pending = false;
        self.fit_cursor();
    }

    /// LF, VT and FF: IND, and with LNM also a carriage return (VT510, LNM).
    pub(in crate::terminal) fn line_feed(&mut self) {
        self.index();
        if self.modes.contains(Modes::NEW_LINE) {
            self.carriage_return();
        }
    }

    /// NEL.
    pub(in crate::terminal) fn next_line(&mut self) {
        self.index();
        self.carriage_return();
    }

    /// RI: up one line, scrolling the region down at the top margin.
    pub(in crate::terminal) fn reverse_index(&mut self) {
        if self.cursor.y == self.margins.top {
            if self.inside_columns() {
                self.scroll_down(1);
            }
        } else if self.cursor.y > 0 {
            self.cursor.y -= 1;
        }
        self.cursor.pending = false;
        self.fit_cursor();
    }

    /// DECSTBM. `top` and `bottom` count from 1, 0 meaning the default. Not valid (nothing
    /// changes, the cursor stays) unless the region has at least two lines. The cursor goes home.
    pub(in crate::terminal) fn set_vertical_margins(&mut self, top: usize, bottom: usize) {
        let top = top.max(1) - 1;
        let bottom = (if bottom == 0 { self.rows } else { bottom.min(self.rows) }) - 1;
        if top >= bottom {
            return;
        }
        self.margins.top = top;
        self.margins.bottom = bottom;
        self.home();
    }

    /// DECSLRM, which only exists while DECLRMM is set. Same rules as DECSTBM, on columns.
    pub(in crate::terminal) fn set_horizontal_margins(&mut self, left: usize, right: usize) {
        if !self.modes.contains(Modes::LEFT_RIGHT_MARGINS) {
            return;
        }
        let left = left.max(1) - 1;
        let right = (if right == 0 { self.cols } else { right.min(self.cols) }) - 1;
        if left >= right {
            return;
        }
        self.margins.left = left;
        self.margins.right = right;
        self.home();
    }

    /// DECFI: forward one column; at the right margin the columns inside the margins move left
    /// (VT510, DECFI).
    pub(in crate::terminal) fn forward_index(&mut self) {
        let Terminal { cursor, margins, .. } = *self;
        let right = margins.right.min(self.row_right(cursor.y));
        if cursor.x == right && cursor.x >= margins.left {
            let bg = self.erase_color();
            for y in margins.top..=margins.bottom {
                self.grid.delete_cells(y, margins.left, margins.right, 1, bg);
            }
        } else if cursor.x < self.row_right(cursor.y) {
            self.cursor.x += 1;
        }
        self.cursor.pending = false;
    }

    /// DECBI: back one column; at the left margin the columns inside the margins move right.
    pub(in crate::terminal) fn back_index(&mut self) {
        let Terminal { cursor, margins, .. } = *self;
        if cursor.x == margins.left && cursor.x <= margins.right {
            let bg = self.erase_color();
            for y in margins.top..=margins.bottom {
                self.grid.insert_cells(y, margins.left, margins.right, 1, bg);
            }
        } else if cursor.x > 0 {
            self.cursor.x -= 1;
        }
        self.cursor.pending = false;
    }
}
