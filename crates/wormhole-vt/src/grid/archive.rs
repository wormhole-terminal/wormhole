//! History lines on their way out: the copy a host can keep when the grid lets go of them.
//!
//! The grid frees history by pages (see `history.rs`). When a host asked for it, each page that
//! goes is copied first, row by row, into batches the terminal turns into events. Off, which is
//! the default, the grid holds no archive and the only cost is one `Option` test where a page is
//! dropped.
//!
//! The batches wait in the grid until the host takes them, so they are capped in bytes. Past the
//! cap the newest lines are not copied, only counted (`skipped`), and nothing more is copied until
//! the host has taken what is waiting: the lines that did arrive and the ones that were skipped
//! stay in the order they left the history.

use crate::types::{Cell as DrawCell, LineSize, RowExtras};

use super::copy::copy_page_row;
use super::page::Page;
use super::{Grid, RowMark};

/// Why history lines left the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DropReason {
    /// Output scrolled them out past the history's limit; the oldest go first.
    Scrolled,
    /// The history was emptied: ED 3, a full reset (RIS) or `clear_history`.
    Cleared,
    /// The limit itself fell: the host lowered the scrollback, or a resize changed how many lines
    /// the limit holds (a wider screen has fewer of them in the same bytes).
    Limit,
}

/// One row of history that left the grid: what [`copy_row`](crate::Terminal::copy_row) would have
/// given for it, and whether it was soft-wrapped.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HistoryLine {
    /// One cell per column; the last of a wrapped row has [`Flags::WRAPLINE`](crate::Flags::WRAPLINE).
    pub cells: Vec<DrawCell>,
    /// Zero-width characters, underline colors and hyperlinks of the row's cells.
    pub extras: RowExtras,
    /// The block (OSC 133) the row carries a part of.
    pub mark: Option<RowMark>,
    /// The line goes on in the next row: the text of a wrapped row and of the rows after it, up
    /// to the first that is not wrapped, is one line of text.
    pub wrapped: bool,
    /// How big the row was drawn (DECDWL, DECDHL).
    pub size: LineSize,
}

/// History lines that left the engine, oldest first, and why. See
/// [`Event::HistoryDropped`](crate::Event::HistoryDropped).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DroppedHistory {
    /// Why they left.
    pub reason: DropReason,
    /// The lines, oldest first.
    pub lines: Vec<HistoryLine>,
    /// How many more lines of this reason came right after `lines` and were not copied, because
    /// the host had not taken the ones waiting (the backlog limit). Zero when nothing was lost.
    pub skipped: usize,
}

/// What a line costs the backlog besides its extras.
fn line_cost(cols: usize) -> usize {
    cols * size_of::<DrawCell>() + size_of::<HistoryLine>()
}

/// The lines waiting for the host.
pub(super) struct Archive {
    /// Most bytes of lines to hold.
    cap: usize,
    used: usize,
    /// A line was skipped since the host last took the batches: copy none until it has.
    full: bool,
    batches: Vec<DroppedHistory>,
}

impl Archive {
    /// Copy the first `count` rows of `page`, which the grid is about to drop.
    fn record(&mut self, page: &Page, count: usize, reason: DropReason) {
        let Archive { cap, used, full, batches } = self;
        if batches.last().is_none_or(|last| last.reason != reason) {
            batches.push(DroppedHistory { reason, lines: Vec::new(), skipped: 0 });
        }
        let Some(batch) = batches.last_mut() else { return };
        let cost = line_cost(page.cols());
        for row in 0..count {
            if *full || *used + cost > *cap {
                *full = true;
                batch.skipped += 1;
                continue;
            }
            let mut cells = Vec::new();
            let mut extras = RowExtras::default();
            let mark = copy_page_row(page, row, &mut cells, Some(&mut extras));
            *used += cost + extras.heap_bytes();
            batch.lines.push(HistoryLine {
                cells,
                extras,
                mark,
                wrapped: page.rows[row].wrapped(),
                size: page.rows[row].size(),
            });
        }
        if batch.lines.is_empty() && batch.skipped == 0 {
            batches.pop();
        }
    }
}

impl Grid {
    /// Start (or, with a different `cap`, go on) collecting the lines the history drops, holding at
    /// most `cap` bytes of them for the host; `None` stops, and what waits is discarded: take it
    /// first with [`take_archive`](Self::take_archive).
    pub fn set_archive(&mut self, cap: Option<usize>) {
        match (cap, self.archive.as_deref_mut()) {
            (Some(cap), Some(archive)) => archive.cap = cap,
            (Some(cap), None) => {
                self.archive = Some(Box::new(Archive { cap, used: 0, full: false, batches: Vec::new() }));
            }
            (None, _) => self.archive = None,
        }
    }

    /// Whether the dropped lines are collected.
    pub fn archiving(&self) -> bool {
        self.archive.is_some()
    }

    /// The batches of dropped lines since the last call, oldest first. The backlog starts again.
    pub fn take_archive(&mut self) -> Vec<DroppedHistory> {
        let Some(archive) = self.archive.as_deref_mut() else { return Vec::new() };
        archive.used = 0;
        archive.full = false;
        std::mem::take(&mut archive.batches)
    }

    /// Copy the first `count` rows of page `page` before they are dropped. A grid without an
    /// archive does nothing.
    #[inline]
    pub(super) fn archive_rows(&mut self, page: usize, count: usize, reason: DropReason) {
        if let (Some(archive), Some(page)) = (self.archive.as_deref_mut(), self.pages.get(page)) {
            archive.record(page, count, reason);
        }
    }
}
