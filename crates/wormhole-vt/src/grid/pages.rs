//! The grid's page list, its history limit and its view: how rows are found, pages are added and
//! dropped, and the part of the history on show is moved.

use crate::types::{Color, Line, Scroll};

use super::cell::Cell;
use super::page::Page;
use super::row::{Region, Row};
use super::{BG_DEFAULT, Grid};

/// Cells in a page, at most: 16,384 (128 KiB) is a few hundred rows of a normal terminal, small
/// enough that dropping the oldest page loses little history, large enough that the page list
/// stays short.
const PAGE_CELLS: usize = 16_384;
/// A page holds fewer cells than this, so a style id (16 bits) is never missing and a cell's
/// offset is a `u32` with room to spare.
const PAGE_CELLS_MAX: usize = 65_534;

impl Grid {
    /// The bytes of history the grid may keep; see [`Self::set_scrollback_limit`].
    pub fn scrollback_limit(&self) -> usize {
        self.limit
    }

    /// Cap the history at `bytes` of cells and row records, and drop what is over. The cap is a
    /// ceiling: the oldest pages go whole, so between one and two pages (128 KiB each at most) less
    /// than the cap are kept. The history's combining marks and links may take as many bytes again
    /// (they are tiny in text people read, and a program sends as many as it likes); its styles
    /// are not counted, as a page has at most one per cell.
    pub fn set_scrollback_limit(&mut self, bytes: usize) {
        self.limit = bytes;
        self.enforce_limit();
    }

    /// Bytes the grid holds in pages (cells, row records, style and side tables).
    pub fn memory_bytes(&self) -> usize {
        self.pages.iter().map(Page::bytes).sum()
    }

    /// Bytes one row costs in the limit: its cells and its record.
    pub(crate) fn row_cost(&self) -> usize {
        self.cols * size_of::<Cell>() + size_of::<Row>()
    }

    // ---- the page list ----

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

    /// Drop history over the cap: the oldest pages, whole, that hold no live row. The page being
    /// filled turns into history row by row, so room for all of it is kept free: the history never
    /// outgrows the cap between two pages, and ends up a page or two short of it. The marks and
    /// links of the pages that are all history have a cap of the same size.
    pub(super) fn enforce_limit(&mut self) {
        let room = self.pages.back().map_or(0, Page::capacity);
        let max_rows = (self.limit / self.row_cost()).saturating_sub(room);
        let top = self.top_serial();
        let mut extra: usize = self
            .pages
            .iter_mut()
            .take_while(|page| page.base + page.rows.len() as u64 <= top)
            .map(Page::extra_bytes)
            .sum();
        while self.pages.len() > 1 && (self.history_len() > max_rows || extra > self.limit) {
            let Some(first) = self.pages.front_mut() else { break };
            if first.base + first.rows.len() as u64 > top {
                break;
            }
            extra -= first.extra_bytes();
            self.drop_first_page();
        }
        if self.limit == 0 && self.history_len() > 0 {
            self.drop_history_rows(self.history_len());
        }
        self.offset = self.offset.min(self.history_len());
        self.prune_placements();
    }

    pub(super) fn drop_first_page(&mut self) {
        let page = self.pages.pop_front();
        if self.spare.is_none() {
            self.spare = page;
        }
    }

    /// Drop the oldest `n` rows of history (at most all of it): whole pages while they fit, then
    /// the first rows of the page that holds the rest.
    pub(super) fn drop_history_rows(&mut self, mut n: usize) {
        while n > 0 {
            let Some(len) = self.pages.front().map(|page| page.rows.len()) else { break };
            if len <= n && self.pages.len() > 1 {
                n -= len;
                self.drop_first_page();
            } else {
                if let Some(first) = self.pages.front_mut() {
                    first.drop_front_rows(n.min(len));
                }
                break;
            }
        }
        self.offset = self.offset.min(self.history_len());
        self.prune_placements();
    }

    /// Forget the history. The screen stays; pins into the history are gone.
    pub fn clear_scrollback(&mut self) {
        self.drop_history_rows(self.history_len());
        self.offset = 0;
        self.all_dirty = true;
    }

    /// Blank the screen and forget the history (RIS). Pins stay where they are.
    pub fn reset(&mut self) {
        self.clear_placements();
        self.clear_scrollback();
        self.erase_rows(0..self.rows, BG_DEFAULT, false);
    }

    /// Move everything on the screen up into the history and leave it blank (a clear that keeps
    /// the scrollback): the lines down to the last one with anything on it, so that no blank
    /// lines are added.
    pub fn scroll_screen_into_history(&mut self, bg: Color) {
        let used = self.used_rows();
        if used > 0 {
            self.scroll_up(Region::full(self.cols, self.rows), used, bg);
        }
    }

    /// Rows of the live screen up to the last one that holds anything (a character, or a style such
    /// as an erased background); 0 for a blank screen.
    pub fn used_rows(&self) -> usize {
        let pictures = self.placements_used_rows();
        pictures.max(
            (0..self.rows)
                .rev()
                .find(|&y| {
                    let (page, row) = self.locate_y(y);
                    !self.pages[page].row_is_blank(row)
                })
                .map_or(0, |y| y + 1),
        )
    }

    // ---- the view ----

    /// How many lines the view is scrolled up into the history (0 for the live screen).
    pub fn display_offset(&self) -> usize {
        self.offset
    }

    /// Move the view; it stays inside the history. Everything shown changes, so the next snapshot
    /// copies all of it.
    pub fn scroll_view(&mut self, scroll: Scroll) {
        let history = self.history_len();
        let before = self.offset;
        self.offset = match scroll {
            Scroll::Delta(lines) => (self.offset as i64 + lines as i64).clamp(0, history as i64) as usize,
            Scroll::Top => history,
            Scroll::Bottom => 0,
        };
        self.all_dirty |= self.offset != before;
    }

    /// Mark everything as changed: the next snapshot copies every row.
    pub fn damage_all(&mut self) {
        self.all_dirty = true;
    }
}
