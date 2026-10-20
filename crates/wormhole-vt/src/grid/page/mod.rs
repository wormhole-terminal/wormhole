//! A page: a block of rows with their own style table and side tables.
//!
//! A page owns `capacity` row slots of `columns` cells each, and a list of [`Row`] records saying
//! which slot is which row, in order. Slots are handed out in order and a row is moved by moving its
//! record, never its cells. Rows are appended at the end, so a new row's cells are already blank (a
//! slot freed by cutting rows off the end is given back; one freed from the front stays dead, zeroed,
//! until the page goes). A page is dropped whole when all its rows fall out of the scrollback, which
//! frees its styles and side tables with it; nothing is counted down cell by cell.
//!
//! Every cell mutation in the grid, and every change to a row's record (wrapped, block, changed),
//! is one of the methods here. They keep three things true:
//!
//! - **Counts.** The style table counts the cells that use each style, the link table the cells
//!   under each link; a cell is released before it is overwritten.
//! - **Side tables.** A cell has an entry in the grapheme or link table exactly when its flag says
//!   so, and the entry's key is the cell's offset in the page.
//! - **Wide characters.** A `WIDE` cell is followed by its `SPACER_TAIL` and a tail follows a
//!   wide cell, always: a write, an erase, a shift or a copy that would cut a pair blanks the half
//!   that is left (keeping its style, so the background stays).
//!
//! Offsets are `slot * columns + column`. A page holds fewer than 65,535 cells, which bounds both
//! the style ids and the `u32` keys.
//!
//! Image placements are not a side table of the page: a picture spans rows, and so pages, and
//! scrolls with the text, so the grid keeps them by row serial (`images`). The cells under a
//! picture are ordinary cells, and the `IMAGE` flag is unused.

use std::ops::Range;

use crate::types::Hyperlink;

use super::cell::{Cell, CellFlags, Spare, StyleRun, run_of};
use super::row::{DIRTY, Pen, Row, RowMark, WRAPPED};
use super::side_tables::{Graphemes, Links};
use super::style::Style;
use super::style_table::StyleTable;

mod edit;
mod rect;
#[cfg(test)]
mod tests;

pub(super) use rect::Copied;

pub(super) struct Page {
    /// Unique among the pages of a grid, never reused: a cache keyed by it can't mistake a new
    /// page for the old one.
    pub(super) uid: u64,
    /// The serial number of the first row (see `Grid`).
    pub(super) base: u64,
    cols: usize,
    capacity: usize,
    /// Slots handed out so far; `cells.len() == next_slot * cols`.
    next_slot: usize,
    cells: Vec<Cell>,
    pub(super) rows: Vec<Row>,
    pub(super) styles: StyleTable,
    pub(super) graphemes: Graphemes,
    pub(super) links: Links,
    /// The bytes of the marks and links, measured once the page holds only history (see
    /// [`Page::extra_bytes`]); `None` until then, or when it may have changed since.
    extra: Option<usize>,
}

impl Page {
    pub(super) fn new(uid: u64, base: u64, cols: usize, capacity: usize) -> Page {
        debug_assert!(cols * capacity < 65_535 && cols > 0 && capacity > 0);
        Page {
            uid,
            base,
            cols,
            capacity,
            next_slot: 0,
            cells: Vec::with_capacity(cols * capacity),
            rows: Vec::with_capacity(capacity),
            styles: StyleTable::default(),
            graphemes: Graphemes::default(),
            links: Links::default(),
            extra: None,
        }
    }

    /// A page without memory, to leave behind while the real one is taken out.
    pub(super) fn empty() -> Page {
        Page {
            uid: 0,
            base: 0,
            cols: 0,
            capacity: 0,
            next_slot: 0,
            cells: Vec::new(),
            rows: Vec::new(),
            styles: StyleTable::unallocated(),
            graphemes: Graphemes::default(),
            links: Links::default(),
            extra: None,
        }
    }

