//! Moving the cursor: CUP and its relatives, the controls that move it, tabs, DECSC and DECRC.
//!
//! Every move clears the delayed wrap (xterm's `xenl`; see the module docs of `terminal`). Rows are
//! limited by the scrolling region and columns by the left and right margins, but only when the
//! cursor starts inside them: a cursor outside a margin may move to the edge of the screen
//! (VT510, CUU/CUD/CUF/CUB: "the cursor stops at the margin" when it is inside).
//!
//! DECSC and DECRC carry the hyperlink (OSC 8) with the rendition: the OSC 8 text calls the link
//! "a cell attribute like color", and the rendition is what DECSC exists to save.

use crate::grid::Style;
use crate::types::{CursorShape, Hyperlink, Line, Point};

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;
use crate::terminal::text::charset::Charsets;

/// The cursor's place. `pending` is the delayed wrap: the last character filled the row, the next
/// one wraps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::terminal) struct CursorPos {
    pub(in crate::terminal) x: usize,
    pub(in crate::terminal) y: usize,
    pub(in crate::terminal) pending: bool,
}

/// What DECSC saves (and DECRC brings back): VT510 DECSC lists the position, the rendition, the
/// character sets, the origin mode, the selective erase attribute and the wrap flag.
#[derive(Clone, Debug)]
pub(in crate::terminal) struct Saved {
    pub(in crate::terminal) cursor: CursorPos,
    pub(in crate::terminal) style: Style,
    pub(in crate::terminal) protected: bool,
    pub(in crate::terminal) origin: bool,
    pub(in crate::terminal) charsets: Charsets,
    pub(in crate::terminal) link: Option<Hyperlink>,
}

impl Default for Saved {
    /// The state a terminal starts with: home, normal rendition, ASCII.
    fn default() -> Saved {
        Saved {
            cursor: CursorPos::default(),
            style: Style::DEFAULT,
            protected: false,
            origin: false,
            charsets: Charsets::default(),
            link: None,
        }
    }
}

/// The cursor's look as DECSCUSR and mode 12 asked for it; `None` leaves each to the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::terminal) struct CursorStyle {
    pub(in crate::terminal) shape: Option<CursorShape>,
    pub(in crate::terminal) blink: Option<bool>,
}

/// Where a vertical move lands in its row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum Landing {
    /// In the column it started in (CUU, CUD, VPR).
    Stay,
    /// At the start of the line (CPL, CNL).
    LineStart,
}

impl Terminal {
    /// The last column going right can reach from here: the right margin, or the screen's edge
    /// for a cursor that is already beyond it; on a double-width or double-height row (see
    /// `line_size`) no further than the half of the columns the row shows.
    #[inline]
    pub(in crate::terminal) fn right_limit(&self) -> usize {
        self.right_edge(self.cursor.x, self.cursor.y)
    }

    /// The first column going left can reach from here.
    #[inline]
    pub(in crate::terminal) fn left_limit(&self) -> usize {
        if self.cursor.x >= self.margins.left { self.margins.left } else { 0 }
    }

    /// Bring the cursor in line with the row under it and the margins in force, after something
    /// changed either without moving it: a soft reset that put the margins back, a size given to
    /// the row, a scroll that put another row under the cursor, a swap of the screens (the cursor
    /// is shared, the rows are not), a picture that moved it. The column is cut to what the row
    /// shows, and a delayed wrap that waits at a column that is no longer where this row wraps is
    /// dropped (the cursor stays where it is: nothing is due there). Every change of the screen
    /// that can leave the cursor out of step ends here, so that the checked invariants (a wrap is
    /// only pending on the edge, the cursor is in a shown column) hold after every sequence.
    #[inline]
    pub(in crate::terminal) fn normalize_cursor(&mut self) {
        self.cursor.x = self.cursor.x.min(self.row_right(self.cursor.y));
        self.cursor.pending &= self.cursor.x == self.right_limit();
    }

    /// Put the cursor at `x`, `y` (inside the screen) and end a pending wrap.
    #[inline]
    pub(in crate::terminal) fn place(&mut self, x: usize, y: usize) {
        self.cursor.x = if self.grid.has_line_sizes() { x.min(self.row_right(y)) } else { x };
        self.cursor.y = y;
        self.cursor.pending = false;
    }

