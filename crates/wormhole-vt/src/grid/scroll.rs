//! Scrolling and inserting lines.
//!
//! Scrolling the whole screen up, the common case (output at the bottom), appends a row to the page
//! list: the top row becomes history where it is and nothing is copied. A part of the screen
//! (margins, DECSTBM) rotates row records inside one page, which also copies no cell; only a part
//! that straddles two pages, or one with left and right margins (DECLRMM), copies cells.
//!
//! Which scrolls feed the history (as terminals do): a scroll *up* of a region that starts at the
//! top row and spans the full width, on a grid that keeps history. Scrolling down, inserting and
//! deleting lines, and scrolling a region that starts lower, lose the lines that leave.

use std::ops::Range;

use crate::types::Color;

use super::page::Page;
use super::row::Region;
use super::{Grid, Spare};

impl Grid {
    /// Scroll the lines of `region` up by `n`: the top `n` leave (into the history, when the
    /// region is the full width from the top row and the grid keeps one), the rest move up and `n`
    /// blank lines (with `bg` as background) enter at the bottom. LF at the bottom margin and SU.
    pub fn scroll_up(&mut self, region: Region, n: usize, bg: Color) {
        let Some(region) = self.clamp_region(region) else { return };
        let n = n.min(region.bottom - region.top + 1);
        if n == 0 {
            return;
        }
        if region.top == 0 && self.full_width(region) && self.limit > 0 {
            self.scroll_into_history(region.bottom, n, bg);
        } else {
            self.shift_lines(region, region.top, n, true, bg);
        }
    }

    /// Scroll the lines of `region` down by `n`: the bottom `n` are lost, `n` blank lines enter at
    /// the top. RI at the top margin and SD.
    pub fn scroll_down(&mut self, region: Region, n: usize, bg: Color) {
        let Some(region) = self.clamp_region(region) else { return };
        let n = n.min(region.bottom - region.top + 1);
        if n > 0 {
            self.shift_lines(region, region.top, n, false, bg);
        }
    }

    /// IL: insert `n` blank lines at row `y` of `region`, pushing the lines from there to the bottom
    /// margin down; those pushed past it are lost.
    pub fn insert_lines(&mut self, y: usize, region: Region, n: usize, bg: Color) {
        let Some(region) = self.clamp_region(region) else { return };
        if y < region.top || y > region.bottom {
            return;
        }
        let n = n.min(region.bottom - y + 1);
        if n > 0 {
            self.shift_lines(region, y, n, false, bg);
        }
    }

    /// DL: delete `n` lines at row `y` of `region`, pulling the lines below up; blank lines enter at
    /// the bottom margin.
    pub fn delete_lines(&mut self, y: usize, region: Region, n: usize, bg: Color) {
        let Some(region) = self.clamp_region(region) else { return };
        if y < region.top || y > region.bottom {
            return;
        }
        let n = n.min(region.bottom - y + 1);
        if n > 0 {
            self.shift_lines(region, y, n, true, bg);
        }
    }

    /// Mark live rows `y0..=y1` as changed. One lookup, then a pass over each page's records.
    fn mark_rows_dirty(&mut self, y0: usize, y1: usize) {
        let y1 = y1.min(self.rows - 1);
        if y0 > y1 {
            return;
        }
        let (mut page, mut row) = self.locate_y(y0);
        let mut left = y1 - y0 + 1;
        while left > 0 {
            let in_page = &mut self.pages[page];
            let take = left.min(in_page.rows.len() - row);
            in_page.touch_rows(row..row + take);
            left -= take;
            page += 1;
            row = 0;
        }
    }

    fn clamp_region(&self, region: Region) -> Option<Region> {
        let region = Region {
            top: region.top,
            bottom: region.bottom.min(self.rows - 1),
            left: region.left,
            right: region.right.min(self.cols - 1),
        };
        (region.top <= region.bottom && region.left <= region.right).then_some(region)
    }

    fn full_width(&self, region: Region) -> bool {
        region.left == 0 && region.right == self.cols - 1
    }

    /// The rows `from..=bottom` of the region move by `n` (up or down) and `n` blank rows take the
    /// vacated end; `region`'s columns say how much of each row moves.
    fn shift_lines(&mut self, region: Region, from: usize, n: usize, up: bool, bg: Color) {
        let bottom = region.bottom;
        self.placements_shifted(region, from, n, up);
        if from == 0 && bottom == self.rows - 1 {
            self.all_dirty = true;
        } else {
            self.mark_rows_dirty(from, bottom);
        }
        if !self.full_width(region) {
            self.shift_partial_rows(region, from, n, up, bg);
            return;
        }
        let (first, last) = (self.locate_y(from), self.locate_y(bottom));
        if first.0 == last.0 {
            // One page: blank the lines that leave, then rotate the records. No cell moves.
            let (page, a) = (&mut self.pages[first.0], first.1);
            let b = last.1;
            if up {
                for row in a..a + n {
                    page.clear_row(row, bg);
                }
                page.rotate_rows_up(a..b + 1, n);
            } else {
                for row in b + 1 - n..=b {
                    page.clear_row(row, bg);
                }
                page.rotate_rows_down(a..b + 1, n);
            }
        } else if !self.rotate_across_pages(from, bottom, n, up, bg) {
            self.shift_rows_by_copy(from, bottom, n, up, bg);
        }
    }

