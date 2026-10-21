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
//! is one of the methods of this folder (`edit`, `rect` and `write` are its parts). They keep three
//! things true:
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

use crate::types::LineSize;

use super::cell::{Cell, CellFlags, Spare, StyleRun, run_of};
use super::graphemes::Graphemes;
use super::link_table::Links;
use super::row::{DIRTY, Meta, Row, RowMark, SIZE_MASK, SIZE_SHIFT, WRAPPED};
use super::style_table::StyleTable;

mod edit;
mod rect;
#[cfg(test)]
mod tests;
mod write;

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

    /// The bytes of the marks, the links and the styles, which a program can make much larger than
    /// the cells they are on (32 marks a cell, a link of 8 KiB a cell, a style of its own of about
    /// 30 bytes for a cell of 8). Measured once and kept: a page of history does not change until a
    /// resize, which calls [`Page::forget_extra_bytes`].
    pub(super) fn extra_bytes(&mut self) -> usize {
        if let Some(bytes) = self.extra {
            return bytes;
        }
        let bytes = self.graphemes.bytes() + self.links.bytes() + self.styles.used_bytes();
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

    /// The size `row` is drawn at.
    #[inline]
    pub(super) fn line_size(&self, row: usize) -> LineSize {
        self.rows[row].size()
    }

    /// Give `row` the size `size`; the row is marked changed when that is a change.
    pub(super) fn set_line_size(&mut self, row: usize, size: LineSize) {
        let flags = &mut self.rows[row].flags;
        let mut new = (*flags & !SIZE_MASK) | size.to_bits() << SIZE_SHIFT;
        if size.is_double() {
            // A row of such a size is never soft-wrapped (see `Terminal::wrap`).
            new &= !WRAPPED;
        }
        if new != *flags {
            *flags = new | DIRTY;
        }
    }

    /// What a row copied over another takes along: its wrap flag, block and size.
    pub(super) fn meta(&self, row: usize) -> Meta {
        self.rows[row].meta()
    }

    /// Give `row` the wrap flag, block and size of a row that was copied over it, and mark it
    /// changed.
    pub(super) fn set_meta(&mut self, row: usize, meta: Meta) {
        self.set_wrapped(row, meta.wrapped);
        self.set_line_size(row, meta.size);
        self.rows[row].block = meta.block;
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
}
