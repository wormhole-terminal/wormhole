//! The grid's page list: how rows are found by serial number, line or screen row, and how pages
//! are added. The history's limit is in `history.rs`, the view in `view.rs`.

use crate::types::Line;

use super::Grid;
use super::cell::Cell;
use super::page::Page;

/// Cells in a page, at most: 16,384 (128 KiB) is a few hundred rows of a normal terminal, small
/// enough that dropping the oldest page loses little history, large enough that the page list
/// stays short.
const PAGE_CELLS: usize = 16_384;
/// A page holds fewer cells than this, so a style id (16 bits) is never missing and a cell's
/// offset is a `u32` with room to spare.
const PAGE_CELLS_MAX: usize = 65_534;

impl Grid {
    #[inline]
    pub(super) fn first_serial(&self) -> u64 {
        self.pages.front().map_or(0, |page| page.base)
    }

    /// Serial number of the live screen's top row.
    #[inline]
    pub(super) fn top_serial(&self) -> u64 {
        self.end - self.rows as u64
    }

    /// The page and the index in it of the row with this serial number, which must exist.
    pub(super) fn locate(&self, serial: u64) -> (usize, usize) {
        debug_assert!(serial >= self.first_serial() && serial < self.end);
        let mut page = self.pages.len() - 1;
        // The live screen is in the last pages: look back from the end before searching.
        for _ in 0..4 {
            if self.pages[page].base <= serial || page == 0 {
                return (page, (serial - self.pages[page].base) as usize);
            }
            page -= 1;
        }
        let page = self.pages.partition_point(|p| p.base <= serial).saturating_sub(1);
        (page, (serial - self.pages[page].base) as usize)
    }

    /// The page and row index of live screen row `y`.
    #[inline]
    pub(super) fn locate_y(&self, y: usize) -> (usize, usize) {
        debug_assert!(y < self.rows);
        self.locate(self.top_serial() + y as u64)
    }

    /// The serial number of `line` if the grid has it.
    pub(super) fn serial_of(&self, line: Line) -> Option<u64> {
        let serial = self.top_serial().checked_add_signed(line.0 as i64)?;
        (serial >= self.first_serial() && serial < self.end).then_some(serial)
    }

    /// The page and row index of `line`, history included.
    pub(super) fn locate_line(&self, line: Line) -> Option<(usize, usize)> {
        self.serial_of(line).map(|serial| self.locate(serial))
    }

    #[inline]
    pub(super) fn page_row_mut(&mut self, y: usize) -> (&mut Page, usize) {
        let (page, row) = self.locate_y(y);
        (&mut self.pages[page], row)
    }

    /// Append a blank row after the last; whether it took a new page to do it.
    pub(super) fn push_row(&mut self) -> bool {
        let new_page = self.pages.back().is_none_or(Page::is_full);
        if new_page {
            self.add_page();
        }
        if let Some(page) = self.pages.back_mut() {
            page.push_row();
            self.end += 1;
        }
        new_page
    }

    pub(super) fn add_page(&mut self) {
        let uid = self.next_uid;
        self.next_uid += 1;
        let page = match self.spare.take() {
            Some(mut page) if page.cols() == self.cols && page.capacity() == self.page_capacity() => {
                page.recycle(uid, self.end);
                page
            }
            _ => Page::new(uid, self.end, self.cols, self.page_capacity()),
        };
        self.pages.push_back(page);
    }

    /// How many rows a new page holds. A page with no history to keep is exactly the screen.
    pub(super) fn page_capacity(&self) -> usize {
        if self.page_rows != 0 {
            return self.page_rows.min(PAGE_CELLS_MAX / self.cols).max(1);
        }
        let target = if self.limit == 0 {
            self.rows * self.cols
        } else {
            // About eight pages in the limit, so that a small limit isn't a single page.
            (self.limit / (8 * size_of::<Cell>())).clamp(2048, PAGE_CELLS)
        };
        let by_target = (target / self.cols).max(1);
        // Room for the screen where that is cheap: the live rows then span two pages at most.
        let wanted = if self.limit == 0 { by_target } else { by_target.max(self.rows) };
        wanted.min(PAGE_CELLS_MAX / self.cols).max(1)
    }
}
