//! The screens' cells as text.
//!
//! The main screen is written the way it was made: its rows, history first, one after the other
//! with CR LF between them, so that the receiver scrolls the oldest into its own history as it
//! goes, and a row that wrapped continues straight into the next (nothing between them, so that
//! the receiver's autowrap makes the same soft wrap and a reflow later agrees). The other screen
//! has no history and is written by addressing. A cell nothing was written to is skipped with a
//! cursor movement or erased, never printed as a space: the receiver's cell stays what the
//! sender's is, blank.

use crate::grid::{Grid, RowView, Style};
use crate::types::{Color, Hyperlink, Line};

use super::{Gaps, Writer};

/// A run of cells nothing was written to, not yet put into the stream: they are skipped over,
/// or, when they have a background, erased.
#[derive(Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
    /// The background they were erased with; `None` for the default.
    bg: Option<Color>,
}

/// Whether a style is one an erase can leave on a blank: the default, or just a background.
fn erasable(style: &Style) -> bool {
    style.is_default() || *style == Style::erased(style.bg)
}

/// Whether the grid holds anything at all (a cell that is not the zero word).
pub(super) fn has_cells(grid: &Grid) -> bool {
    (0..grid.rows()).any(|y| {
        grid.row_view(Line(y as i32)).is_some_and(|view| view.cells().iter().any(|cell| !cell.is_blank()))
    })
}

impl<'a> Writer<'a> {
    /// Write the cells of `grid`, the main screen with its history or (`alt`) the other one.
    pub(super) fn screen_rows(&mut self, grid: &'a Grid, alt: bool) {
        let rows = grid.rows();
        let history = if alt { 0 } else { grid.history_len() };
        if !alt {
            self.history_lines = history;
        }
        let total = history + rows;
        let row_at = |n: usize| grid.row_view(Line(n as i32 - history as i32));
        let blank = |view: &RowView<'_>| !view.wrapped() && view.cells().iter().all(|cell| cell.is_blank());
        // Rows after the last one with anything in them only matter when the screen has to scroll
        // that far to push the history up.
        // A row that wraps needs the one it wraps into, however empty.
        let last_used = (0..total).rev().find_map(|n| {
            let view = row_at(n)?;
            if view.wrapped() { Some((n + 1).min(total - 1)) } else { (!blank(&view)).then_some(n) }
        });
        let end = match last_used {
            _ if history > 0 => total,
            Some(n) => n + 1,
            None => 0,
        };
        let mut continues = false;
        for n in 0..end {
            let Some(view) = row_at(n) else { continue };
            let last = n + 1 == total;
            let next = if last { None } else { row_at(n + 1) };
            let wraps = view.wrapped() && next.is_some();
            if view.wrapped() && next.is_none() {
                self.gaps |= Gaps::WRAP;
            }
            if alt && !continues {
                if blank(&view) {
                    continue;
                }
                self.csi2(n + 1, 1, b'H');
            }
            self.row(view, next.as_ref(), wraps, continues);
            continues = wraps;
            if !alt && n + 1 < end && !wraps {
                // A scroll fills the new line with the pen's background: keep it the default.
                if self.pen.style.bg != Style::DEFAULT.bg {
                    self.set_style(&Style::DEFAULT);
                }
                self.put(b"\r\n");
            }
        }
        self.clusters_on();
        if !alt {
            self.reset_pen();
        }
    }