    /// Empty the page for reuse as a new one, keeping every allocation.
    pub(super) fn recycle(&mut self, uid: u64, base: u64) {
        self.uid = uid;
        self.base = base;
        self.next_slot = 0;
        self.cells.clear();
        self.rows.clear();
        self.styles.clear();
        self.graphemes.clear();
        self.links.clear();
        self.extra = None;
    }

    #[inline]
    pub(super) fn cols(&self) -> usize {
        self.cols
    }

    #[inline]
    pub(super) fn capacity(&self) -> usize {
        self.capacity
    }

    /// Whether no more rows can be appended.
    #[inline]
    pub(super) fn is_full(&self) -> bool {
        self.next_slot >= self.capacity
    }

    /// Slots handed out (rows ever appended here, less none: dead slots count).
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn slots_used(&self) -> usize {
        self.next_slot
    }

    /// Bytes this page holds.
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn bytes(&self) -> usize {
        self.cells.len() * size_of::<Cell>()
            + self.rows.capacity() * size_of::<Row>()
            + self.styles.bytes()
            + self.graphemes.bytes()
            + self.links.bytes()
    }

    /// The bytes of the marks and the links, which a program can make much larger than the cells
    /// they are on (32 marks a cell, a link of 8 KiB a cell). Measured once and kept: a page of
    /// history does not change until a resize, which calls [`Page::forget_extra_bytes`].
    pub(super) fn extra_bytes(&mut self) -> usize {
        if let Some(bytes) = self.extra {
            return bytes;
        }
        let bytes = self.graphemes.bytes() + self.links.bytes();
        self.extra = Some(bytes);
        bytes
    }

    /// The page may change again (its rows are back on screen, or it lost some).
    pub(super) fn forget_extra_bytes(&mut self) {
        self.extra = None;
    }

    /// Append a blank row; its index.
    pub(super) fn push_row(&mut self) -> usize {
        debug_assert!(!self.is_full());
        let slot = self.next_slot;
        self.next_slot += 1;
        self.cells.resize(self.next_slot * self.cols, Cell::EMPTY);
        self.rows.push(Row::new(slot));
        self.rows.len() - 1
    }

    #[inline]
    fn base_of(&self, row: usize) -> usize {
        self.rows[row].slot as usize * self.cols
    }

    /// The cells of row `row`.
    #[inline]
    pub(super) fn cells(&self, row: usize) -> &[Cell] {
        let base = self.base_of(row);
        &self.cells[base..base + self.cols]
    }

    /// The offset side-table keys use for the cell at `col` of `row`.
    #[inline]
    pub(super) fn offset(&self, row: usize, col: usize) -> u32 {
        (self.base_of(row) + col) as u32
    }

    /// The block mark of row `row`, read from its cells: which parts of the block it holds.
    pub(super) fn row_mark(&self, row: usize) -> Option<RowMark> {
        let block = self.rows[row].block;
        if block == 0 {
            return None;
        }
        let parts = self.cells(row).iter().fold(0u8, |parts, cell| parts | 1 << cell.content() as u8) & !1;
        RowMark::from_parts(block, parts)
    }

    /// Every cell of the buffer, dead slots included (the checker's view).
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn all_cells(&self) -> &[Cell] {
        &self.cells
    }

    #[inline]
    pub(super) fn touch(&mut self, row: usize) {
        self.rows[row].flags |= DIRTY;
    }

    /// Mark rows `rows` as changed.
    pub(super) fn touch_rows(&mut self, rows: Range<usize>) {
        for record in &mut self.rows[rows] {
            record.flags |= DIRTY;
        }
    }

    /// Whether the snapshot has yet to copy `row`; clears the mark.
    #[inline]
    pub(super) fn take_dirty(&mut self, row: usize) -> bool {
        let flags = &mut self.rows[row].flags;
        let dirty = *flags & DIRTY != 0;
        *flags &= !DIRTY;
        dirty
    }

