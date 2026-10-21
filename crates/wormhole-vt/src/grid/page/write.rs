//! The page's writing operations: the cells text is printed into, the marks and links that come
//! with them. The invariants they keep are the ones in `mod.rs`.

use crate::types::Hyperlink;

use super::Page;
use crate::grid::cell::{Cell, CellFlags, StyleId};
use crate::grid::pen::Pen;
use crate::grid::row::DIRTY;
use crate::grid::style::Style;

impl Page {
    /// Write printable ASCII `bytes` into row `row` from column `col`: the hot path. The caller
    /// keeps them inside the row.
    #[inline]
    pub(in crate::grid) fn write_ascii(&mut self, row: usize, col: usize, bytes: &[u8], pen: &Pen) {
        debug_assert!(col + bytes.len() <= self.cols && bytes.is_ascii());
        let n = bytes.len();
        if n == 0 {
            return;
        }
        let (a, b, template) = self.claim_run(row, col, n, pen);
        for (cell, &byte) in self.cells[a..b].iter_mut().zip(bytes) {
            *cell = template.with_codepoint(byte as u32);
        }
        self.note_written(row, pen);
    }

    /// Take over the `n` cells from `col` of `row` for a write in `pen`'s look: the edges of the
    /// run no longer cut a wide character, the style is counted once for all `n` cells and what
    /// they held is given back, and the run is put under the pen's link. Gives the absolute
    /// offsets of the run and the cell (without a character) every one of them becomes.
    #[inline]
    fn claim_run(&mut self, row: usize, col: usize, n: usize, pen: &Pen) -> (usize, usize, Cell) {
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
        (a, b, Cell::compose(0, style, flags))
    }

    /// Write `c`, one cell wide and without marks, in `n` cells from `col` of `row` in `pen`'s
    /// look: DECFRA's row, one style count and one link for the run where `write_char` made them
    /// cell by cell.
    pub(in crate::grid) fn fill_char(&mut self, row: usize, col: usize, n: usize, c: char, pen: &Pen) {
        if n == 0 {
            return;
        }
        let (a, b, template) = self.claim_run(row, col, n, pen);
        self.cells[a..b].fill(template.with_codepoint(c as u32));
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
    /// text (`link_table::LINK_TEXT_MAX`) and they stay without one.
    pub(in crate::grid) fn attach_link(&mut self, a: usize, n: usize, link: &Hyperlink) -> bool {
        if !self.links.has_room_for(link) {
            return false;
        }
        let id = self.links.acquire(link, n as u32);
        self.links.attach(a as u32, n as u32, id);
        true
    }

    /// Write `c` of `width` 1 or 2 at `col` of `row`, with the marks that follow it in its cluster.
    /// A wide character takes the next cell as well, which the caller has made sure exists.
    pub(in crate::grid) fn write_char(
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
    pub(in crate::grid) fn write_spacer_head(&mut self, row: usize, pen: &Pen) {
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
    pub(in crate::grid) fn push_mark(&mut self, row: usize, col: usize, c: char) -> bool {
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
    pub(in crate::grid) fn set_width(&mut self, row: usize, col: usize, width: u8) -> bool {
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
    pub(in crate::grid) fn put(
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
        self.place(at, c, marks, id, link, flags);
    }

    /// Set the cell at absolute offset `at` to `c` in style `id`, which the caller has counted,
    /// with the marks, link and flags given. What the cell held was given back already.
    pub(in crate::grid) fn place(
        &mut self,
        at: usize,
        c: Option<char>,
        marks: &[char],
        id: StyleId,
        link: Option<&Hyperlink>,
        flags: CellFlags,
    ) {
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