    /// CUP/HVP: `row` and `col` count from 0, from the region's corner in origin mode.
    pub(in crate::terminal) fn position(&mut self, row: usize, col: usize) {
        let (x, y) = if self.modes.contains(Modes::ORIGIN) {
            (
                (self.margins.left + col).min(self.margins.right),
                (self.margins.top + row).min(self.margins.bottom),
            )
        } else {
            (col.min(self.cols - 1), row.min(self.rows - 1))
        };
        self.place(x, y);
    }

    /// The home position: the screen's corner, or the region's in origin mode.
    pub(in crate::terminal) fn home(&mut self) {
        self.position(0, 0);
    }

    /// CHA/HPA: the column only.
    pub(in crate::terminal) fn set_column(&mut self, col: usize) {
        let x = if self.modes.contains(Modes::ORIGIN) {
            (self.margins.left + col).min(self.margins.right)
        } else {
            col.min(self.cols - 1)
        };
        self.place(x, self.cursor.y);
    }

    /// VPA: the row only.
    pub(in crate::terminal) fn set_row(&mut self, row: usize) {
        let y = if self.modes.contains(Modes::ORIGIN) {
            (self.margins.top + row).min(self.margins.bottom)
        } else {
            row.min(self.rows - 1)
        };
        self.place(self.cursor.x, y);
    }

    /// CUU, and CPL.
    pub(in crate::terminal) fn cursor_up(&mut self, n: usize, landing: Landing) {
        let floor = if self.cursor.y >= self.margins.top { self.margins.top } else { 0 };
        let y = self.cursor.y.saturating_sub(n).max(floor);
        let x = if landing == Landing::LineStart { self.carriage_column() } else { self.cursor.x };
        self.place(x, y);
    }

    /// CUD, and CNL.
    pub(in crate::terminal) fn cursor_down(&mut self, n: usize, landing: Landing) {
        let ceiling = if self.cursor.y <= self.margins.bottom { self.margins.bottom } else { self.rows - 1 };
        let y = self.cursor.y.saturating_add(n).min(ceiling);
        let x = if landing == Landing::LineStart { self.carriage_column() } else { self.cursor.x };
        self.place(x, y);
    }

    /// CUF, HPR.
    pub(in crate::terminal) fn cursor_forward(&mut self, n: usize) {
        let x = self.cursor.x.saturating_add(n).min(self.right_limit());
        self.place(x, self.cursor.y);
    }

    /// CUB: `n` columns left, stopping at the left margin. With reverse wraparound each column is
    /// a [`backspace`](Self::backspace), so the cursor may go up a line.
    pub(in crate::terminal) fn cursor_back(&mut self, n: usize) {
        if self.reverse_wraps() {
            // More steps than the screen has cells only go round again. The steps along a row are
            // taken at once and a step that changes nothing ends the walk, so a program repeating
            // `CSI 65535 D` costs a few steps per row, not one per cell.
            let mut steps = n.min(self.cols * self.rows + 1);
            while steps > 0 {
                let floor = self.left_limit();
                if !self.cursor.pending && self.cursor.x > floor {
                    let run = steps.min(self.cursor.x - floor);
                    self.place(self.cursor.x - run, self.cursor.y);
                    steps -= run;
                    continue;
                }
                let before = self.cursor;
                self.backspace_step();
                if self.cursor == before {
                    break;
                }
                steps -= 1;
            }
            return;
        }
        let x = self.cursor.x.saturating_sub(n).max(self.left_limit());
        self.place(x, self.cursor.y);
    }

    /// The column CR goes to: the left margin, or the screen's edge for a cursor left of it.
    fn carriage_column(&self) -> usize {
        self.left_limit()
    }

    /// CR.
    pub(in crate::terminal) fn carriage_return(&mut self) {
        let x = self.carriage_column();
        self.place(x, self.cursor.y);
    }

    /// Whether BS and CUB wrap back to the line above (mode 45 or 1045, with autowrap).
    fn reverse_wraps(&self) -> bool {
        self.modes.contains(Modes::AUTOWRAP)
            && self.modes.intersects(Modes::REVERSE_WRAP | Modes::REVERSE_WRAP_EXTENDED)
    }

    /// BS.
    pub(in crate::terminal) fn backspace(&mut self) {
        self.backspace_step();
    }

