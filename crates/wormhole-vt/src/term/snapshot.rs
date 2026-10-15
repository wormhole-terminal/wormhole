//! What the host draws and asks from: the same shape as `wormhole-core`'s `Screen` trait, so
//! implementing it for [`Terminal`] is forwarding.
//!
//! The host locks the terminal, copies what changed ([`Terminal::copy_viewport`] copies only the
//! rows marked dirty since the last call, into buffers the host keeps) and lets go; nothing here
//! allocates once the buffers are the right size.

use crate::grid::{Grid, PinId, RowMark, ViewRow};
use crate::types::{Cell, Column, CursorShape, ImagePlacement, Line, Point, RowExtras, Scroll, TermMode};

use super::Terminal;
use super::modes::Modes;

/// Where the cursor is and how a program wants it drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalCursor {
    /// A grid point, on the first cell of a double-width character.
    pub point: Point,
    /// DECTCEM: the program shows the cursor.
    pub visible: bool,
    /// What DECSCUSR chose, `None` for the host's own.
    pub shape: Option<CursorShape>,
    /// What DECSCUSR or mode 12 chose, `None` for the host's own.
    pub blinking: Option<bool>,
    /// On a double-width character.
    pub wide: bool,
}

impl Terminal {
    /// Columns of the screen.
    pub fn columns(&self) -> usize {
        self.cols
    }

    /// Rows of the live screen.
    pub fn screen_lines(&self) -> usize {
        self.rows
    }

    /// Lines of history above the live screen (0 on the alternate screen).
    pub fn history_size(&self) -> usize {
        self.grid.history_len()
    }

    /// How many lines the view is scrolled up into the history.
    pub fn display_offset(&self) -> usize {
        self.grid.display_offset()
    }

    /// The modes the app reads: cursor, wrap, alternate screen, mouse, paste, focus and the
    /// kitty keyboard flags.
    pub fn mode(&self) -> TermMode {
        // The low 17 bits are the same in both sets; bit 17 is a mode of ours (insert) and the
        // locator's is not one of the terminal's modes, so it is added by hand.
        TermMode::from_bits_truncate((self.modes.bits() & 0x1_ffff) as u32) | self.locator_mode()
    }

    /// The cursor as it is drawn.
    pub fn cursor(&self) -> TerminalCursor {
        let (mut x, y) = (self.cursor.x, self.cursor.y);
        let cell = self.grid.live_cell(y, x);
        let mut wide = cell.is_some_and(|cell| cell.is_wide());
        if cell.is_some_and(|cell| cell.is_spacer_tail()) && x > 0 {
            x -= 1;
            wide = true;
        }
        TerminalCursor {
            point: Point::new(Line(y as i32), Column(x)),
            visible: self.modes.contains(Modes::SHOW_CURSOR),
            shape: self.cursor_shape,
            blinking: self.cursor_blink,
            wide,
        }
    }

    /// Where the cursor is on the grid, shown or not (the cell it is waiting on, with a wrap
    /// pending).
    pub fn grid_cursor(&self) -> Point {
        self.cursor_point()
    }

    /// Copy the rows of the view into `out`; see [`Grid::copy_viewport`].
    pub fn copy_viewport(&mut self, out: &mut Vec<ViewRow>, force: bool) {
        self.grid.copy_viewport(out, force);
    }

    /// Copy the row at `line` (history included); see [`Grid::copy_row`].
    pub fn copy_row(
        &self,
        line: Line,
        cells: &mut Vec<Cell>,
        extras: Option<&mut RowExtras>,
    ) -> Option<RowMark> {
        self.grid.copy_row(line, cells, extras)
    }

    /// Whether `line` goes on in the next row (a soft wrap).
    pub fn is_wrapped(&self, line: Line) -> bool {
        self.grid.is_wrapped(line)
    }

    /// The block mark (OSC 133) the row at `line` carries.
    pub fn row_mark(&self, line: Line) -> Option<RowMark> {
        self.grid.row_mark(line)
    }

    /// The text of `line`; see [`Grid::row_text`].
    pub fn row_text(&self, line: Line, trim: bool) -> String {
        self.grid.row_text(line, trim)
    }

    /// Which viewport rows changed since the last call; see [`Grid::take_damage`].
    pub fn take_damage(&mut self, damaged: &mut [bool]) -> bool {
        self.grid.take_damage(damaged)
    }

    /// Scroll the view through the history.
    pub fn scroll_display(&mut self, scroll: Scroll) {
        self.grid.scroll_view(scroll);
    }

    /// Erase the history; the screen stays.
    pub fn clear_history(&mut self) {
        self.grid.clear_scrollback();
    }

    /// The screen being shown, for everything else that reads it: cells, styles, hyperlinks,
    /// pins.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// A reference to the row at `point` that follows its text while the screen scrolls and the
    /// width is reflowed (selections, search hits and marks hold these); see [`Grid::pin`].
    pub fn pin(&mut self, point: Point) -> PinId {
        self.grid.pin(point)
    }

    /// Where the pin is now, `None` once its row has left the history.
    pub fn pin_point(&self, pin: PinId) -> Option<Point> {
        self.grid.pin_point(pin)
    }

    /// Forget a pin.
    pub fn unpin(&mut self, pin: PinId) {
        self.grid.unpin(pin);
    }

    /// Re-aim a pin.
    pub fn move_pin(&mut self, pin: PinId, point: Point) {
        self.grid.move_pin(pin, point);
    }

    /// Whether any picture is placed on the screen showing (history included): a frame without
    /// one skips [`placements`](Self::placements).
    pub fn has_placements(&self) -> bool {
        self.grid.has_placements()
    }

    /// The pictures that touch the view, into `out`: lowest `z` first, the oldest first among
    /// equals, which is the order to draw them in. Each has its cells, the part of the picture
    /// to show and how it is sized; see [`ImagePlacement`]. Cheap: no allocation once `out` has
    /// grown, and nothing at all while no picture is placed.
    pub fn placements(&self, out: &mut Vec<ImagePlacement>) {
        self.grid.copy_placements(out, self.cell_px);
    }

    /// Whether a synchronized update (mode 2026) is in progress: the host may hold back drawing
    /// until it ends.
    pub fn synchronized_output(&self) -> bool {
        self.modes.contains(Modes::SYNC_OUTPUT)
    }
}
