//! The page's editing operations: erasing, shifting cells inside a row, copying cells between
//! rows, and moving or dropping whole rows. The cell-level invariants are the ones in `mod.rs`.

use std::ops::Range;

use crate::types::Color;

use super::Page;
use crate::grid::cell::{Cell, CellFlags, Spare, StyleRun};
use crate::grid::row::{DIRTY, WRAPPED};
use crate::grid::style::Style;

impl Page {
    // ---- erasing ----

    /// Blank `cols` of `row` with `bg` as their background, but for the cells `spare` leaves alone.
    /// Erasing up to the last column ends the row's soft wrap: the line doesn't continue.
    pub(in crate::grid) fn erase(&mut self, row: usize, cols: Range<usize>, bg: Color, spare: Spare) {
        let Range { start, end } = cols;
        if start >= end {
            return;
        }
        let base = self.base_of(row);
        self.guard_edges_sparing(base, start, end, spare);
        let (a, b) = (base + start, base + end);
        if spare != Spare::Nothing {
            self.erase_unspared(a, b, bg, spare);
        } else {
            let id = self.styles.acquire(&Style::erased(bg), (b - a) as u32);
            self.release_range(a, b);
            self.cells[a..b].fill(Cell::compose(0, id, CellFlags::empty()));
        }
        let record = &mut self.rows[row];
        record.flags |= DIRTY;
        if end == self.cols {
            record.flags &= !WRAPPED;
        }
        if start == 0 && end == self.cols {
            record.block = 0;
        }
    }

    fn erase_unspared(&mut self, a: usize, b: usize, bg: Color, spare: Spare) {
        let erased = Style::erased(bg);
        for at in a..b {
            if self.cells[at].is_spared(spare) {
                continue;
            }
            self.release(at, at + 1);
            let id = self.styles.acquire(&erased, 1);
            self.cells[at] = Cell::compose(0, id, CellFlags::empty());
        }
    }

    /// HT started on `col` of `row`: mark the cell when it holds nothing, so a copy of the text can
    /// give the gap as a tab. The mark is invisible, so the row is not dirty.
    pub(in crate::grid) fn mark_tab(&mut self, row: usize, col: usize) {
        let at = self.base_of(row) + col;
        let cell = self.cells[at];
        if cell.codepoint() == 0 && cell.flag_set().is_empty() {
            self.cells[at] = cell.with_flags_set(CellFlags::TAB);
        }
    }

    /// Blank the whole row and forget what the terminal knew about the line.
    pub(in crate::grid) fn clear_row(&mut self, row: usize, bg: Color) {
        let n = self.cols;
        self.erase(row, 0..n, bg, Spare::Nothing);
        self.rows[row].flags = DIRTY;
        self.rows[row].block = 0;
    }

    // ---- shifting cells within a row ----

    /// A spacer head only means something at the end of a row; a shift that includes the last
    /// cell makes it an ordinary blank.
    fn forget_spacer_head(&mut self, base: usize, right: usize) {
        let last = base + self.cols - 1;
        if right + 1 == self.cols && self.cells[last].is_spacer_head() {
            self.cells[last] = self.cells[last].without_flags(CellFlags::SPACER_HEAD);
        }
    }

    /// ICH: open `n` blank cells at `col`, pushing the cells from there to `right` along; those
    /// pushed past `right` are lost.
    pub(in crate::grid) fn insert_cells(
        &mut self,
        row: usize,
        col: usize,
        right: usize,
        n: usize,
        bg: Color,
    ) {
        if col > right || right >= self.cols {
            return;
        }
        let n = n.min(right - col + 1);
        let base = self.base_of(row);
        self.forget_spacer_head(base, right);
        let kept_end = right + 1 - n;
        self.split_wide_at(base, col);
        self.split_wide_at(base, right + 1);
        if kept_end > col {
            self.split_wide_at(base, kept_end);
        }
        self.release_range(base + kept_end, base + right + 1);
        if kept_end > col {
            self.move_cells(base + col..base + kept_end, base + col + n);
        }
        self.fill_erased(base + col, base + col + n, bg);
        self.touch(row);
    }

