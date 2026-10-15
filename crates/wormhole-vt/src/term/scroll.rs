//! Line feeds, scrolling and the margins that bound it.

use super::Terminal;
use super::modes::Modes;

impl Terminal {
    /// Scroll the region up by `n` lines: blank lines (in the pen's background) enter at the
    /// bottom margin, and the lines that leave the top go into the history when the region is the
    /// whole width from the first row (the grid decides).
    pub(super) fn scroll_up(&mut self, n: usize) {
        let region = self.region();
        let bg = self.erase_color();
        self.grid.scroll_up(region, n, bg);
    }

    /// Scroll the region down by `n` lines.
    pub(super) fn scroll_down(&mut self, n: usize) {
        let region = self.region();
        let bg = self.erase_color();
        self.grid.scroll_down(region, n, bg);
    }

    /// IND, and LF, VT and FF: down one line, scrolling at the bottom margin. The column stays.
    pub(super) fn index(&mut self) {
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
    }

    /// LF, VT and FF: IND, and with LNM also a carriage return (VT510, LNM).
    pub(super) fn line_feed(&mut self) {
        self.index();
        if self.modes.contains(Modes::NEW_LINE) {
            self.carriage_return();
        }
    }

    /// NEL.
    pub(super) fn next_line(&mut self) {
        self.index();
        self.carriage_return();
    }

    /// RI: up one line, scrolling the region down at the top margin.
    pub(super) fn reverse_index(&mut self) {
        if self.cursor.y == self.margins.top {
            if self.inside_columns() {
                self.scroll_down(1);
            }
        } else if self.cursor.y > 0 {
            self.cursor.y -= 1;
        }
        self.cursor.pending = false;
    }

    /// DECSTBM. `top` and `bottom` count from 1, 0 meaning the default. Not valid (nothing
    /// changes, the cursor stays) unless the region has at least two lines. The cursor goes home.
    pub(super) fn set_vertical_margins(&mut self, top: usize, bottom: usize) {
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
    pub(super) fn set_horizontal_margins(&mut self, left: usize, right: usize) {
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
    pub(super) fn forward_index(&mut self) {
        let Terminal { cursor, margins, .. } = *self;
        if cursor.x == margins.right && cursor.x >= margins.left {
            let bg = self.erase_color();
            for y in margins.top..=margins.bottom {
                self.grid.delete_cells(y, margins.left, margins.right, 1, bg);
            }
        } else if cursor.x + 1 < self.cols {
            self.cursor.x += 1;
        }
        self.cursor.pending = false;
    }

    /// DECBI: back one column; at the left margin the columns inside the margins move right.
    pub(super) fn back_index(&mut self) {
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
