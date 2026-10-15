//! Terminals for tests, in this crate's and in the app's (which turns on the `test-support`
//! feature in its dev-dependencies): a screen nothing listens to that is fed bytes directly, and
//! read back in `wormhole_vt::types`.

use wormhole_vt::types::{Cell, Line, Point, RowExtras, Scroll, SelectionType, Side, TermMode};

use super::screen::Screen;
use super::snapshot::ScreenSnapshot;
use super::{GridSize, Mark};

/// A terminal without a PTY.
pub struct TestTerm {
    screen: Box<Screen>,
    /// The prompts seen so far, which number a block.
    prompts: u64,
}

impl TestTerm {
    /// A screen `cols` × `rows` with room for `history` lines above it.
    pub fn new(cols: u16, rows: u16, history: usize) -> Self {
        let size = GridSize { cols, rows, cell_width: 8.0, cell_height: 16.0 };
        Self { screen: Box::new(Screen::new(size, history)), prompts: 0 }
    }

    /// Print `bytes` as a shell with integration would, and return the shell's marks in them
    /// with the number of the prompt each belongs to: what `Pty` would have told its command log.
    pub fn feed_with_marks(&mut self, bytes: &[u8]) -> Vec<(Mark, u64)> {
        self.screen.feed(bytes);
        self.screen.drain_marks(&mut self.prompts)
    }

    /// Print `bytes` as a program would. A sequence cut between two calls is not joined.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.screen_mut().feed(bytes);
    }

    /// Bring `out` up to date with the screen, as `Pty::snapshot` does (block states aside).
    pub fn snapshot(&mut self, out: &mut ScreenSnapshot, above: usize) {
        out.update(self.screen_mut(), above);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.screen_mut().resize(GridSize { cols, rows, cell_width: 8.0, cell_height: 16.0 });
    }

    pub fn scroll_display(&mut self, scroll: Scroll) {
        self.screen_mut().scroll_display(scroll);
    }

    pub fn clear_history(&mut self) {
        self.screen_mut().clear_history();
    }

    pub fn start_selection(&mut self, kind: SelectionType, point: Point, side: Side) {
        self.screen_mut().start_selection(kind, point, side);
    }

    pub fn update_selection(&mut self, point: Point, side: Side) {
        self.screen_mut().update_selection(point, side);
    }

    pub fn clear_selection(&mut self) {
        self.screen_mut().take_selection();
    }

    /// The cell at grid `line` and `col`.
    pub fn cell(&self, line: i32, col: usize) -> Cell {
        self.row(line).0[col]
    }

    /// Grid `line` with what its cells carry beside them.
    pub fn row(&self, line: i32) -> (Vec<Cell>, RowExtras) {
        let (mut cells, mut extras) = (Vec::new(), RowExtras::default());
        self.screen().copy_row(Line(line), &mut cells, Some(&mut extras));
        (cells, extras)
    }

    /// The characters of grid `line`, one per column, blanks included.
    pub fn row_chars(&self, line: i32) -> String {
        self.row(line).0.iter().map(|cell| cell.c).collect()
    }

    /// The visible screen, one line per row, trailing blanks trimmed.
    pub fn screen_text(&self) -> String {
        super::text::screen_text(self.screen())
    }

    pub fn columns(&self) -> usize {
        self.screen().columns()
    }

    pub fn screen_lines(&self) -> usize {
        self.screen().screen_lines()
    }

    pub fn history_size(&self) -> usize {
        self.screen().history_size()
    }

    pub fn display_offset(&self) -> usize {
        self.screen().display_offset()
    }

    pub fn mode(&self) -> TermMode {
        self.screen().mode()
    }

    /// Where the cursor is on the grid.
    pub fn cursor(&self) -> Point {
        self.screen().grid_cursor()
    }

    pub(crate) fn screen(&self) -> &Screen {
        &self.screen
    }

    pub(crate) fn screen_mut(&mut self) -> &mut Screen {
        &mut self.screen
    }
}