    /// One row's cells. `wraps`: the line goes on in `next`, so the last cell must be printed and
    /// the cursor left where it is. `from_wrap`: the line came from the row before, and the first
    /// cell is what makes the receiver wrap.
    fn row(&mut self, view: RowView<'a>, next: Option<&RowView<'a>>, wraps: bool, from_wrap: bool) {
        let cells = view.cells();
        let cols = cells.len();
        let mut links = view.links().peekable();
        let mut span: Option<Span> = None;
        self.tabs.clear();
        // The blank the row before ended in, if this row is where its line wraps to.
        let mut owed = self.fix_wrap.take().filter(|_| from_wrap);
        // Where the receiver's cursor is on this row.
        let mut cx = 0usize;
        let mut x = 0usize;
        while x < cols {
            let cell = cells[x];
            if cell.is_spacer_tail() {
                x += 1;
                continue;
            }
            if cell.is_spacer_head() {
                self.flush(&mut span, &mut cx, cols);
                // The wide character on the next row prints this blank itself, from the last column.
                if !wraps || next.is_none_or(|n| !n.cells().first().is_some_and(|c| c.is_wide())) {
                    self.gaps |= Gaps::SPACER_HEAD;
                }
                self.move_to(&mut cx, x);
                x += 1;
                continue;
            }
            let style = view.style(cell.style_id());
            let plain = cell.codepoint() == 0 && !cell.has_grapheme() && cell.flags_but_tab() == 0;
            if plain && erasable(style) {
                // The first cell of a continued line is printed (a space the receiver erases
                // again), and the last cell of a line that goes on: that is what wraps.
                let bg = (!style.is_default()).then_some(style.bg);
                if from_wrap && x == 0 {
                    self.wrap_in();
                    self.set_style(&bg.map_or(Style::DEFAULT, Style::erased));
                    self.put(b"\x1b[D\x1b[X");
                    if cell.is_tab_start() {
                        self.put(b"\t\x1b[G");
                    }
                    self.unprint_before(&mut owed, 0, cols);
                    x += 1;
                    continue;
                }
                if wraps && x == cols - 1 {
                    // Printed to wrap from; made blank again once the line has wrapped.
                    self.fix_wrap = Some(bg);
                    if cell.is_tab_start() {
                        // HT marks no cell in the last column; only a reflow leaves such a mark.
                        self.gaps |= Gaps::WRAP;
                    }
                }
                if !(wraps && x == cols - 1) {
                    match &mut span {
                        Some(s) if s.bg == bg && s.end == x => s.end = x + 1,
                        _ => {
                            self.flush(&mut span, &mut cx, cols);
                            span = Some(Span { start: x, end: x + 1, bg });
                        }
                    }
                    if cell.is_tab_start() {
                        self.tabs.push(x);
                    }
                    x += 1;
                    continue;
                }
            }
            self.flush(&mut span, &mut cx, cols);
            while links.peek().is_some_and(|(_, end, _)| *end <= x) {
                links.next();
            }
            let link = links.peek().filter(|(start, _, _)| *start <= x).map(|(_, _, link)| *link);
            if cell.codepoint() == 0 && cell.has_grapheme() {
                // A mark on a cell with no character: the mark is printed from the next column.
                if from_wrap && x == 0 {
                    self.wrap_in();
                    self.put(b"\x1b[D\x1b[X");
                    self.unprint_before(&mut owed, 0, cols);
                    cx = 0;
                }
                if x + 1 < cols {
                    self.move_to(&mut cx, x + 1);
                    self.set_link(link);
                    for &mark in view.marks(x) {
                        self.char(mark);
                    }
                } else {
                    self.gaps |= Gaps::MARK;
                }
                x += 1;
                continue;
            }
            self.move_to(&mut cx, x);
            if from_wrap && x == 0 && style.bg != Style::DEFAULT.bg {
                // The wrap scrolls in a line with the pen's background: wrap with a plain space.
                self.wrap_in();
                self.put(b"\x1b[D\x1b[X");
            }
            self.put_cell(&view, x, link);
            let width = if cell.is_wide() { 2 } else { 1 };
            cx = x + width;
            x += width;
            if from_wrap && x == width {
                self.unprint_before(&mut owed, cx, cols);
            }
        }
        self.flush(&mut span, &mut cx, cols);
        // The blank the row before ended in stays printed if this row did not take it back.
        if owed.is_some() {
            self.gaps |= Gaps::WRAP;
        }
    }

    /// Print a space with the plain pen: the first character of a continued line, which makes the
    /// receiver wrap (and, at the bottom, scroll in a line the pen's background fills).
    fn wrap_in(&mut self) {
        self.set_style(&Style::DEFAULT);
        self.set_link(None);
        self.set_guarded(false);
        self.want_content(0, crate::grid::BlockPart::None);
        self.out.push(b' ');
    }

    /// The row before this one ended in a blank that was printed to wrap from, and the line has
    /// wrapped now (the cursor is at column `cx` of this row, which has its first cell): go back
    /// and delete that blank, which leaves an erased cell and keeps the wrap, and return.
    fn unprint_before(&mut self, owed: &mut Option<Option<crate::types::Color>>, cx: usize, cols: usize) {
        let Some(bg) = owed.take() else { return };
        if cx >= cols || self.term.rows < 2 {
            // A wrap pending cannot be kept through a movement; one row has no row above.
            self.gaps |= Gaps::WRAP;
            return;
        }
        self.set_style(&bg.map_or(Style::DEFAULT, Style::erased));
        self.put(b"\x1b[A");
        self.csi_count(cols, b'G');
        self.put(b"\x1b[P\x1b[B");
        self.csi_count(cx + 1, b'G');
    }

