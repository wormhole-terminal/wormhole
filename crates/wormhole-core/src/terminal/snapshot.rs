//! What the UI draws, copied out of the terminal under one short lock.
//!
//! The PTY thread parses output while it holds the same lock, so the UI should
//! hold it only to copy cells, never while it shapes or paints. The copy is
//! incremental: alacritty records which lines changed since the last snapshot
//! (its "damage"), and only those rows are copied again. Each row says whether
//! it changed, so the UI can keep everything it built for the others.

use std::ops::Range;

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::selection::SelectionRange;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::term::{Term, TermDamage, TermMode};
use alacritty_terminal::vte::ansi::CursorShape;

use super::blocks::BlockState;
use super::marks::{self, RowMark};

/// How far above the viewport the block its top row continues is looked for: a run of unmarked
/// rows this long (blank lines, or lines a program printed as links) ends the block's mark.
const BLOCK_ABOVE_MAX: i32 = 1024;

/// One viewport row.
#[derive(Default)]
pub struct SnapshotRow {
    cells: Vec<Cell>,
    changed: bool,
    /// What the row carries of a command block, as of its last copy.
    mark: Option<RowMark>,
}

impl SnapshotRow {
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Whether the row was copied anew in the last snapshot (its content may
    /// still be the same; unchanged rows are certainly the same).
    pub fn changed(&self) -> bool {
        self.changed
    }
}

/// One command block's rows in the viewport, for the gutter beside them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockSpan {
    pub id: u64,
    pub rows: Range<usize>,
    /// `rows.start` is the block's first row (its prompt's); otherwise the block began above the
    /// viewport.
    pub starts: bool,
    pub state: BlockState,
}

/// Where the cursor is drawn, in viewport cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotCursor {
    pub row: usize,
    pub col: usize,
    pub shape: CursorShape,
    /// On a double-width character.
    pub wide: bool,
    /// The program asked for a blinking cursor (DECSCUSR 1/3/5, or DECSET 12).
    pub blinking: bool,
}

/// A reusable copy of the visible screen; see the module docs.
pub struct ScreenSnapshot {
    cols: usize,
    rows: Vec<SnapshotRow>,
    /// Rows just above the viewport, nearest first (drawn blurred under the title bar).
    above: Vec<Vec<Cell>>,
    /// Scratch: which rows the terminal reported damaged.
    damaged: Vec<bool>,
    pub cursor: Option<SnapshotCursor>,
    selection: Option<SelectionRange>,
    pub mode: TermMode,
    pub display_offset: usize,
    pub history_size: usize,
    /// Colors programs set at runtime (OSC 4/10/11/12), by palette index.
    pub colors: Colors,
    /// The command blocks in view, top to bottom.
    spans: Vec<BlockSpan>,
    /// A block mark was seen at some point: the shell has integration, so blocks are looked for.
    saw_marks: bool,
}

impl Default for ScreenSnapshot {
    fn default() -> Self {
        Self {
            cols: 0,
            rows: Vec::new(),
            above: Vec::new(),
            damaged: Vec::new(),
            cursor: None,
            selection: None,
            mode: TermMode::empty(),
            display_offset: 0,
            history_size: 0,
            colors: Colors::default(),
            spans: Vec::new(),
            saw_marks: false,
        }
    }
}

