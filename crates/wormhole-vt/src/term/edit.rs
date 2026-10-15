//! Erasing and editing: ED, EL, ECH and their selective forms, ICH, DCH, IL, DL, DECIC, DECDC.
//!
//! Erased cells take the pen's background and nothing else of its rendition (`bce`). The erasers
//! work on whole lines whatever the margins say (ECMA-48 and the VT510 manual limit the
//! *editing* functions, not the erasers); ICH, DCH, IL and DL, DECIC and DECDC have no effect
//! with the cursor outside the margins (VT510, "no effect outside the page margins").
//!
//! None of them moves the cursor. All of them end a pending wrap: the line under the cursor
//! changed, and the character that waited to wrap would otherwise leave a hole where an erased
//! last column was (xterm: "with the right part cleared, we can't wrap").

use crate::grid::Spare;

use super::Terminal;
use super::modes::Modes;

/// Which part of the screen or the line an erase covers (the parameter of ED and EL).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Extent {
    /// 0: from the cursor to the end.
    ToEnd,
    /// 1: from the start up to and including the cursor.
    ToCursor,
    /// 2: all of it.
    All,
    /// ED 3: the saved lines.
    Scrollback,
}

impl Extent {
    pub(super) fn from_param(n: u16) -> Option<Extent> {
        match n {
            0 => Some(Extent::ToEnd),
            1 => Some(Extent::ToCursor),
            2 => Some(Extent::All),
            3 => Some(Extent::Scrollback),
            _ => None,
        }
    }
}

/// Which cells an erase leaves alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Erase {
    /// ED, EL: all but the cells in a guarded area (SPA to EPA, ECMA-48 8.3.129).
    Everything,
    /// DECSED, DECSEL: all but the cells DECSCA protected.
    Unprotected,
}

/// Which way SL or SR moves the columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shift {
    Left,
    Right,
}

impl Terminal {
    /// What an erase of this kind leaves alone. A guarded area costs the erasers a look at every
    /// cell, so they only look once a program has started one.
    fn spare(&self, kind: Erase) -> Spare {
        match kind {
            Erase::Everything if self.guard_used => Spare::Guarded,
            Erase::Everything => Spare::Nothing,
            Erase::Unprotected => Spare::Selective,
        }
    }
}

impl Terminal {
    /// ED and DECSED (`CSI Ps J`, `CSI ? Ps J`).
    pub(super) fn erase_display(&mut self, extent: Extent, kind: Erase) {
        self.cursor.pending = false;
        let spare = self.spare(kind);
        let bg = self.erase_color();
        let (x, y) = (self.cursor.x, self.cursor.y);
        match extent {
            Extent::ToEnd => {
                self.grid.erase(y, x..self.cols, bg, spare);
                self.grid.erase_rows(y + 1..self.rows, bg, spare);
            }
            Extent::ToCursor => {
                self.grid.erase_rows(0..y, bg, spare);
                self.grid.erase(y, 0..x + 1, bg, spare);
            }
            Extent::All => self.clear_screen(bg, kind),
            Extent::Scrollback => {
                self.grid.clear_scrollback();
                self.history_just_cleared = true;
            }
        }
    }

    /// ED 2. On the main screen the lines that hold something go into the history first, so that
    /// `clear` or ^L leaves them to scroll back to (as the app has always behaved); the
    /// alternate screen has no history. An ED 3 just before (macOS `clear` sends `ESC[3J ESC[H
    /// ESC[2J`) means the user wants them gone: nothing is saved then.
    fn clear_screen(&mut self, bg: crate::types::Color, kind: Erase) {
        let spare = self.spare(kind);
        let keep = kind == Erase::Everything
            && !self.modes.contains(Modes::ALT_SCREEN)
            && !self.history_just_cleared;
        if keep {
            self.grid.scroll_screen_into_history(bg);
        }
        self.grid.erase_rows(0..self.rows, bg, spare);
    }

