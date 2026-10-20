//! The terminal as the rest of core sees it: the engine's [`Terminal`] (`wormhole_vt`), a locked
//! grid with a history above it, read and driven in `wormhole_vt::types` only, plus the one thing
//! the engine does not know, whether the UI should look at it (a synchronized update, mode 2026,
//! is in progress).
//!
//! Everything in `terminal` that looks at the screen (snapshot, text, find, links, blocks, the
//! minimap) or drives it (scrolling, selecting, resizing) goes through [`Screen`]. Almost every
//! method is a forward: the engine's snapshot API was shaped like this. What is converted is the
//! block marks (the engine numbers blocks as `u32` and keeps them on the rows; core's ids are
//! `u64`), the cursor (the engine says what the program chose and leaves the rest to the host)
//! and the cell size, which the engine needs for its pictures and reports.

use std::time::Instant;

use wormhole_vt::grid::{Content, RowMark as GridMark, TextShape};
use wormhole_vt::parser::Parser;
use wormhole_vt::terminal::{ScreenChecksum, Terminal, VirtualPlacement, WindowGeometry, WindowOps};
use wormhole_vt::types::Rgb;
use wormhole_vt::types::{
    COLOR_COUNT, Cell, Colors, CursorShape, ImagePlacement, Line, Point, RowExtras, Scroll, SelectionRange,
    SelectionType, Side, TermMode,
};

use super::GridSize;
use super::marks::{Part, RowMark};
use wormhole_vt::types::Column;

/// Where the cursor is drawn and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScreenCursor {
    /// A grid point (the visible row is `point.line + display_offset`), on the first cell of a
    /// double-width character.
    pub point: Point,
    /// [`CursorShape::Hidden`] when the program hid it.
    pub shape: CursorShape,
    /// The program asked for a blinking cursor (DECSCUSR 1/3/5, or DECSET 12).
    pub blinking: bool,
    /// On a double-width character.
    pub wide: bool,
}

/// How long a synchronized update may hold the screen back when the program never ends it
/// (the spec leaves it to the terminal; 150 ms is what terminals settle on, long enough for a
/// redraw and short enough that a program that died mid-update does not look frozen).
pub(super) const SYNC_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(150);

/// Whether the screen is held back while a synchronized update runs, and for how long.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Hold {
    /// No update in progress: the UI shows what the engine has.
    #[default]
    Off,
    /// An update began at this time: the UI keeps the last frame until it ends or times out.
    Since(Instant),
    /// The update did not end in time: the screen shows again, and the update's mode stays set
    /// until the program resets it.
    GaveUp,
}

/// A locked terminal. Lines are grid lines: 0 is the live screen's top row, the history above it
/// is negative, and the view's offset into the history doesn't move them. The engine's terminal
/// and the state core keeps beside it.
pub(crate) struct Screen {
    pub(super) term: Terminal,
    pub(super) hold: Hold,
}

fn mark_from(mark: GridMark) -> RowMark {
    RowMark { id: u64::from(mark.block), prompt: mark.prompt, command: mark.command, output: mark.output }
}

impl Screen {
    pub(crate) fn new(size: GridSize, scrollback_lines: usize) -> Self {
        let mut term = Terminal::new(usize::from(size.cols), usize::from(size.rows), scrollback_lines);
        term.set_cell_size(size.cell_width as u16, size.cell_height as u16);
        Self { term, hold: Hold::Off }
    }

