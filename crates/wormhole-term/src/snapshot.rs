//! What the UI draws, copied out of the terminal under one short lock.
//!
//! The PTY thread parses output while it holds the same lock, so the UI should
//! hold it only to copy cells, never while it shapes or paints. The copy is
//! incremental: the engine records which lines changed since the last snapshot
//! (its "damage"), and only those rows are copied again. Each row says whether
//! it changed, so the UI can keep everything it built for the others.

use std::ops::Range;

use wormhole_vt::image::ImagePlacement;
use wormhole_vt::{
    Cell, Colors, CursorShape, Damage, Line, LineSize, Modes, Point, RowExtras, SelectionRange,
};

use super::blocks::BlockState;
use super::marks;
use super::row_mark::RowMark;
use super::screen::Screen;

/// How far above the viewport the block its top row continues is looked for: a run of unmarked
/// rows this long (blank lines, or lines a program printed as links) ends the block's mark.
const BLOCK_ABOVE_MAX: i32 = 1024;

/// One viewport row.
#[derive(Default)]
pub struct SnapshotRow {
    cells: Vec<Cell>,
    /// What the cells carry beside them: zero-width characters, underline colors, hyperlinks.
    extras: RowExtras,
    changed: bool,
    /// What the row carries of a command block, as of its last copy.
    mark: Option<RowMark>,
    /// How big the row is drawn, as of its last copy (a change of it damages the row).
    size: LineSize,
}

impl SnapshotRow {
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// What the cells carry beside them (empty for almost every row).
    pub fn extras(&self) -> &RowExtras {
        &self.extras
    }

