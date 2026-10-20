//! The window changing size, and DECCOLM asking for it.
//!
//! The main screen is reflowed (the grid rewraps soft-wrapped lines and carries the cursor), the
//! alternate screen is cropped or padded: programs redraw it. A resize ends any scrolling region
//! and margins (the screen they were set on is gone), and gives new columns the default tab
//! stops. Spec for DECCOLM: xterm's ctlseqs, `CSI ? 3 h/l`: set 132 / 80 columns, clear the
//! screen, reset the margins; allowed only with mode 40 set; mode 95 (DECNCSM) keeps the screen.
//! The width itself changes only with `Policy::window_ops` (see `set_column_mode`).

use crate::types::{Color, Column, Line, NamedColor, Point};

use crate::terminal::modes::conformance::Level;
use crate::terminal::modes::{Modes, Switch};
use crate::terminal::screen::tabs::TabStops;
use crate::terminal::{Event, Margins, Terminal};

impl Terminal {
    /// The window is now `cols` by `rows`.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let (cols, rows) =
            (cols.clamp(crate::grid::MIN_COLS, crate::grid::MAX_COLS), rows.clamp(1, crate::grid::MAX_ROWS));
        if (cols, rows) == (self.cols, self.rows) {
            // The cell may have changed (`set_cell_size`), which in-band reports tell too.
            self.report_inband_change();
            return;
        }
        let was_pending = self.cursor.pending;
        let alt = self.on_alt_screen();
        let at = self.cursor_point();
        // The screen not showing belongs to the other half of the cursor state: the main screen
        // keeps the cursor DECSC saved (1049 brings it back).
        let other_slot = usize::from(!alt);
        let other_at =
            Point::new(Line(self.saved[other_slot].cursor.y as i32), Column(self.saved[other_slot].cursor.x));
        let moved = self.grid.resize(cols, rows, at, !alt);
        let other_moved = self.other.resize(cols, rows, other_at, alt);
        self.cols = self.grid.cols();
        self.rows = self.grid.rows();
        self.cursor.x = moved.column.0;
        self.cursor.y = moved.line.0.max(0) as usize;
        self.cursor.pending = false;
        if was_pending {
            // The cursor was waiting after the last character; it still is.
            if self.cursor.x + 1 < self.cols {
                self.cursor.x += 1;
            } else {
                self.cursor.pending = true;
            }
        }
        self.fit_cursor();
        let saved = &mut self.saved[other_slot].cursor;
        saved.x = other_moved.column.0;
        saved.y = other_moved.line.0.max(0) as usize;
        let current = &mut self.saved[usize::from(alt)].cursor;
        current.x = current.x.min(self.cols - 1);
        current.y = current.y.min(self.rows - 1);
        self.margins = Margins::full(self.cols, self.rows);
        self.tabs.resize(self.cols);
        // The rows are rearranged: the character REP repeats may no longer stand where it did.
        self.end_text();
        self.apply_history_limit();
        self.report_inband_change();
    }

    /// Mode 3 (DECCOLM) set or reset.
    ///
    /// DECCOLM's other effects (the screen cleared unless DECNCSM keeps it, the margins and tab
    /// stops back to the page, the cursor home; the VT510 manual lists them) always happen. The
    /// width changes only where the host lets a program change its window
    /// ([`Policy::window_ops`](crate::Policy::window_ops), the gate DECSCPP and DECSLPP are behind):
    /// a resize reflows the whole history, so a few kilobytes of `?3h ?3l` toggles cost seconds, and
    /// the host then had to resize the terminal back. Without the policy the mode is still recorded
    /// (DECRQM tells it) and the width stays what the window has.
    pub(in crate::terminal) fn set_column_mode(&mut self, switch: Switch) {
        let wide = switch.is_set();
        if !self.modes.contains(Modes::ALLOW_COLUMNS) {
            return;
        }
        self.modes.set(Modes::COLUMNS_132, wide);
        let cols = if wide { 132 } else { 80 };
        // DECNCSM is a VT500 feature.
        if !(self.modes.contains(Modes::NO_CLEAR_ON_COLUMNS) && self.level >= Level::Vt500) {
            self.grid.erase_rows(0..self.rows, Color::Named(NamedColor::Background), false);
        }
        let resized = self.policy.window_ops && cols != self.cols;
        if resized {
            self.resize(cols, self.rows);
        }
        self.margins = Margins::full(self.cols, self.rows);
        self.modes.remove(Modes::LEFT_RIGHT_MARGINS);
        self.tabs = TabStops::new(self.cols);
        self.home();
        if resized {
            self.push_event(Event::ColumnsRequested(cols));
        }
    }
}