    /// Scroll full rows `from..=bottom` that lie in several pages: rotate the records inside each
    /// page, and copy the `n` rows that cross a page boundary. A scroll by one line copies one row
    /// per boundary, not the region. False (nothing done) when a page holds fewer than `n` of the
    /// rows, which only tiny pages do.
    fn rotate_across_pages(&mut self, from: usize, bottom: usize, n: usize, up: bool, bg: Color) -> bool {
        // The region's rows by page: page, first row, one past the last.
        let mut segments = std::mem::take(&mut self.segments);
        segments.clear();
        let (mut page, mut row) = self.locate_y(from);
        let mut left = bottom - from + 1;
        while left > 0 {
            let take = left.min(self.pages[page].rows.len() - row);
            segments.push((page, row, row + take));
            left -= take;
            page += 1;
            row = 0;
        }
        if segments.iter().any(|&(_, a, b)| b - a < n) {
            self.segments = segments;
            return false;
        }
        let cols = self.cols;
        if up {
            for j in 0..segments.len() {
                let (p, a, b) = segments[j];
                self.pages[p].rotate_rows_up(a..b, n);
                match segments.get(j + 1) {
                    // The next page's first rows move up into the rows that just came round.
                    Some(&(next, na, _)) => {
                        for i in 0..n {
                            self.copy_page_row((next, na + i), (p, b - n + i), 0..cols, true);
                        }
                    }
                    None => {
                        for i in 0..n {
                            self.pages[p].clear_row(b - n + i, bg);
                        }
                    }
                }
            }
        } else {
            for j in (0..segments.len()).rev() {
                let (p, a, b) = segments[j];
                self.pages[p].rotate_rows_down(a..b, n);
                match j.checked_sub(1).map(|prev| segments[prev]) {
                    Some((before, _, bb)) => {
                        for i in 0..n {
                            self.copy_page_row((before, bb - n + i), (p, a + i), 0..cols, true);
                        }
                    }
                    None => {
                        for i in 0..n {
                            self.pages[p].clear_row(a + i, bg);
                        }
                    }
                }
            }
        }
        self.segments = segments;
        true
    }

    /// The same by copying whole rows, for a part of the screen that spans pages.
    fn shift_rows_by_copy(&mut self, from: usize, bottom: usize, n: usize, up: bool, bg: Color) {
        let cols = self.cols;
        if up {
            for dst in from..bottom + 1 - n {
                self.copy_row_over(dst + n, dst, 0..cols, true);
            }
            self.erase_rows_cells(bottom + 1 - n..bottom + 1, bg, Spare::Nothing);
        } else {
            for dst in (from + n..=bottom).rev() {
                self.copy_row_over(dst - n, dst, 0..cols, true);
            }
            self.erase_rows_cells(from..from + n, bg, Spare::Nothing);
        }
    }

    /// Scroll with left and right margins: only the columns inside them move, row by row.
    fn shift_partial_rows(&mut self, region: Region, from: usize, n: usize, up: bool, bg: Color) {
        let cols = region.left..region.right + 1;
        let bottom = region.bottom;
        if up {
            for dst in from..bottom + 1 - n {
                self.copy_row_over(dst + n, dst, cols.clone(), false);
            }
            for y in bottom + 1 - n..=bottom {
                self.erase_cells(y, cols.clone(), bg, Spare::Nothing);
            }
        } else {
            for dst in (from + n..=bottom).rev() {
                self.copy_row_over(dst - n, dst, cols.clone(), false);
            }
            for y in from..from + n {
                self.erase_cells(y, cols.clone(), bg, Spare::Nothing);
            }
        }
    }

    /// Copy `cols` of live row `src` over row `dst`, with the row's wrap flag and block when `meta`.
    fn copy_row_over(&mut self, src: usize, dst: usize, cols: Range<usize>, meta: bool) {
        let (from, to) = (self.locate_y(src), self.locate_y(dst));
        self.copy_page_row(from, to, cols, meta);
    }

    /// The same between rows named by page and index.
    fn copy_page_row(
        &mut self,
        (sp, sr): (usize, usize),
        (dp, dr): (usize, usize),
        cols: Range<usize>,
        meta: bool,
    ) {
        if sp == dp {
            self.pages[sp].copy_within_page(sr, dr, cols);
        } else {
            // Take the source page out to hold it beside the destination.
            let source = std::mem::replace(&mut self.pages[sp], Page::empty());
            self.pages[dp].copy_from_page(&source, sr, dr, cols);
            self.pages[sp] = source;
        }
        let meta = meta.then(|| self.pages[sp].meta(sr));
        let target = &mut self.pages[dp];
        match meta {
            Some(meta) => target.set_meta(dr, meta),
            None => target.touch(dr),
        }
    }

    /// Scroll up the region `0..=bottom` of the full width by `n` into the history: the top `n`
    /// rows become history in place and `n` blank rows are appended. When rows sit below the
    /// region (a status line under a scroll region), they are copied down past the appended rows so
    /// that they stay the last ones: a few rows copied, never the screen.
    fn scroll_into_history(&mut self, bottom: usize, n: usize, bg: Color) {
        let below = self.rows - 1 - bottom;
        if below > 0 {
            self.placements_under_region(bottom + 1, n);
        }
        let mut new_page = false;
        for _ in 0..n {
            new_page |= self.push_row();
        }
        let cols = self.cols;
        if below > 0 {
            // After the append the rows under the region sit n rows too high.
            for src in (self.rows - n - below..self.rows - n).rev() {
                self.copy_row_over(src, src + n, 0..cols, true);
            }
        }
        if below > 0 || !bg_is_default(bg) {
            self.erase_rows_cells(bottom + 1 - n..bottom + 1, bg, Spare::Nothing);
        }
        // Every row on screen now holds other text than before: one flag, not a walk over the rows.
        self.all_dirty = true;
        if self.offset > 0 {
            // The view stays on the text it was showing while output arrives below.
            self.offset += n;
        }
        // The history only outgrows its cap when a page is added.
        if new_page {
            self.enforce_limit();
        }
    }
}

fn bg_is_default(bg: Color) -> bool {
    super::style::Style::erased(bg).is_default()
}