impl ScreenSnapshot {
    pub fn columns(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> &[SnapshotRow] {
        &self.rows
    }

    /// The command blocks in view, top to bottom (none without shell integration, or while a
    /// full-screen program has the screen).
    pub fn block_spans(&self) -> &[BlockSpan] {
        &self.spans
    }

    /// The block viewport `row` belongs to.
    pub fn block_at_row(&self, row: usize) -> Option<&BlockSpan> {
        self.spans.iter().find(|span| span.rows.contains(&row))
    }

    /// Give every block in view its state (`Pty::snapshot` does, from the commands' log).
    pub(super) fn set_block_states(&mut self, mut state: impl FnMut(u64) -> BlockState) {
        for span in &mut self.spans {
            span.state = state(span.id);
        }
    }

    /// Up to `n` rows above the viewport, nearest first.
    pub fn above(&self) -> &[Vec<Cell>] {
        &self.above
    }

    /// Whether earlier output sits above the visible screen.
    pub fn has_content_above(&self) -> bool {
        self.history_size > self.display_offset
    }

    /// The columns `[first, last]` of viewport `row` that are selected.
    pub fn selected_columns(&self, row: usize) -> Option<(usize, usize)> {
        let range = self.selection?;
        let line = Line(row as i32 - self.display_offset as i32);
        if line < range.start.line || line > range.end.line || self.cols == 0 {
            return None;
        }
        if range.is_block {
            return Some((range.start.column.0, range.end.column.0));
        }
        let first = if line == range.start.line { range.start.column.0 } else { 0 };
        let last = if line == range.end.line { range.end.column.0 } else { self.cols - 1 };
        (first <= last).then_some((first, last))
    }

    /// Bring the copy up to date with `term`. `above` is how many rows above
    /// the viewport to copy as well. This reads and resets the terminal's
    /// damage, so only one snapshot may follow a terminal (`Pty::snapshot`).
    pub fn update<T: EventListener>(&mut self, term: &mut Term<T>, above: usize) {
        let (cols, lines) = (term.columns(), term.screen_lines());
        let display_offset = term.grid().display_offset();
        // Clearing the history moves the view without damaging any line.
        let mut full = cols != self.cols || lines != self.rows.len() || display_offset != self.display_offset;
        self.damaged.clear();
        self.damaged.resize(lines, false);
        match term.damage() {
            TermDamage::Full => full = true,
            TermDamage::Partial(damage) => {
                for line in damage {
                    if let Some(row) = self.damaged.get_mut(line.line) {
                        *row = true;
                    }
                }
            }
        }
        term.reset_damage();

        self.cols = cols;
        self.display_offset = display_offset;
        self.rows.resize_with(lines, SnapshotRow::default);
        for (row, snapshot) in self.rows.iter_mut().enumerate() {
            snapshot.changed = full || self.damaged[row];
            if snapshot.changed {
                let grid_row = &term.grid()[Line(row as i32 - display_offset as i32)];
                snapshot.cells.clear();
                snapshot.cells.extend((0..cols).map(|col| grid_row[Column(col)].clone()));
                snapshot.mark = marks::row_mark(&snapshot.cells);
                self.saw_marks |= snapshot.mark.is_some();
            }
        }
        self.resolve_blocks(term);

        self.above.truncate(above);
        let topmost = term.topmost_line().0;
        let mut kept = 0;
        for k in 1..=above as i32 {
            let line = -(display_offset as i32) - k;
            if line < topmost {
                break;
            }
            let grid_row = &term.grid()[Line(line)];
            if self.above.len() <= kept {
                self.above.push(Vec::with_capacity(cols));
            }
            let copy = &mut self.above[kept];
            copy.clear();
            copy.extend((0..cols).map(|col| grid_row[Column(col)].clone()));
            kept += 1;
        }
        self.above.truncate(kept);

        let content = term.renderable_content();
        self.mode = content.mode;
        self.selection = content.selection;
        self.colors = *content.colors;
        let cursor = content.cursor;
        let blinking = term.cursor_style().blinking;
        self.cursor = cursor_row(cursor.point, display_offset, lines)
            .filter(|_| cursor.shape != CursorShape::Hidden)
            .map(|row| SnapshotCursor {
                row,
                col: cursor.point.column.0,
                shape: cursor.shape,
                wide: term.grid()[cursor.point].flags.contains(Flags::WIDE_CHAR),
                blinking,
            });
        self.history_size = term.grid().history_size();
    }

    /// Which block each row is in, from the rows' own marks: a row without one continues the
    /// block above it (a blank line), the top row the nearest marked line above the viewport. On
    /// the live screen the blank rows under the last block are nobody's. States are filled in
    /// later (`set_block_states`).
    fn resolve_blocks<T: EventListener>(&mut self, term: &Term<T>) {
        self.spans.clear();
        if !self.saw_marks || term.mode().contains(TermMode::ALT_SCREEN) {
            return;
        }
        // Also when the top row is marked: a prompt row whose block began above (a secondary
        // prompt) doesn't start it.
        let mut current = block_above(term, Line(-(self.display_offset as i32)));
        let last_marked = self.rows.iter().rposition(|row| row.mark.is_some());
        let live_screen = self.display_offset == 0;
        for (row, snapshot) in self.rows.iter().enumerate() {
            if live_screen && last_marked.is_none_or(|last| row > last) {
                break;
            }
            let (id, starts) = match snapshot.mark {
                Some(mark) => (mark.id, mark.prompt && current != Some(mark.id)),
                None => match current {
                    Some(id) => (id, false),
                    None => continue,
                },
            };
            current = Some(id);
            match self.spans.last_mut() {
                Some(span) if span.id == id && span.rows.end == row && !starts => span.rows.end = row + 1,
                _ => self.spans.push(BlockSpan { id, rows: row..row + 1, starts, state: BlockState::Prompt }),
            }
        }
    }
}

/// The block the line above `top` belongs to: the nearest marked line above, within reach.
fn block_above<T: EventListener>(term: &Term<T>, top: Line) -> Option<u64> {
    let lowest = term.topmost_line().max(Line(top.0 - BLOCK_ABOVE_MAX));
    let mut line = top - 1;
    while line >= lowest {
        if let Some(mark) = marks::mark_at(term, line) {
            return Some(mark.id);
        }
        line -= 1;
    }
    None
}

/// The viewport row of a grid point, if it is on screen.
fn cursor_row(point: Point, display_offset: usize, lines: usize) -> Option<usize> {
    let row = point.line.0 + display_offset as i32;
    (0..lines as i32).contains(&row).then_some(row as usize)
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::grid::Scroll;
    use alacritty_terminal::index::{Column, Line, Point, Side};
    use alacritty_terminal::selection::{Selection, SelectionType};
    use alacritty_terminal::term::{Config, Term};
    use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

    use super::ScreenSnapshot;
    use crate::terminal::GridSize;

    fn term(cols: u16, rows: u16) -> Term<VoidListener> {
        let size = GridSize { cols, rows, cell_width: 8.0, cell_height: 16.0 };
        Term::new(Config { scrolling_history: 100, ..Config::default() }, &size, VoidListener)
    }

    fn feed(term: &mut Term<VoidListener>, bytes: &[u8]) {
        Processor::<StdSyncHandler>::new().advance(term, bytes);
    }

    fn row_text(snapshot: &ScreenSnapshot, row: usize) -> String {
        snapshot.rows()[row].cells().iter().map(|c| c.c).collect::<String>().trim_end().to_string()
    }

    fn changed(snapshot: &ScreenSnapshot) -> Vec<usize> {
        (0..snapshot.rows().len()).filter(|&r| snapshot.rows()[r].changed()).collect()
    }

    #[test]
    fn only_damaged_rows_are_copied_again() {
        let mut term = term(10, 4);
        feed(&mut term, b"one\r\ntwo\r\n$ ");
        let mut snapshot = ScreenSnapshot::default();
        snapshot.update(&mut term, 0);
        assert_eq!(changed(&snapshot), [0, 1, 2, 3], "the first copy is complete");
        assert_eq!(row_text(&snapshot, 1), "two");

        snapshot.update(&mut term, 0);
        // Nothing printed: at most the cursor's row is reported.
        assert!(changed(&snapshot).iter().all(|&row| row == 2));

        feed(&mut term, b"ls");
        snapshot.update(&mut term, 0);
        assert_eq!(changed(&snapshot), [2]);
        assert_eq!(row_text(&snapshot, 2), "$ ls");
        assert_eq!(row_text(&snapshot, 0), "one", "untouched rows keep their copy");
    }

    #[test]
    fn scrolling_and_clearing_history_recopy_everything() {
        let mut term = term(10, 3);
        let mut snapshot = ScreenSnapshot::default();
        feed(&mut term, b"1\r\n2\r\n3\r\n4\r\n5");
        snapshot.update(&mut term, 2);
        assert_eq!(row_text(&snapshot, 0), "3");
        assert!(snapshot.has_content_above());
        let above: Vec<char> = snapshot.above().iter().map(|row| row[0].c).collect();
        assert_eq!(above, ['2', '1'], "nearest first");

        term.scroll_display(Scroll::Delta(1));
        snapshot.update(&mut term, 2);
        assert_eq!(changed(&snapshot), [0, 1, 2]);
        assert_eq!(row_text(&snapshot, 0), "2");
        assert_eq!(snapshot.display_offset, 1);

        term.grid_mut().clear_history();
        snapshot.update(&mut term, 2);
        assert_eq!(changed(&snapshot), [0, 1, 2], "the view moved without damage");
        assert_eq!(row_text(&snapshot, 0), "3");
        assert!(!snapshot.has_content_above());
        assert!(snapshot.above().is_empty());
    }

    #[test]
    fn selection_spans_per_row() {
        let mut term = term(10, 3);
        feed(&mut term, b"aaaa\r\nbbbb\r\ncccc");
        let mut selection = Selection::new(SelectionType::Simple, Point::new(Line(0), Column(2)), Side::Left);
        selection.update(Point::new(Line(1), Column(1)), Side::Right);
        term.selection = Some(selection);
        let mut snapshot = ScreenSnapshot::default();
        snapshot.update(&mut term, 0);
        assert_eq!(snapshot.selected_columns(0), Some((2, 9)));
        assert_eq!(snapshot.selected_columns(1), Some((0, 1)));
        assert_eq!(snapshot.selected_columns(2), None);
    }

    /// The spans as (id, rows, starts).
    fn spans(snapshot: &ScreenSnapshot) -> Vec<(u64, std::ops::Range<usize>, bool)> {
        snapshot.block_spans().iter().map(|s| (s.id, s.rows.clone(), s.starts)).collect()
    }

    #[test]
    fn block_spans_cover_each_blocks_rows_and_leave_the_live_screens_tail_alone() {
        use crate::terminal::blocks::tests::{SESSION, shown};
        let (mut term, _) = shown(10, SESSION);
        let mut snapshot = ScreenSnapshot::default();
        snapshot.update(&mut term, 0);
        // Rows: `$ ls` / a.rs / b.rs / `$ make` / error / blank / `$ ` / blank …: the blank row
        // after `error` is the block's (a blank line it printed), the rows under the prompt nobody's.
        assert_eq!(spans(&snapshot), [(1, 0..3, true), (2, 3..6, true), (3, 6..7, true)]);
        assert_eq!(snapshot.block_at_row(5).map(|s| s.id), Some(2));
        assert_eq!(snapshot.block_at_row(8), None);
        // A full-screen program: no blocks.
        feed(&mut term, b"\x1b[?1049hvim");
        snapshot.update(&mut term, 0);
        assert!(snapshot.block_spans().is_empty());
    }

    #[test]
    fn a_block_that_began_above_the_viewport_continues_at_its_top() {
        use crate::terminal::blocks::tests::{SESSION, shown};
        let (mut term, _) = shown(3, SESSION);
        let mut snapshot = ScreenSnapshot::default();
        // The live screen shows `error`, blank, `$ `: the first row is inside block 2.
        snapshot.update(&mut term, 0);
        assert_eq!(spans(&snapshot), [(2, 0..2, false), (3, 2..3, true)]);
        // Scrolled up by one: `$ make` is the top row and starts its block in view.
        term.scroll_display(Scroll::Delta(1));
        snapshot.update(&mut term, 0);
        assert_eq!(spans(&snapshot), [(2, 0..3, true)]);
        // Scrolled to the top: `$ ls` and its output.
        term.scroll_display(Scroll::Bottom);
        term.scroll_display(Scroll::Delta(4));
        snapshot.update(&mut term, 0);
        assert_eq!(spans(&snapshot), [(1, 0..3, true)]);
    }

    #[test]
    fn without_shell_integration_no_rows_are_looked_at_for_blocks() {
        let mut term = term(10, 3);
        feed(&mut term, b"plain\r\noutput");
        let mut snapshot = ScreenSnapshot::default();
        snapshot.update(&mut term, 0);
        assert!(!snapshot.saw_marks && snapshot.block_spans().is_empty());
    }

    #[test]
    fn the_cursor_is_where_the_screen_shows_it() {
        let mut term = term(10, 3);
        feed(&mut term, "1\r\n2\r\n3\r\n日".as_bytes());
        let mut snapshot = ScreenSnapshot::default();
        snapshot.update(&mut term, 0);
        let cursor = snapshot.cursor.expect("cursor");
        assert_eq!((cursor.row, cursor.col, cursor.wide), (2, 2, false));
        // Scrolled up far enough, the cursor's row leaves the screen.
        term.scroll_display(Scroll::Delta(1));
        snapshot.update(&mut term, 0);
        assert_eq!(snapshot.cursor, None);
    }
}
