//! What the host draws and asks from: the size, the modes, the cursor, rows copied out, damage,
//! the view's scroll position, pins and the pictures to place.
//!
//! The host locks the terminal, copies what changed ([`Terminal::copy_viewport`] copies only the
//! rows marked dirty since the last call, into buffers the host keeps) and lets go; nothing here
//! allocates once the buffers are the right size.

use crate::grid::{Pin, RowMark, ViewRow};
use crate::types::{Cell, Column, CursorShape, ImagePlacement, Line, LineSize, Point, RowExtras, Scroll};

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;

/// Where the cursor is and how a program wants it drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cursor {
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
    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Rows of the live screen.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The screen's size as `(cols, rows)`.
    #[must_use]
    pub fn size(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    /// Lines of history above the live screen (0 on the alternate screen).
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.grid.history_len()
    }

    /// How many lines the view is scrolled up into the history; 0 is the live screen.
    #[must_use]
    pub fn scroll_offset(&self) -> usize {
        self.grid.display_offset()
    }

    /// The cursor as it is drawn.
    pub fn cursor(&self) -> Cursor {
        let (mut x, y) = (self.cursor.x, self.cursor.y);
        let cell = self.grid.live_cell(y, x);
        let mut wide = cell.is_some_and(|cell| cell.is_wide());
        if cell.is_some_and(|cell| cell.is_spacer_tail()) && x > 0 {
            x -= 1;
            wide = true;
        }
        Cursor {
            point: Point::new(Line(y as i32), Column(x)),
            visible: self.modes.contains(Modes::SHOW_CURSOR),
            shape: self.cursor_style.shape,
            blinking: self.cursor_style.blink,
            wide,
        }
    }

    /// Copy the rows of the view (the live screen, or the history if the view is scrolled up)
    /// into `out`, which is resized to the screen's height. Only rows that changed since the last
    /// call are copied, all of them after `force`, a resize, a clear or a move of the view; every
    /// row's `changed` says which. Clears the rows' changed marks.
    pub fn copy_viewport(&mut self, out: &mut Vec<ViewRow>, force: bool) {
        self.grid.copy_viewport(out, force);
    }

    /// Copy the row at `line` (history included) into `cells`, and what its cells carry beside
    /// them into `extras` when given. Returns the block mark the row carries. Doesn't touch the
    /// changed marks.
    pub fn copy_row(
        &self,
        line: Line,
        cells: &mut Vec<Cell>,
        extras: Option<&mut RowExtras>,
    ) -> Option<RowMark> {
        self.grid.copy_row(line, cells, extras)
    }

    /// The cell at `point` as a host draws it: its character, resolved colours and flags. `None`
    /// outside the screen and the history. The characters a cluster adds, the underline colour
    /// and the hyperlink are in the row's [`RowExtras`] ([`copy_row`](Self::copy_row)); this reads
    /// one cell, so a host that wants a whole row copies the row.
    #[must_use]
    pub fn cell(&self, point: Point) -> Option<Cell> {
        self.grid.draw_cell(point)
    }

    /// Whether `line` goes on in the next row (a soft wrap).
    pub fn is_wrapped(&self, line: Line) -> bool {
        self.grid.is_wrapped(line)
    }

    /// How big the row at `line` is drawn (DECDWL, DECDHL); `Single` for a line the terminal does
    /// not have. The rows of [`copy_viewport`](Self::copy_viewport) carry it too.
    pub fn line_size(&self, line: Line) -> LineSize {
        self.grid.line_size(line)
    }

    /// The block mark (OSC 133) the row at `line` carries.
    pub fn row_mark(&self, line: Line) -> Option<RowMark> {
        self.grid.row_mark(line)
    }

    /// The text of `line`: characters and their marks, wide characters once, empty cells as
    /// spaces, with trailing blanks cut when `trim` is set.
    pub fn row_text(&self, line: Line, trim: bool) -> String {
        self.grid.row_text(line, trim)
    }

    /// Scroll the view through the history. Not the program's scrolling: the screen and the
    /// cursor stay as they are, only what [`copy_viewport`](Self::copy_viewport) shows moves.
    pub fn scroll(&mut self, scroll: Scroll) {
        self.grid.scroll_view(scroll);
    }

    /// Erase the history; the screen stays.
    pub fn clear_history(&mut self) {
        self.grid.clear_scrollback();
    }

    /// The screen being shown, for our own tools (the fuzz targets, the benches) that look at the
    /// storage itself.
    #[cfg(feature = "internals")]
    #[doc(hidden)]
    pub fn grid(&self) -> &crate::grid::Grid {
        &self.grid
    }

    /// A reference to the row at `point` that follows its text while the screen scrolls and the
    /// width is reflowed (selections, search hits and marks hold these), and says so when the row has
    /// left the history.
    pub fn pin(&mut self, point: Point) -> Pin {
        self.grid.pin(point)
    }

    /// Where the pin is now, `None` once its row has left the history.
    pub fn pin_point(&self, pin: Pin) -> Option<Point> {
        self.grid.pin_point(pin)
    }

    /// Forget a pin.
    pub fn unpin(&mut self, pin: Pin) {
        self.grid.unpin(pin);
    }

    /// Re-aim a pin.
    pub fn move_pin(&mut self, pin: Pin, point: Point) {
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
