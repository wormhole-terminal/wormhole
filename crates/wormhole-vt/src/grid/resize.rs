//! Resizing: the height, and the width with a reflow.
//!
//! # What users expect, and what this does
//!
//! - **Soft-wrapped lines are rewrapped** to the new width; a line the program ended with a newline
//!   is not joined to the next. The history is reflowed with the screen, as one sequence of rows.
//! - **Wide characters never split.** One that doesn't fit at the end of a row goes to the next, and
//!   leaves a spacer head behind. The narrowest grid is two columns, so one always fits.
//! - **Trailing blanks go.** Cells at the end of a line that nothing was written to (no character,
//!   no style) are not part of the line; a line shorter than the width is one row. Cells with a
//!   background (erased with a color) are kept.
//! - **The cursor is carried along** to the same character. A cursor past the end of its line's text
//!   (a prompt, waiting) stays right after the text, on a row of its own if the text filled the last.
//! - **Height.** When the screen is full (the cursor or the last text is on its bottom row), it is
//!   bottom-aligned: growing pulls rows back from the history, shrinking pushes the top rows into
//!   it, and the cursor stays on the bottom row. When there is blank space under the text, the
//!   screen keeps its top: growing adds blank rows at the bottom, shrinking drops the blank rows
//!   first and pushes rows into the history only if the text still doesn't fit. Rows below the cursor
//!   that no longer fit are dropped. The cursor always stays on the screen.
//! - **The view** (scrolled back into the history) stays on the text it showed.
//! - **Pins** follow their text through the reflow.
//! - **The alternate screen** isn't reflowed (programs redraw it): `reflow == false` crops and pads.

use crate::types::{Column, Line, Point};

use super::page::Page;
use super::{Grid, MAX_COLS, MAX_ROWS, MIN_COLS};

/// A point to carry through a rebuild, and where it ended up.
pub(super) struct Track {
    /// Where it is in the caller's list (the rewrap sorts the tracks by row).
    pub(super) id: usize,
    pub(super) serial: u64,
    pub(super) col: usize,
    pub(super) result: (u64, usize),
}

impl Grid {
    /// Resize to `cols` by `rows`, carrying `cursor` (a point of the live screen) along; returns
    /// where it is now. With `reflow` the soft-wrapped lines are rewrapped (the primary screen);
    /// without it rows are cropped and padded (the alternate screen). See the module docs for the
    /// rules. Everything on screen is damaged.
    pub fn resize(&mut self, cols: usize, rows: usize, cursor: Point, reflow: bool) -> Point {
        let cols = cols.clamp(MIN_COLS, MAX_COLS);
        let rows = rows.clamp(1, MAX_ROWS);
        let cursor_line = cursor.line.0.clamp(0, self.rows as i32 - 1) as usize;
        let cursor_col = cursor.column.0.min(self.cols - 1);
        if cols == self.cols && rows == self.rows {
            return Point::new(Line(cursor_line as i32), Column(cursor_col));
        }

        let top = self.top_serial();
        let cursor_serial = top + cursor_line as u64;
        let used_rows = self.used_rows().max(cursor_line + 1);
        let full = used_rows == self.rows;
        let used_row = top + used_rows as u64 - 1;

        let mut tracks = vec![
            Track { id: 0, serial: cursor_serial, col: cursor_col, result: (cursor_serial, cursor_col) },
            Track { id: 1, serial: top, col: 0, result: (top, 0) },
        ];
        let view = (self.offset > 0).then(|| top - self.offset as u64);
        if let Some(view) = view {
            tracks.push(Track { id: 2, serial: view, col: 0, result: (view, 0) });
        }
        let pins_from = tracks.len();
        let first = self.first_serial();
        let pin_ids: Vec<usize> = self
            .pins
            .live()
            .filter(|&(_, serial, _)| serial >= first && serial < self.end)
            .map(|(ix, serial, col)| {
                tracks.push(Track { id: tracks.len(), serial, col, result: (serial, col) });
                ix
            })
            .collect();

        if cols != self.cols {
            self.placements.drop_headless(first as i64, cols);
        }
        let placements_from = tracks.len();
        let placement_ids: Vec<usize> = self
            .placements
            .anchors()
            .filter(|&(_, top, _)| top >= first as i64 && top < self.end as i64)
            .map(|(ix, top, col)| {
                tracks.push(Track { id: tracks.len(), serial: top as u64, col, result: (top as u64, col) });
                ix
            })
            .collect();

        let mut used_end = used_row + 1;
        if cols != self.cols {
            used_end = if reflow {
                self.rewrap(cols, &mut tracks, used_row)
            } else {
                self.crop(cols, &mut tracks, used_row)
            };
        }
        for (track, ix) in tracks[pins_from..placements_from].iter().zip(&pin_ids) {
            self.pins.set_at(*ix, track.result.0, track.result.1);
        }
        for (track, ix) in tracks[placements_from..].iter().zip(&placement_ids) {
            self.placements.set_anchor(*ix, track.result.0 as i64, track.result.1);
        }

        // Where the screen sits in the rebuilt sequence.
        let (cursor_at, top_at) = (tracks[0].result, tracks[1].result.0);
        let fit = rows as u64;
        let used_end = used_end.max(cursor_at.0 + 1);
        let wanted = used_end.saturating_sub(fit);
        let mut s = if full { wanted } else { top_at.max(wanted) };
        s = s.min(cursor_at.0).max(self.first_serial());
        let new_end = s + fit;
        self.rows = rows;
        if new_end > self.end {
            for _ in 0..new_end - self.end {
                self.push_row();
            }
        } else if new_end < self.end {
            self.drop_tail_rows((self.end - new_end) as usize);
            self.pins.drop_from(new_end);
            self.cut_placements_at(new_end);
        }
        self.offset = match view {
            Some(_) => {
                let view_at = tracks[2].result.0;
                (s.saturating_sub(view_at) as usize).min(self.history_len())
            }
            None => 0,
        };
        self.all_dirty = true;
        self.drop_placements_off_screen();
        // Rows went from the history to the screen and back: measure the pages again.
        self.pages.iter_mut().for_each(Page::forget_extra_bytes);
        self.enforce_limit();
        Point::new(Line((cursor_at.0 - s) as i32), Column(cursor_at.1.min(self.cols - 1)))
    }

    /// Drop the last `n` rows.
    fn drop_tail_rows(&mut self, mut n: usize) {
        while n > 0 {
            let many = self.pages.len() > 1;
            let Some(last) = self.pages.back_mut() else { return };
            let len = last.rows.len();
            let k = n.min(len);
            if k == len && many {
                self.pages.pop_back();
            } else {
                last.drop_back_rows(k);
            }
            self.end -= k as u64;
            n -= k;
        }
    }
}
