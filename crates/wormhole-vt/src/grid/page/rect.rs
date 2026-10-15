//! The page's side of the rectangular operations: changing the look of a run of cells (DECCARA,
//! DECRARA) and copying cells between rows (DECCRA).
//!
//! Copying goes through a snapshot of the source cells ([`Copied`]) because the source and the
//! destination may be the same row, overlapping, or rows of two pages with their own style tables.

use std::ops::Range;

use super::Page;
use crate::grid::cell::CellFlags;
use crate::grid::style::Style;
use crate::types::Hyperlink;

/// One cell taken out of a page, with everything it has beside its word: its style by value, its
/// link and where its marks are in the scratch list of marks.
pub(in crate::grid) struct Copied {
    c: Option<char>,
    style: Style,
    link: Option<Hyperlink>,
    flags: CellFlags,
    marks: Range<usize>,
}

impl Page {
    /// Give the cells `cols` of `row` the look `change` makes of theirs. A run of cells with one
    /// style is looked up and counted once. A wide character the columns cut in two changes as a
    /// whole: its halves always share a style.
    pub(in crate::grid) fn restyle(
        &mut self,
        row: usize,
        cols: Range<usize>,
        change: impl Fn(&Style) -> Style,
    ) {
        let base = self.base_of(row);
        let (mut a, mut b) = (base + cols.start, base + cols.end.min(self.cols));
        if a >= b {
            return;
        }
        if cols.start > 0 && self.cells[a].is_spacer_tail() {
            a -= 1;
        }
        if self.cells[b - 1].is_wide() && b < base + self.cols {
            b += 1;
        }
        let mut i = a;
        while i < b {
            let id = self.cells[i].style_id();
            let run = self.cells[i..b].iter().take_while(|cell| cell.style_id() == id).count();
            let old = *self.styles.get(id);
            let new = change(&old);
            if new != old {
                // Count the new style before giving the old one back: ids may be reused in between.
                let fresh = self.styles.acquire(&new, run as u32);
                for cell in &mut self.cells[i..i + run] {
                    *cell = cell.with_style(fresh);
                }
                self.styles.release(id, run as u32);
            }
            i += run;
        }
        self.touch(row);
    }

    /// Append the cells `cols` of `row` to `out` (their marks to `marks`).
    pub(in crate::grid) fn snapshot(
        &self,
        row: usize,
        cols: Range<usize>,
        out: &mut Vec<Copied>,
        marks: &mut Vec<char>,
    ) {
        let cells = self.cells(row);
        for (x, &cell) in cells.iter().enumerate().take(cols.end).skip(cols.start) {
            let at = self.offset(row, x);
            let first = marks.len();
            if cell.has_grapheme() {
                marks.extend_from_slice(self.graphemes.get(at));
            }
            out.push(Copied {
                c: cell.character(),
                style: *self.styles.get(cell.style_id()),
                link: if cell.has_hyperlink() { self.links.at(at).cloned() } else { None },
                flags: cell.flag_set(),
                marks: first..marks.len(),
            });
        }
    }

    /// Put the snapshot `cells` over the cells of `row` from `col`, which the caller has kept
    /// inside the row. A wide character the rectangle cut in two is left as a blank.
    pub(in crate::grid) fn store(&mut self, row: usize, col: usize, cells: &[Copied], marks: &[char]) {
        if cells.is_empty() {
            return;
        }
        let base = self.base_of(row);
        // Whole destination first: a wide character half under it must not keep its other half.
        self.guard_edges(base, col, col + cells.len());
        for (i, cell) in cells.iter().enumerate() {
            let cell_marks = marks.get(cell.marks.clone()).unwrap_or(&[]);
            self.put(row, col + i, cell.c, cell_marks, &cell.style, cell.link.as_ref(), cell.flags);
        }
        self.mend_copied_edges(base + col, base + col + cells.len());
        self.touch(row);
    }
}