    pub(super) fn set_wrapped(&mut self, row: usize, wrapped: bool) {
        let flags = &mut self.rows[row].flags;
        if wrapped {
            *flags |= WRAPPED | DIRTY;
        } else {
            *flags &= !WRAPPED;
            *flags |= DIRTY;
        }
    }

    /// The command block (OSC 133) of `row`, 0 for none.
    #[inline]
    pub(super) fn block(&self, row: usize) -> u32 {
        self.rows[row].block
    }

    /// Put `row` on command block `block`.
    pub(super) fn set_block(&mut self, row: usize, block: u32) {
        self.rows[row].block = block;
    }

    /// Whether `row` is soft-wrapped and its block: what a row copied over another takes along.
    pub(super) fn meta(&self, row: usize) -> (bool, u32) {
        let record = &self.rows[row];
        (record.wrapped(), record.block)
    }

    /// Give `row` the wrap flag and block of a row that was copied over it, and mark it changed.
    pub(super) fn set_meta(&mut self, row: usize, (wrapped, block): (bool, u32)) {
        self.set_wrapped(row, wrapped);
        self.rows[row].block = block;
    }

    // ---- releasing and filling ----

    /// Give back what the cells in `a..b` (absolute offsets) hold: their count in the style
    /// table, their side-table entries. The cells themselves are left as they are for the caller to
    /// overwrite.
    fn release(&mut self, a: usize, b: usize) {
        let Page { cells, styles, graphemes, links, .. } = self;
        let mut run = (0u16, 0u32);
        let mut linked = false;
        for (i, cell) in cells[a..b].iter().enumerate() {
            let id = cell.style_id();
            if id != run.0 {
                styles.release(run.0, run.1);
                run = (id, 0);
            }
            run.1 += 1;
            if cell.has_side_entry() {
                let offset = (a + i) as u32;
                if cell.has_grapheme() {
                    graphemes.remove(offset);
                }
                linked |= cell.has_hyperlink();
            }
        }
        styles.release(run.0, run.1);
        if linked {
            links.detach(a as u32, b as u32);
        }
    }

    /// Give back what the cells in `a..b` hold before they are overwritten. One pass over the words
    /// (an OR and an AND, which vectorize) tells what is there: nothing to give back, one style
    /// throughout (a single count to lower, however many cells), or a mixture, which takes the
    /// cell-by-cell walk.
    #[inline]
    fn release_range(&mut self, a: usize, b: usize) {
        match self.uniform_style(a, b) {
            StyleRun::Plain => {}
            StyleRun::Uniform(id) => self.styles.release(id, (b - a) as u32),
            StyleRun::Mixed => self.release(a, b),
        }
    }

    /// What the cells in `a..b` (absolute offsets) hold that must be counted: nothing, one style
    /// throughout, or a mixture (or entries in a side table).
    #[inline]
    fn uniform_style(&self, a: usize, b: usize) -> StyleRun {
        run_of(&self.cells[a..b])
    }

    /// Make `col` a place no wide character straddles: if it is the tail of one, blank both halves
    /// (keeping their style, so a background stays). `base` is the row's first offset.
    #[inline]
    fn split_wide_at(&mut self, base: usize, col: usize) {
        if col > 0 && col < self.cols && self.cells[base + col].is_spacer_tail() {
            self.split_wide_slow(base + col);
        }
    }

    #[cold]
    fn split_wide_slow(&mut self, tail: usize) {
        self.blank_keeping_style(tail - 1);
        self.blank_keeping_style(tail);
    }

    /// Turn the cell at absolute offset `at` into an empty one of the same style.
    fn blank_keeping_style(&mut self, at: usize) {
        let cell = self.cells[at];
        if cell.has_side_entry() {
            if cell.has_grapheme() {
                self.graphemes.remove(at as u32);
            }
            if cell.has_hyperlink() {
                self.links.detach(at as u32, at as u32 + 1);
            }
        }
        self.cells[at] = Cell::compose(0, cell.style_id(), CellFlags::empty());
    }