    /// DCH: delete `n` cells at `col`, pulling the rest up to `right` left; blank cells enter at
    /// `right`.
    pub(in crate::grid) fn delete_cells(
        &mut self,
        row: usize,
        col: usize,
        right: usize,
        n: usize,
        bg: Color,
    ) {
        if col > right || right >= self.cols {
            return;
        }
        let n = n.min(right - col + 1);
        let base = self.base_of(row);
        self.forget_spacer_head(base, right);
        self.split_wide_at(base, col);
        if col + n <= right {
            self.split_wide_at(base, col + n);
        }
        self.split_wide_at(base, right + 1);
        self.release_range(base + col, base + col + n);
        if col + n <= right {
            self.move_cells(base + col + n..base + right + 1, base + col);
        }
        self.fill_erased(base + right + 1 - n, base + right + 1, bg);
        self.touch(row);
    }

    /// Fill `a..b` with erased cells; whatever the cells held was moved or released already.
    fn fill_erased(&mut self, a: usize, b: usize, bg: Color) {
        let id = self.styles.acquire(&Style::erased(bg), (b - a) as u32);
        self.cells[a..b].fill(Cell::compose(0, id, CellFlags::empty()));
    }

    /// Move the cells `from` to start at `to` (overlap allowed), carrying their side-table entries.
    /// The counts don't change: the cells are the same, and the ones they leave behind are
    /// overwritten by the caller without being released.
    fn move_cells(&mut self, from: Range<usize>, to: usize) {
        let any = self.cells[from.clone()].iter().fold(0u64, |acc, cell| acc | cell.bits());
        if CellFlags::in_word(any).intersects(CellFlags::GRAPHEME | CellFlags::HYPERLINK) {
            // Walk away from the direction of the move, so a destination is free when it is reached.
            let len = from.len();
            for k in 0..len {
                let at = if to > from.start { from.end - 1 - k } else { from.start + k };
                let cell = self.cells[at];
                let target = to + (at - from.start);
                if cell.has_grapheme() {
                    self.graphemes.relocate(at as u32, target as u32);
                }
            }
            if CellFlags::in_word(any).contains(CellFlags::HYPERLINK) {
                self.links.relocate(from.start as u32..from.end as u32, to as u32);
            }
        }
        self.cells.copy_within(from, to);
    }

    // ---- copying cells between rows ----

    /// Copy `cols` of `src_row` of this page over the same columns of `dst_row`.
    pub(in crate::grid) fn copy_within_page(&mut self, src_row: usize, dst_row: usize, cols: Range<usize>) {
        if cols.is_empty() || src_row == dst_row {
            return;
        }
        let (sbase, dbase) = (self.base_of(src_row), self.base_of(dst_row));
        let (start, end) = (cols.start, cols.end);
        self.guard_edges(dbase, start, end);
        let (da, db) = (dbase + start, dbase + end);
        self.release_range(da, db);
        self.cells.copy_within(sbase + start..sbase + end, da);
        // The copied cells use their styles and links once more: one count for a uniform range.
        match self.uniform_style(da, db) {
            StyleRun::Plain => {}
            StyleRun::Uniform(id) => self.styles.retain(id, (db - da) as u32),
            StyleRun::Mixed => self.retain_copied(sbase + start, da, db - da),
        }
        self.mend_copied_edges(da, db);
    }

    /// The cells at `to..to+n` were copied from `from..`: count their styles and links, and copy
    /// their marks.
    fn retain_copied(&mut self, from: usize, to: usize, n: usize) {
        let mut run = (0u16, 0u32);
        let mut linked = false;
        for i in 0..n {
            let cell = self.cells[to + i];
            if cell.style_id() != run.0 {
                self.styles.retain(run.0, run.1);
                run = (cell.style_id(), 0);
            }
            run.1 += 1;
            if cell.has_side_entry() {
                let (src, dst) = ((from + i) as u32, (to + i) as u32);
                if cell.has_grapheme() {
                    self.graphemes.copy(src, dst);
                }
                linked |= cell.has_hyperlink();
            }
        }
        self.styles.retain(run.0, run.1);
        if linked {
            self.links.copy(from as u32, to as u32, n as u32);
        }
    }

