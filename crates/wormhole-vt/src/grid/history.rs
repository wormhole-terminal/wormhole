//! The history's limit and its clearing: how much of it is kept, and what drops the oldest rows,
//! empties it or moves the screen into it.

use crate::types::Color;

use super::cell::Cell;
use super::page::Page;
use super::row::{Region, Row};
use super::{BG_DEFAULT, Grid};

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
}