    /// Whether the receiver, clustering characters as mode 2027 says, makes this cell from its
    /// characters in `on` (`true`) or off mode.
    fn builds(base: char, marks: &[char], wide: bool, on: bool) -> bool {
        use crate::unicode::{GraphemeState, extend_width, width};
        if !on {
            return usize::from(width(base)) == if wide { 2 } else { 1 }
                && marks.iter().all(|&m| width(m) == 0);
        }
        let mut state = GraphemeState::new();
        let mut w = state.step(base).width;
        let mut last = base;
        for &m in marks {
            let step = state.step(m);
            if step.breaks {
                if step.width != 0 {
                    return false;
                }
                // A zero-width character that starts nothing is a mark of its own accord.
                state = GraphemeState::new();
                continue;
            }
            w = extend_width(w, last, m);
            last = m;
        }
        (w == 2) == wide && w > 0
    }

    /// Choose the mode 2027 the next cell is printed in: the one that makes the cell it has. The
    /// sender's own setting says nothing about how a cell came to be.
    fn cluster_mode(&mut self, cell: crate::grid::Cell, marks: &[char]) {
        let base = cell.character().unwrap_or(' ');
        if base.is_ascii() && marks.is_empty() {
            return;
        }
        let wide = cell.is_wide();
        let on = Self::builds(base, marks, wide, true);
        let want = if on {
            true
        } else if Self::builds(base, marks, wide, false) {
            false
        } else {
            self.gaps |= Gaps::CELL;
            true
        };
        if want != self.clusters {
            self.put(if want { b"\x1b[?2027h" } else { b"\x1b[?2027l" });
            self.clusters = want;
        }
    }

    /// Mode 2027 back on, as a receiver has it.
    pub(super) fn clusters_on(&mut self) {
        if !self.clusters {
            self.put(b"\x1b[?2027h");
            self.clusters = true;
        }
    }

    /// Print the cell at `x` of `view` with the pen it was written with.
    pub(super) fn put_cell(&mut self, view: &RowView<'a>, x: usize, link: Option<&'a Hyperlink>) {
        let cell = view.cells()[x];
        if self.term.last_char.is_none() {
            self.gaps |= Gaps::LAST_CHAR;
        }
        let marks: &[char] = if cell.has_grapheme() { view.marks(x) } else { &[] };
        self.cluster_mode(cell, marks);
        // The cell before was printed with the same look and nothing came between: the pen is
        // already right (the style is told apart by where its entry is, as its id is the page's).
        let style = view.style(cell.style_id());
        let key = (
            std::ptr::from_ref(style) as usize,
            cell.flags(),
            view.block(),
            link.map_or(0, |link| std::ptr::from_ref(link) as usize),
        );
        if self.text_end != self.out.len() || key != self.pen_key {
            self.set_style(style);
            self.set_link(link);
            self.set_protected(cell.is_protected());
            self.set_guarded(cell.is_guarded());
            self.want_content(view.block(), cell.content());
            self.pen_key = key;
        }
        let c = cell.character().unwrap_or(' ');
        let rich = !c.is_ascii() || !marks.is_empty();
        // Two cells printed one after the other could join into one cluster in the receiver (a
        // joiner, a flag's second half, a prepended mark): a NUL, which does nothing, ends the
        // cluster between them.
        if self.clusters && (rich || self.text_rich) && self.text_end == self.out.len() {
            self.out.push(0);
        }
        self.char(c);
        for &mark in marks {
            self.char(mark);
        }
        self.text_end = self.out.len();
        self.text_rich = rich;
    }

    /// Move the receiver's cursor right to column `x` of the row.
    fn move_to(&mut self, cx: &mut usize, x: usize) {
        if *cx < x {
            self.csi_count(x - *cx, b'C');
            *cx = x;
        }
    }

    /// Put a span of unwritten cells into the stream: erased if they have a background, skipped
    /// over, with a tab mark where HT started on one.
    fn flush(&mut self, span: &mut Option<Span>, cx: &mut usize, cols: usize) {
        let Some(Span { start, end, bg }) = span.take() else { return };
        self.move_to(cx, start);
        if let Some(bg) = bg {
            self.set_style(&Style::erased(bg));
            self.csi_count(end - start, b'X');
        }
        // HT marks the blank cell it starts on, and goes to the next stop, from where the cursor is
        // brought back; only cells before the last column are ever marked.
        let tabs = std::mem::take(&mut self.tabs);
        for &t in &tabs {
            self.move_to(cx, t);
            self.put(b"\t");
            // Back to the column after the marked one (CHA counts from 1).
            self.csi_count(t + 2, b'G');
            *cx = t + 1;
        }
        self.tabs = tabs;
        self.tabs.clear();
        if end < cols {
            self.move_to(cx, end);
        } else {
            *cx = (*cx).max(start);
        }
    }

    /// One character of text.
    pub(super) fn char(&mut self, c: char) {
        if c.is_ascii() {
            self.out.push(c as u8);
        } else {
            let mut buf = [0u8; 4];
            self.out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
}