    /// Both ends of a range: the places where it must not cut a wide character.
    #[inline]
    fn guard_edges(&mut self, base: usize, from: usize, to: usize) {
        self.split_wide_at(base, from);
        self.split_wide_at(base, to);
    }

    /// As [`Self::guard_edges`] for an erase that leaves the cells `spare` names alone: a wide
    /// character those protect is left whole where the range cuts it (both halves carry the
    /// protection, so the erase passes over the half inside).
    pub(super) fn guard_edges_sparing(&mut self, base: usize, from: usize, to: usize, spare: Spare) {
        for col in [from, to] {
            if col > 0 && col < self.cols && !self.cells[base + col - 1].is_spared(spare) {
                self.split_wide_at(base, col);
            }
        }
    }

    // ---- writing ----

    /// Write printable ASCII `bytes` into row `row` from column `col`: the hot path. The caller
    /// keeps them inside the row.
    #[inline]
    pub(super) fn write_ascii(&mut self, row: usize, col: usize, bytes: &[u8], pen: &Pen) {
        debug_assert!(col + bytes.len() <= self.cols && bytes.is_ascii());
        let n = bytes.len();
        if n == 0 {
            return;
        }
        let base = self.base_of(row);
        let (a, b) = (base + col, base + col + n);
        self.guard_edges(base, col, col + n);
        // Count the new style before giving the old back: the same style never drops to zero in
        // between, so rewriting a line in the color it has costs two additions.
        let style = self.styles.acquire(&pen.style, n as u32);
        self.release_range(a, b);
        let mut flags = pen.cell_flags();
        if let Some(link) = &pen.link
            && self.attach_link(a, n, link)
        {
            flags |= CellFlags::HYPERLINK;
        }
        let template = Cell::compose(0, style, flags);
        for (cell, &byte) in self.cells[a..b].iter_mut().zip(bytes) {
            *cell = template.with_codepoint(byte as u32);
        }
        self.note_written(row, pen);
    }

    #[inline]
    fn note_written(&mut self, row: usize, pen: &Pen) {
        let record = &mut self.rows[row];
        record.flags |= DIRTY;
        if pen.content as u8 != 0 {
            record.block = pen.block;
        }
    }

    /// Put the `n` cells from `a` under `link`; false when the page has no room for another link's
    /// text (`side_tables::LINK_TEXT_MAX`) and they stay without one.
    pub(super) fn attach_link(&mut self, a: usize, n: usize, link: &Hyperlink) -> bool {
        if !self.links.has_room_for(link) {
            return false;
        }
        let id = self.links.acquire(link, n as u32);
        self.links.attach(a as u32, n as u32, id);
        true
    }

    /// Write `c` of `width` 1 or 2 at `col` of `row`, with the marks that follow it in its cluster.
    /// A wide character takes the next cell as well, which the caller has made sure exists.
    pub(super) fn write_char(
        &mut self,
        row: usize,
        col: usize,
        c: char,
        marks: &[char],
        width: u8,
        pen: &Pen,
    ) {
        let n = width as usize;
        debug_assert!((1..=2).contains(&n) && col + n <= self.cols);
        let base = self.base_of(row);
        let (a, b) = (base + col, base + col + n);
        self.guard_edges(base, col, col + n);
        let style = self.styles.acquire(&pen.style, n as u32);
        self.release_range(a, b);
        let mut flags = pen.cell_flags();
        if let Some(link) = &pen.link
            && self.attach_link(a, n, link)
        {
            flags |= CellFlags::HYPERLINK;
        }
        let mut head = flags;
        if !marks.is_empty() {
            head |= CellFlags::GRAPHEME;
            self.graphemes.set_from(a as u32, marks);
        }
        if n == 2 {
            self.cells[a] = Cell::compose(c as u32, style, head | CellFlags::WIDE);
            self.cells[a + 1] = Cell::compose(0, style, flags | CellFlags::SPACER_TAIL);
        } else {
            self.cells[a] = Cell::compose(c as u32, style, head);
        }
        self.note_written(row, pen);
    }