    /// One column left, stopping at the left margin. With reverse wraparound (xterm's ctlseqs,
    /// modes 45 and 1045; both need autowrap) at the left edge the cursor goes to the right
    /// margin of the line above, if that line wrapped into this one (45), or whatever it is
    /// (1045); at the top margin 1045 goes round to the bottom margin. A cursor waiting to wrap
    /// is taken to be one column beyond the last, so the step lands on the last column.
    fn backspace_step(&mut self) {
        let reverse = self.reverse_wraps();
        if reverse && self.cursor.pending {
            self.cursor.pending = false;
            return;
        }
        let floor = self.left_limit();
        if self.cursor.x > floor {
            self.place(self.cursor.x - 1, self.cursor.y);
            return;
        }
        let extended = self.modes.contains(Modes::REVERSE_WRAP_EXTENDED);
        let row = if !reverse {
            None
        } else if self.cursor.y == self.margins.top || self.cursor.y == 0 {
            // Above the region too: the top of the screen has no line before it.
            extended.then_some(if self.cursor.y == 0 && self.margins.top > 0 {
                self.rows - 1
            } else {
                self.margins.bottom
            })
        } else {
            let above = self.cursor.y - 1;
            (extended || self.grid.is_wrapped(Line(above as i32))).then_some(above)
        };
        match row {
            Some(y) => {
                let x = if self.cursor.x <= self.margins.right { self.margins.right } else { self.cols - 1 };
                self.place(x, y);
            }
            None => self.cursor.pending = false,
        }
    }

    /// HT, CHT: forward `n` tab stops; without more stops the right margin (or the last column).
    /// HT never wraps (VT510, HT): at the last column it goes nowhere and the wrap that was due
    /// still is, unless mode 41 asks for the wrap first.
    pub(in crate::terminal) fn tab_forward(&mut self, n: usize) {
        if self.cursor.pending && self.modes.contains(Modes::MORE_FIX | Modes::AUTOWRAP) {
            self.wrap();
        }
        let limit = self.right_limit();
        let mut x = self.cursor.x;
        for _ in 0..n {
            let next = self.tabs.next(x, limit);
            if next == x {
                break;
            }
            // Each stop passed leaves its mark, so a copy gives one tab per hop.
            self.grid.mark_tab(self.cursor.y, x);
            x = next;
        }
        // A tab that goes nowhere is not a move: the wrap that was due still is.
        if x != self.cursor.x {
            self.place(x, self.cursor.y);
        }
    }

    /// CBT: back `n` tab stops. Unlike HT it ignores the left margin and goes on to the screen's
    /// edge (esctest2 `CBT_IgnoresRegion`; xterm's ctlseqs gives no margin to stop at).
    pub(in crate::terminal) fn tab_back(&mut self, n: usize) {
        let floor = 0;
        let mut x = self.cursor.x;
        for _ in 0..n {
            let prev = self.tabs.prev(x, floor);
            if prev == x {
                break;
            }
            x = prev;
        }
        self.place(x, self.cursor.y);
    }

    /// DECSC: the cursor, its rendition, character sets, origin mode and protection, for the
    /// screen that shows.
    pub(in crate::terminal) fn save_cursor(&mut self) {
        let saved = &mut self.saved[self.screen_slot()];
        saved.cursor = self.cursor;
        saved.style = self.pen.style;
        saved.protected = self.pen.protected;
        saved.origin = self.modes.contains(Modes::ORIGIN);
        saved.charsets = self.charsets;
        saved.link = self.pen.link.clone();
    }

    /// DECRC: bring back what DECSC saved (home and normal rendition when nothing was saved).
    pub(in crate::terminal) fn restore_cursor(&mut self) {
        let saved = self.saved[self.screen_slot()].clone();
        self.cursor = saved.cursor;
        self.cursor.x = self.cursor.x.min(self.cols - 1);
        self.cursor.y = self.cursor.y.min(self.rows - 1);
        // Saved on the last column of a screen that has been widened since: nothing is due there.
        self.normalize_cursor();
        self.pen.style = saved.style;
        self.pen.protected = saved.protected;
        self.modes.set(Modes::ORIGIN, saved.origin);
        self.charsets = saved.charsets;
        self.pen.link = saved.link;
    }

    /// Where the cursor is, shown or not: the cell it is waiting on, with a wrap pending. In grid
    /// coordinates, the live screen's top row being line 0 whatever the view shows. Unlike
    /// [`Terminal::cursor`]'s point it is not moved back onto the first half of a wide character.
    pub fn cursor_point(&self) -> Point {
        Point::new(Line(self.cursor.y as i32), crate::types::Column(self.cursor.x))
    }
}