    /// How big the row is drawn: double-width or half of a double-height line (DECDWL, DECDHL)
    /// show half the columns, each twice as wide.
    pub fn size(&self) -> LineSize {
        self.size
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
    damage: Damage,
    pub cursor: Option<SnapshotCursor>,
    selection: Option<SelectionRange>,
    pub mode: Modes,
    pub display_offset: usize,
    pub history_size: usize,
    /// Colors programs set at runtime (OSC 4/10/11/12), by palette index.
    pub colors: Colors,
    /// The command blocks in view, top to bottom.
    spans: Vec<BlockSpan>,
    /// The pictures placed in the grid that touch the view, in drawing order (kitty's Unicode
    /// placeholders are cells, not placements).
    placements: Vec<ImagePlacement>,
    /// A block mark was seen at some point: the shell has integration, so blocks are looked for.
    saw_marks: bool,
}

impl Default for ScreenSnapshot {
    fn default() -> Self {
        Self {
            cols: 0,
            rows: Vec::new(),
            above: Vec::new(),
            damage: Damage::new(),
            cursor: None,
            selection: None,
            mode: Modes::empty(),
            display_offset: 0,
            history_size: 0,
            colors: Colors::default(),
            spans: Vec::new(),
            placements: Vec::new(),
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

    /// The pictures placed in the grid that touch the view, lowest `z` first: draw them in this
    /// order. Their lines are grid lines; add [`display_offset`](Self::display_offset) for the
    /// viewport row.
    pub fn placements(&self) -> &[ImagePlacement] {
        &self.placements
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

    /// Bring the copy up to date with the screen. `above` is how many rows above
    /// the viewport to copy as well. This reads and resets the screen's
    /// damage, so only one snapshot may follow a terminal (`Pty::snapshot`).
    pub(super) fn update(&mut self, screen: &mut Screen, above: usize) {
        let (cols, lines) = (screen.columns(), screen.screen_lines());
        // A synchronized update shows nothing until it ends; the damage waits in the screen.
        if screen.held_back() && cols == self.cols && lines == self.rows.len() {
            for row in &mut self.rows {
                row.changed = false;
            }
            return;
        }
        let display_offset = screen.display_offset();
        // Clearing the history moves the view without damaging any line.
        let mut full = cols != self.cols || lines != self.rows.len() || display_offset != self.display_offset;
        screen.take_damage(&mut self.damage);
        full |= self.damage.is_full();

        self.cols = cols;
        self.display_offset = display_offset;
        self.rows.resize_with(lines, SnapshotRow::default);
        for (row, snapshot) in self.rows.iter_mut().enumerate() {
            snapshot.changed = full || self.damage.contains(row);
            if snapshot.changed {
                let line = Line(row as i32 - display_offset as i32);
                snapshot.mark = screen.copy_row(line, &mut snapshot.cells, Some(&mut snapshot.extras));
                snapshot.size = screen.line_size(line);
                self.saw_marks |= snapshot.mark.is_some();
            }
        }
        self.resolve_blocks(screen);

        self.above.truncate(above);
        let topmost = screen.topmost_line().0;
        let mut kept = 0;
        for k in 1..=above as i32 {
            let line = -(display_offset as i32) - k;
            if line < topmost {
                break;
            }
            if self.above.len() <= kept {
                self.above.push(Vec::with_capacity(cols));
            }
            screen.copy_row(Line(line), &mut self.above[kept], None);
            kept += 1;
        }
        self.above.truncate(kept);

        self.mode = screen.mode();
        self.selection = screen.selection_range();
        self.colors = screen.colors();
        let cursor = screen.cursor();
        self.cursor = cursor_row(cursor.point, display_offset, lines)
            .filter(|_| cursor.shape != CursorShape::Hidden)
            .map(|row| SnapshotCursor {
                row,
                col: cursor.point.column.0,
                shape: cursor.shape,
                wide: cursor.wide,
                blinking: cursor.blinking,
            });
        self.history_size = screen.history_size();
        screen.placements(&mut self.placements);
    }

    /// Which block each row is in, from the rows' own marks: a row without one continues the
    /// block above it (a blank line), the top row the nearest marked line above the viewport. On
    /// the live screen the blank rows under the last block are nobody's. States are filled in
    /// later (`set_block_states`).
    fn resolve_blocks(&mut self, screen: &Screen) {
        self.spans.clear();
        if !self.saw_marks || screen.mode().contains(Modes::ALT_SCREEN) {
            return;
        }
        // Also when the top row is marked: a prompt row whose block began above (a secondary
        // prompt) doesn't start it.
        let mut current = block_above(screen, Line(-(self.display_offset as i32)));
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
fn block_above(screen: &Screen, top: Line) -> Option<u64> {
    let lowest = screen.topmost_line().max(Line(top.0 - BLOCK_ABOVE_MAX));
    let mut line = top - 1;
    while line >= lowest {
        if let Some(mark) = marks::mark_at(screen, line) {
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
    use wormhole_vt::{Column, Line, Point, Scroll, SelectionType, Side};

    use super::ScreenSnapshot;
    use crate::testing::TestTerm;

    fn term(cols: u16, rows: u16) -> TestTerm {
        TestTerm::new(cols, rows, 100)
    }

    fn feed(term: &mut TestTerm, bytes: &[u8]) {
        term.feed(bytes);
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
        term.snapshot(&mut snapshot, 0);
        assert_eq!(changed(&snapshot), [0, 1, 2, 3], "the first copy is complete");
        assert_eq!(row_text(&snapshot, 1), "two");

        term.snapshot(&mut snapshot, 0);
        // Nothing printed: at most the cursor's row is reported.
        assert!(changed(&snapshot).iter().all(|&row| row == 2));

        feed(&mut term, b"ls");
        term.snapshot(&mut snapshot, 0);
        assert_eq!(changed(&snapshot), [2]);
        assert_eq!(row_text(&snapshot, 2), "$ ls");
        assert_eq!(row_text(&snapshot, 0), "one", "untouched rows keep their copy");
    }

    #[test]
    fn scrolling_and_clearing_history_recopy_everything() {
        let mut term = term(10, 3);
        let mut snapshot = ScreenSnapshot::default();
        feed(&mut term, b"1\r\n2\r\n3\r\n4\r\n5");
        term.snapshot(&mut snapshot, 2);
        assert_eq!(row_text(&snapshot, 0), "3");
        assert!(snapshot.has_content_above());
        let above: Vec<char> = snapshot.above().iter().map(|row| row[0].c).collect();
        assert_eq!(above, ['2', '1'], "nearest first");

        term.scroll_display(Scroll::Delta(1));
        term.snapshot(&mut snapshot, 2);
        assert_eq!(changed(&snapshot), [0, 1, 2]);
        assert_eq!(row_text(&snapshot, 0), "2");
        assert_eq!(snapshot.display_offset, 1);

        term.clear_history();
        term.snapshot(&mut snapshot, 2);
        assert_eq!(changed(&snapshot), [0, 1, 2], "the view moved without damage");
        assert_eq!(row_text(&snapshot, 0), "3");
        assert!(!snapshot.has_content_above());
        assert!(snapshot.above().is_empty());
    }

    #[test]
    fn clear_buffer_then_the_shells_ctrl_l_leaves_no_history_and_a_clean_screen() {
        // Clear Buffer (⌘K) empties the history and sends ^L; zsh answers ^L with these bytes
        // (captured from `zsh -f` on macOS) and redraws its prompt. The screen it cleared must not
        // be saved into the history that was just emptied.
        let mut term = term(10, 3);
        feed(&mut term, b"1\r\n2\r\n3\r\n4\r\n% ");
        assert!(term.history_size() > 0);
        term.clear_history();
        feed(&mut term, b"\x1b[H\x1b[2J\x1b[0m\x1b[27m\x1b[24m\x1b[J\x1b[K% ");
        assert_eq!(term.history_size(), 0);
        assert_eq!(term.screen_text().trim(), "%");
    }

    #[test]
    fn selection_spans_per_row() {
        let mut term = term(10, 3);
        feed(&mut term, b"aaaa\r\nbbbb\r\ncccc");
        term.start_selection(SelectionType::Simple, Point::new(Line(0), Column(2)), Side::Left);
        term.update_selection(Point::new(Line(1), Column(1)), Side::Right);
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot, 0);
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
        use crate::blocks::tests::{SESSION, shown};
        let (mut term, _) = shown(10, SESSION);
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot, 0);
        // Rows: `$ ls` / a.rs / b.rs / `$ make` / error / blank / `$ ` / blank …: the blank row
        // after `error` is the block's (a blank line it printed), the rows under the prompt nobody's.
        assert_eq!(spans(&snapshot), [(1, 0..3, true), (2, 3..6, true), (3, 6..7, true)]);
        assert_eq!(snapshot.block_at_row(5).map(|s| s.id), Some(2));
        assert_eq!(snapshot.block_at_row(8), None);
        // A full-screen program: no blocks.
        feed(&mut term, b"\x1b[?1049hvim");
        term.snapshot(&mut snapshot, 0);
        assert!(snapshot.block_spans().is_empty());
    }

    #[test]
    fn a_block_that_began_above_the_viewport_continues_at_its_top() {
        use crate::blocks::tests::{SESSION, shown};
        let (mut term, _) = shown(3, SESSION);
        let mut snapshot = ScreenSnapshot::default();
        // The live screen shows `error`, blank, `$ `: the first row is inside block 2.
        term.snapshot(&mut snapshot, 0);
        assert_eq!(spans(&snapshot), [(2, 0..2, false), (3, 2..3, true)]);
        // Scrolled up by one: `$ make` is the top row and starts its block in view.
        term.scroll_display(Scroll::Delta(1));
        term.snapshot(&mut snapshot, 0);
        assert_eq!(spans(&snapshot), [(2, 0..3, true)]);
        // Scrolled to the top: `$ ls` and its output.
        term.scroll_display(Scroll::Bottom);
        term.scroll_display(Scroll::Delta(4));
        term.snapshot(&mut snapshot, 0);
        assert_eq!(spans(&snapshot), [(1, 0..3, true)]);
    }

    #[test]
    fn without_shell_integration_no_rows_are_looked_at_for_blocks() {
        let mut term = term(10, 3);
        feed(&mut term, b"plain\r\noutput");
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot, 0);
        assert!(!snapshot.saw_marks && snapshot.block_spans().is_empty());
    }

    #[test]
    fn the_cursor_is_where_the_screen_shows_it() {
        let mut term = term(10, 3);
        feed(&mut term, "1\r\n2\r\n3\r\n日".as_bytes());
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot, 0);
        let cursor = snapshot.cursor.expect("cursor");
        assert_eq!((cursor.row, cursor.col, cursor.wide), (2, 2, false));
        // Scrolled up far enough, the cursor's row leaves the screen.
        term.scroll_display(Scroll::Delta(1));
        term.snapshot(&mut snapshot, 0);
        assert_eq!(snapshot.cursor, None);
    }

    #[test]
    fn a_rows_size_is_copied_with_it_and_damages_it() {
        use wormhole_vt::LineSize;
        let mut term = term(10, 4);
        feed(&mut term, b"one\r\ntwo\r\nthree");
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot, 0);
        assert!(snapshot.rows().iter().all(|row| row.size() == LineSize::Single));
        term.snapshot(&mut snapshot, 0);
        feed(&mut term, b"\x1b[2;1H\x1b#3\x1b[3;1H\x1b#4");
        term.snapshot(&mut snapshot, 0);
        assert_eq!(changed(&snapshot), [1, 2], "only the rows whose size changed");
        let sizes: Vec<LineSize> = snapshot.rows().iter().map(|row| row.size()).collect();
        assert_eq!(
            sizes,
            [LineSize::Single, LineSize::DoubleHeightTop, LineSize::DoubleHeightBottom, LineSize::Single]
        );
        // A row the size left stays as copied while the rows around it change.
        feed(&mut term, b"\x1b[4;1Hx");
        term.snapshot(&mut snapshot, 0);
        assert_eq!(snapshot.rows()[1].size(), LineSize::DoubleHeightTop);
    }

    /// What a full copy of a 200×60 screen of colored text costs (every row damaged: scrolling the
    /// view does it). Run with `cargo test --release -p wormhole-core full_copy -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore = "measurement; run with --release --ignored --nocapture"]
    fn full_copy_cost() {
        let text: Vec<u8> = (0..120)
            .flat_map(|i| {
                format!(
                    "\x1b[1;34m●\x1b[0m \x1b[38;2;{};90;200mline {i}\x1b[0m lorem ipsum {}\r\n",
                    i % 256,
                    "x".repeat(150)
                )
                .into_bytes()
            })
            .collect();
        // Every cell its own two true colors (the `wormhole` demo): nothing repeats.
        let blocks: Vec<u8> = (0..120usize)
            .flat_map(|row| {
                let mut line = String::new();
                for col in 0..200usize {
                    let (r, g, b) = ((row * 7 + col) % 256, (col * 13) % 256, (row * 29 + col * 3) % 256);
                    line.push_str(&format!("\x1b[38;2;{r};{g};{b};48;2;{b};{r};{g}m▀"));
                }
                line.push_str("\x1b[0m\r\n");
                line.into_bytes()
            })
            .collect();
        // Output of a shell with integration: every cell carries the output mark's link.
        let marked: Vec<u8> = (0..120)
            .flat_map(|i| {
                format!(
                    "\x1b]8;id=wormhole-prompt-{};wormhole:output\x07line {i} lorem ipsum {}\x1b]8;;\x07\r\n",
                    i / 10 + 1,
                    "x".repeat(150)
                )
                .into_bytes()
            })
            .collect();
        for (name, bytes) in
            [("colored text", text), ("true color blocks", blocks), ("marked output", marked)]
        {
            let mut term = term(200, 60);
            feed(&mut term, &bytes);
            let mut snapshot = ScreenSnapshot::default();
            term.snapshot(&mut snapshot, 3);
            let rounds = 20_000;
            let start = std::time::Instant::now();
            for round in 0..rounds {
                term.scroll_display(Scroll::Delta(if round % 2 == 0 { 1 } else { -1 }));
                term.snapshot(&mut snapshot, 3);
            }
            let per_copy = start.elapsed() / rounds;
            eprintln!(
                "full copy of 200x60, {name}: {per_copy:?} ({:.2} ns per cell)",
                per_copy.as_nanos() as f64 / 12_000.0
            );
        }
    }
}