    /// Put a spacer head in the last cell of `row`: the wide character that didn't fit goes to the
    /// next row.
    pub(super) fn write_spacer_head(&mut self, row: usize, pen: &Pen) {
        let last = self.cols - 1;
        let base = self.base_of(row);
        self.split_wide_at(base, last);
        self.release_range(base + last, base + last + 1);
        let style = self.styles.acquire(&Style::erased(pen.style.bg), 1);
        self.cells[base + last] = Cell::compose(0, style, CellFlags::SPACER_HEAD);
        self.touch(row);
    }

    /// Add `c` to the cluster of the cell at `col` (of its head, when `col` is a wide character's
    /// tail). False when there is nothing to attach to or the cell is full of marks.
    pub(super) fn push_mark(&mut self, row: usize, col: usize, c: char) -> bool {
        let base = self.base_of(row);
        let mut at = base + col;
        if self.cells[at].is_spacer_tail() && col > 0 {
            at -= 1;
        }
        let cell = self.cells[at];
        if cell.is_spacer_head() {
            return false;
        }
        if !self.graphemes.push(at as u32, c) {
            return false;
        }
        self.cells[at] = cell.with_flags_set(CellFlags::GRAPHEME);
        self.touch(row);
        true
    }

    /// Make the character at `col` one cell wide or two (a variation selector changed its
    /// presentation). Widening needs the next cell, which it takes over; false when there is none.
    pub(super) fn set_width(&mut self, row: usize, col: usize, width: u8) -> bool {
        let base = self.base_of(row);
        let at = base + col;
        let cell = self.cells[at];
        if cell.is_spacer_tail() || cell.is_spacer_head() {
            return false;
        }
        match (width, cell.is_wide()) {
            (2, false) => {
                if col + 1 >= self.cols {
                    return false;
                }
                self.guard_edges(base, col + 1, col + 2);
                self.release_range(at + 1, at + 2);
                let style = cell.style_id();
                self.styles.retain(style, 1);
                // The tail is part of the same content and as protected, and under the same link.
                let tail_flags = (cell.flag_set() & (CellFlags::PROTECTED | CellFlags::GUARDED))
                    | CellFlags::with_content(cell.content())
                    | CellFlags::SPACER_TAIL;
                if cell.has_hyperlink() {
                    if let Some(id) = self.links.id_at(at as u32) {
                        self.links.retain(id, 1);
                        self.links.attach(at as u32 + 1, 1, id);
                        self.cells[at + 1] = Cell::compose(0, style, tail_flags | CellFlags::HYPERLINK);
                    }
                } else {
                    self.cells[at + 1] = Cell::compose(0, style, tail_flags);
                }
                self.cells[at] = cell.with_flags_set(CellFlags::WIDE);
            }
            (1, true) => {
                self.blank_keeping_style(at + 1);
                self.cells[at] = cell.without_flags(CellFlags::WIDE);
            }
            _ => return true,
        }
        self.touch(row);
        true
    }

    /// One cell with everything given outright (the rewrap and the copies between pages build rows
    /// this way). `style` is looked up in this page's table.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn put(
        &mut self,
        row: usize,
        col: usize,
        c: Option<char>,
        marks: &[char],
        style: &Style,
        link: Option<&Hyperlink>,
        flags: CellFlags,
    ) {
        let at = self.base_of(row) + col;
        self.release_range(at, at + 1);
        let id = self.styles.acquire(style, 1);
        let mut flags = flags - CellFlags::SIDE_ENTRY;
        if !marks.is_empty() {
            flags |= CellFlags::GRAPHEME;
            self.graphemes.set_from(at as u32, marks);
        }
        if let Some(link) = link
            && self.attach_link(at, 1, link)
        {
            flags |= CellFlags::HYPERLINK;
        }
        self.cells[at] = Cell::compose(c.map_or(0, |c| c as u32), id, flags);
    }
}