    /// Copy `cols` of `src_row` of another page over the same columns of `dst_row` here, carrying
    /// styles over by value into this page's table.
    pub(in crate::grid) fn copy_from_page(
        &mut self,
        src: &Page,
        src_row: usize,
        dst_row: usize,
        cols: Range<usize>,
    ) {
        if cols.is_empty() {
            return;
        }
        let sbase = src.base_of(src_row);
        let dbase = self.base_of(dst_row);
        let (start, end) = (cols.start, cols.end);
        self.guard_edges(dbase, start, end);
        let (da, db) = (dbase + start, dbase + end);
        self.release_range(da, db);
        let source = &src.cells[sbase + start..sbase + end];
        let mut i = 0;
        while i < source.len() {
            let id = source[i].style_id();
            let run = source[i..].iter().take_while(|cell| cell.style_id() == id).count();
            let mapped = if id == 0 { 0 } else { self.styles.acquire(src.styles.get(id), run as u32) };
            for (k, cell) in source[i..i + run].iter().enumerate() {
                let at = da + i + k;
                self.cells[at] = cell.with_style(mapped);
                if cell.has_side_entry() {
                    let from = (sbase + start + i + k) as u32;
                    if cell.has_grapheme() {
                        self.graphemes.set_from(at as u32, src.graphemes.get(from));
                    }
                }
            }
            i += run;
        }
        // The links, run by run: the copied cells are under the same links as the source's.
        let (sa, sb) = ((sbase + start) as u32, (sbase + end) as u32);
        for (lo, hi, id) in src.links.within(sa, sb) {
            let (at, n) = (da + (lo - sa) as usize, (hi - lo) as usize);
            if !self.attach_link(at, n, src.links.link(id)) {
                for cell in &mut self.cells[at..at + n] {
                    *cell = cell.without_flags(CellFlags::HYPERLINK);
                }
            }
        }
        self.mend_copied_edges(da, db);
    }

    /// A copied range that begins with a tail or ends with a head has lost the other half: blank it.
    /// A spacer head that ended up anywhere but the last cell of a row (a row cropped wider) is only
    /// a blank.
    pub(super) fn mend_copied_edges(&mut self, a: usize, b: usize) {
        if self.cells[a].is_spacer_tail() {
            self.blank_keeping_style(a);
        }
        let last = self.cells[b - 1];
        if last.is_wide() || (last.is_spacer_head() && !b.is_multiple_of(self.cols)) {
            self.blank_keeping_style(b - 1);
        }
    }

    // ---- moving rows ----

    /// Rotate the records of rows `range` so that the first `n` come last (scrolling up).
    pub(in crate::grid) fn rotate_rows_up(&mut self, range: Range<usize>, n: usize) {
        self.rows[range].rotate_left(n);
    }

    /// Rotate the records of rows `range` so that the last `n` come first (scrolling down).
    pub(in crate::grid) fn rotate_rows_down(&mut self, range: Range<usize>, n: usize) {
        self.rows[range].rotate_right(n);
    }

    /// Drop the first `n` rows: release their cells and zero them (their slots stay dead).
    pub(in crate::grid) fn drop_front_rows(&mut self, n: usize) {
        for row in 0..n {
            let base = self.base_of(row);
            let (a, b) = (base, base + self.cols);
            self.release_range(a, b);
            self.cells[a..b].fill(Cell::EMPTY);
        }
        self.rows.drain(..n);
        self.base += n as u64;
        self.forget_extra_bytes();
    }

    /// Drop the last `n` rows the same way. Their slots stay dead, but the newest slots can be
    /// handed out again when they were the last.
    pub(in crate::grid) fn drop_back_rows(&mut self, n: usize) {
        let keep = self.rows.len() - n;
        for row in keep..self.rows.len() {
            let base = self.base_of(row);
            let (a, b) = (base, base + self.cols);
            self.release_range(a, b);
            self.cells[a..b].fill(Cell::EMPTY);
        }
        // Take the slots back when they sit at the end of the buffer, so that a screen that
        // shrinks and grows again doesn't use the page up.
        let mut dead: Vec<usize> = self.rows[keep..].iter().map(|row| row.slot as usize).collect();
        self.rows.truncate(keep);
        dead.sort_unstable();
        let mut top = self.next_slot;
        while dead.pop_if(|last| *last + 1 == top).is_some() {
            top -= 1;
        }
        self.next_slot = top;
        self.cells.truncate(top * self.cols);
    }

    /// Whether row `row` holds nothing but blank cells and no style (fresh memory, or tab marks).
    pub(in crate::grid) fn row_is_blank(&self, row: usize) -> bool {
        self.cells(row).iter().all(|cell| cell.is_blank_but_tab())
    }
}