    /// EL and DECSEL (`CSI Ps K`, `CSI ? Ps K`).
    pub(super) fn erase_line(&mut self, extent: Extent, kind: Erase) {
        self.cursor.pending = false;
        let spare = self.spare(kind);
        let bg = self.erase_color();
        let (x, y) = (self.cursor.x, self.cursor.y);
        match extent {
            Extent::ToEnd => self.grid.erase(y, x..self.cols, bg, spare),
            Extent::ToCursor => self.grid.erase(y, 0..x + 1, bg, spare),
            Extent::All => self.grid.erase(y, 0..self.cols, bg, spare),
            Extent::Scrollback => {}
        }
    }

    /// ECH: erase `n` cells from the cursor, without moving anything.
    pub(super) fn erase_chars(&mut self, n: usize) {
        self.cursor.pending = false;
        let bg = self.erase_color();
        let end = self.cursor.x.saturating_add(n).min(self.cols);
        let spare = self.spare(Erase::Everything);
        self.grid.erase(self.cursor.y, self.cursor.x..end, bg, spare);
    }

    /// Whether the cursor is inside the left and right margins.
    pub(super) fn inside_columns(&self) -> bool {
        (self.margins.left..=self.margins.right).contains(&self.cursor.x)
    }

    /// ICH.
    pub(super) fn insert_chars(&mut self, n: usize) {
        self.cursor.pending = false;
        if self.inside_columns() {
            let bg = self.erase_color();
            self.grid.insert_cells(self.cursor.y, self.cursor.x, self.margins.right, n, bg);
        }
    }

    /// DCH.
    pub(super) fn delete_chars(&mut self, n: usize) {
        self.cursor.pending = false;
        if self.inside_columns() {
            let bg = self.erase_color();
            self.grid.delete_cells(self.cursor.y, self.cursor.x, self.margins.right, n, bg);
        }
    }

    /// IL.
    pub(super) fn insert_lines(&mut self, n: usize) {
        self.cursor.pending = false;
        if self.inside_columns() {
            let (y, region, bg) = (self.cursor.y, self.region(), self.erase_color());
            self.grid.insert_lines(y, region, n, bg);
        }
    }

    /// DL.
    pub(super) fn delete_lines(&mut self, n: usize) {
        self.cursor.pending = false;
        if self.inside_columns() {
            let (y, region, bg) = (self.cursor.y, self.region(), self.erase_color());
            self.grid.delete_lines(y, region, n, bg);
        }
    }

    /// SL and SR (ECMA-48 8.3.121, 8.3.135): the columns of the scrolling region and between the
    /// margins move `n` places, the ones that leave are lost and blanks come in at the other side.
    /// The cursor stays where it is.
    pub(super) fn shift_columns(&mut self, direction: Shift, n: usize) {
        let bg = self.erase_color();
        for y in self.margins.top..=self.margins.bottom {
            match direction {
                Shift::Left => self.grid.delete_cells(y, self.margins.left, self.margins.right, n, bg),
                Shift::Right => self.grid.insert_cells(y, self.margins.left, self.margins.right, n, bg),
            }
        }
    }

    /// DECIC: insert `n` blank columns at the cursor in every line of the region.
    pub(super) fn insert_columns(&mut self, n: usize) {
        self.cursor.pending = false;
        if self.inside_columns() && (self.margins.top..=self.margins.bottom).contains(&self.cursor.y) {
            let bg = self.erase_color();
            for y in self.margins.top..=self.margins.bottom {
                self.grid.insert_cells(y, self.cursor.x, self.margins.right, n, bg);
            }
        }
    }

    /// DECDC.
    pub(super) fn delete_columns(&mut self, n: usize) {
        self.cursor.pending = false;
        if self.inside_columns() && (self.margins.top..=self.margins.bottom).contains(&self.cursor.y) {
            let bg = self.erase_color();
            for y in self.margins.top..=self.margins.bottom {
                self.grid.delete_cells(y, self.cursor.x, self.margins.right, n, bg);
            }
        }
    }
}