    /// The shell's marks the engine queued, numbered as `Pty` numbers them, and the other events
    /// dropped: for a test terminal, which has no host to hear them.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn drain_marks(&mut self, prompts: &mut u64) -> Vec<(crate::Mark, u64)> {
        self.term.drain_events().filter_map(|event| super::engine::mark_of(event, prompts)).collect()
    }

    /// Note whether a synchronized update is in progress after the engine read some output.
    pub(super) fn update_hold(&mut self, now: Instant) {
        self.hold = match (self.term.synchronized_output(), self.hold) {
            (false, _) => Hold::Off,
            (true, Hold::Off) => Hold::Since(now),
            (true, hold) => hold,
        };
    }

    pub(crate) fn columns(&self) -> usize {
        self.term.columns()
    }

    /// Rows of the live screen.
    pub(crate) fn screen_lines(&self) -> usize {
        self.term.screen_lines()
    }

    /// Lines of history above the live screen.
    pub(crate) fn history_size(&self) -> usize {
        self.term.history_size()
    }

    /// How many lines the view is scrolled up into the history.
    pub(crate) fn display_offset(&self) -> usize {
        self.term.display_offset()
    }

    pub(crate) fn mode(&self) -> TermMode {
        self.term.mode()
    }

    /// The palette entries programs have overridden (OSC 4/10/11/12).
    pub(crate) fn colors(&self) -> Colors {
        self.term.colors()
    }

    /// The oldest line of the history (the live screen's top when there is none).
    pub(crate) fn topmost_line(&self) -> Line {
        Line(-(self.history_size() as i32))
    }

    /// The live screen's bottom row.
    pub(crate) fn bottommost_line(&self) -> Line {
        Line(self.screen_lines() as i32 - 1)
    }

    /// The last column (0 for a grid without any).
    pub(crate) fn last_column(&self) -> Column {
        Column(self.columns().saturating_sub(1))
    }

    /// Copy grid `line` into `cells` (emptied first; one cell per column) and, when asked, what
    /// its cells carry beyond that into `extras` (emptied first). Returns the block mark the row
    /// carries, which is found in the same pass.
    pub(crate) fn copy_row(
        &self,
        line: Line,
        cells: &mut Vec<Cell>,
        extras: Option<&mut RowExtras>,
    ) -> Option<RowMark> {
        self.term.copy_row(line, cells, extras).map(mark_from)
    }

    /// Whether `line` goes on in the next row (a soft wrap, not a newline).
    pub(crate) fn wraps(&self, line: Line) -> bool {
        self.term.is_wrapped(line)
    }

    /// The block mark on `line` alone, without copying it (a scan of the whole grid calls this).
    pub(crate) fn row_mark(&self, line: Line) -> Option<RowMark> {
        self.term.row_mark(line).map(mark_from)
    }

    /// The text of the cells of `line` that carry `part` of block `id` (wide characters'
    /// spacers skipped, a blank where a cell holds no character), untrimmed.
    pub(crate) fn part_text(&self, line: Line, id: u64, part: Part) -> String {
        // A block number past u32 is none of the engine's.
        let Ok(block) = u32::try_from(id) else { return String::new() };
        let content = match part {
            Part::Prompt => Content::Prompt,
            Part::Command => Content::Command,
            Part::Output => Content::Output,
        };
        self.term.content_text(line, block, content)
    }

    /// The text from `start` to `end` inclusive, wrapped lines joined, as copying it would give.
    pub(crate) fn text_between(&self, start: Point, end: Point) -> String {
        self.term.text_between(start, end, TextShape::Stream)
    }

    /// The selected text.
    pub(crate) fn selection_text(&self) -> Option<String> {
        self.term.selection_text()
    }

    /// The selected cells, clamped to the grid.
    pub(crate) fn selection_range(&self) -> Option<SelectionRange> {
        self.term.selection_range()
    }

    pub(crate) fn start_selection(&mut self, kind: SelectionType, point: Point, side: Side) {
        self.term.start_selection(kind, point, side);
    }

    /// Move the selection being dragged to `point` (nothing without one).
    pub(crate) fn update_selection(&mut self, point: Point, side: Side) {
        self.term.update_selection(point, side);
    }

    /// The pointer for the DEC locator: a button changed (`Some`) or it moved (`None`). The reports
    /// the engine makes of it, for the program.
    pub(crate) fn locator_event(
        &mut self,
        button: Option<(wormhole_vt::terminal::LocatorButton, bool)>,
        at: wormhole_vt::terminal::Pointer,
    ) -> Vec<u8> {
        match button {
            Some((button, pressed)) => self.term.locator_button(button, pressed, at),
            None => self.term.locator_moved(at),
        }
        let replies = self.term.replies().to_vec();
        self.term.clear_replies();
        replies
    }

    /// Drop the selection; whether there was one.
    pub(crate) fn take_selection(&mut self) -> bool {
        self.term.take_selection()
    }

    /// Drop the selection when it covers nothing (a click without a drag).
    pub(crate) fn drop_empty_selection(&mut self) {
        self.term.drop_empty_selection();
    }

    /// The cursor as drawn.
    pub(crate) fn cursor(&self) -> ScreenCursor {
        let cursor = self.term.cursor();
        ScreenCursor {
            point: cursor.point,
            // The program's choice (DECSCUSR), else a steady block, which is what the app draws
            // for a program that never asked.
            shape: if cursor.visible {
                cursor.shape.unwrap_or(CursorShape::Block)
            } else {
                CursorShape::Hidden
            },
            blinking: cursor.blinking.unwrap_or(false),
            wide: cursor.wide,
        }
    }

    /// Where the cursor is on the grid, whether or not it is shown.
    pub(crate) fn grid_cursor(&self) -> Point {
        self.term.grid_cursor()
    }

    pub(crate) fn scroll_display(&mut self, scroll: Scroll) {
        self.term.scroll_display(scroll);
    }

    /// Erase the history (the screen stays).
    pub(crate) fn clear_history(&mut self) {
        self.term.clear_history();
    }

    pub(crate) fn resize(&mut self, size: GridSize) {
        self.term.set_cell_size(size.cell_width as u16, size.cell_height as u16);
        self.term.resize(usize::from(size.cols), usize::from(size.rows));
    }

    /// Which rows changed since the last call, and forget them: `damaged[row]` (a viewport row)
    /// is set for each. Returns whether everything changed (a resize, a scroll of the view,
    /// the alternate screen), in which case `damaged` says nothing.
    pub(crate) fn take_damage(&mut self, damaged: &mut [bool]) -> bool {
        self.term.take_damage(damaged)
    }

    /// Parse `bytes` as program output. Whole sequences only: a sequence cut in two calls is
    /// not joined (what replaying saved text and the tests need; the PTY thread has its own
    /// stream).
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        self.term.advance(&mut Parser::new(), bytes);
        self.update_hold(Instant::now());
    }

    /// A synchronized update (mode 2026) is in progress and the UI should keep showing the last
    /// frame until it ends or gives out.
    pub(crate) fn held_back(&self) -> bool {
        matches!(self.hold, Hold::Since(_))
    }

    /// The pictures placed in the grid that touch the view, lowest `z` first (emptied first).
    pub(crate) fn placements(&self, out: &mut Vec<ImagePlacement>) {
        if self.term.has_placements() {
            self.term.placements(out);
        } else {
            out.clear();
        }
    }

    /// Whether the engine answers DECRQCRA (a program reading the screen back); it does not until
    /// the host says so.
    pub(crate) fn set_screen_checksum(&mut self, allow: bool) {
        self.term.set_screen_checksum(if allow { ScreenChecksum::Allow } else { ScreenChecksum::Deny });
    }

    /// Whether a program may act on the window and read its title; it may not until the host
    /// says so.
    pub(crate) fn set_window_ops(&mut self, allow: bool) {
        self.term.set_window_ops(if allow { WindowOps::Allow } else { WindowOps::Deny });
    }

    /// Whether programs may act on the window.
    pub(crate) fn window_ops_allowed(&self) -> bool {
        self.term.window_ops() == WindowOps::Allow
    }

    /// Where the host's window is, for the reports a program asks for.
    pub(crate) fn set_window(&mut self, geometry: WindowGeometry) {
        self.term.set_window(geometry);
    }

    /// The theme's colors, which the engine answers a program's color queries (OSC 4, 10, 11, 12)
    /// with unless the program set the entry itself.
    pub(crate) fn set_default_colors(&mut self, palette: &[Rgb; COLOR_COUNT]) {
        self.term.set_default_colors(palette);
    }

    /// The virtual placement (kitty's `U=1`) the placeholder cells with this id belong to.
    pub(crate) fn virtual_placement(&self, image_id: u32, placement_id: u32) -> Option<VirtualPlacement> {
        self.term.virtual_placement(image_id, placement_id)
    }
}
