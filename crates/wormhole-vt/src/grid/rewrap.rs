//! The two ways a width change rebuilds the rows: rewrapping soft-wrapped lines, and cropping.
//!
//! Both read the old pages row by row into a fresh grid of the new width (`shell`) and then take
//! its pages over, carrying the [`Track`]s (cursor, view, pins, pictures) to where their text went.
//! The rules, and the height, are in `resize.rs`.

use std::collections::HashMap;

use crate::types::Hyperlink;

use super::Grid;
use super::cell::{Cell, CellFlags, StyleId};
use super::page::Page;
use super::resize::Track;
use super::style::Style;

/// One cell of a line being rewrapped, free of the page it came from.
struct Pend {
    /// The cell with its style id zeroed.
    cell: Cell,
    /// Index into the line's styles.
    style: u32,
    /// Index into the line's side data, or `u32::MAX`.
    aux: u32,
}

struct Aux {
    marks: Vec<char>,
    link: Option<Hyperlink>,
}

#[derive(Default)]
struct LineBuf {
    cells: Vec<Pend>,
    styles: Vec<Style>,
    /// `styles` by value, for a line with many: a scan is quicker for a few, and quadratic for a
    /// line of thousands of colors.
    by_style: HashMap<Style, u32>,
    aux: Vec<Aux>,
    block: u32,
    /// Tracked points on the line: offset into `cells` (maybe beyond the end), index in `tracks`.
    tracked: Vec<(usize, usize)>,
    holds_used_end: bool,
    /// The last style looked up: page, id, index.
    memo: (u64, StyleId, u32),
}

impl LineBuf {
    fn clear(&mut self) {
        self.cells.clear();
        self.styles.clear();
        self.by_style.clear();
        self.aux.clear();
        self.block = 0;
        self.tracked.clear();
        self.holds_used_end = false;
        self.memo = (0, 0, 0);
    }

    fn style_index(&mut self, page: &Page, id: StyleId) -> u32 {
        if id == 0 {
            return u32::MAX;
        }
        if self.memo.0 == page.uid && self.memo.1 == id {
            return self.memo.2;
        }
        let style = *page.styles.get(id);
        let found = if self.styles.len() <= 8 {
            self.styles.iter().position(|s| *s == style).map(|ix| ix as u32)
        } else {
            self.by_style.get(&style).copied()
        };
        let ix = found.unwrap_or_else(|| {
            self.styles.push(style);
            let ix = self.styles.len() as u32 - 1;
            self.by_style.insert(style, ix);
            ix
        });
        self.memo = (page.uid, id, ix);
        ix
    }
}

struct Rewrap {
    next: Grid,
    line: LineBuf,
    tracks: Vec<Track>,
    /// Tracks are sorted by serial; this is the next one not yet on a line.
    ti: usize,
    /// The last row (serial) of the old grid that held text or the cursor.
    used_row: u64,
    /// After the pass: the serial after the last row of the line that holds `used_row`.
    used_end: u64,
    /// The serial number of the row being filled by a line, and its next column.
    cur: Option<u64>,
    col: usize,
}

impl Grid {
    /// A fresh grid shell with the new width, to build the rows of the old one into.
    fn shell(&self, cols: usize) -> Grid {
        Grid {
            end: self.first_serial(),
            next_uid: self.next_uid,
            ..Grid::blank(cols, self.rows, self.limit, self.page_rows)
        }
    }

    /// Take over the pages of `next`.
    fn adopt(&mut self, next: Grid) {
        self.cols = next.cols;
        self.pages = next.pages;
        self.end = next.end;
        self.next_uid = next.next_uid;
        self.spare = None;
    }

    /// Rebuild every row at width `cols`, cropping or padding each: one new row per old row, same
    /// serial number, so `used_row` is still where the text ends.
    pub(super) fn crop(&mut self, cols: usize, tracks: &mut [Track], used_row: u64) -> u64 {
        let mut next = self.shell(cols);
        let keep = cols.min(self.cols);
        let old = std::mem::take(&mut self.pages);
        for page in &old {
            for row in 0..page.rows.len() {
                next.push_row();
                let Some(dst) = next.pages.back_mut() else { continue };
                let dst_row = dst.rows.len() - 1;
                dst.copy_from_page(page, row, dst_row, 0..keep);
                dst.set_meta(dst_row, page.meta(row));
            }
        }
        self.adopt(next);
        for track in tracks {
            track.result = (track.serial, track.col.min(cols - 1));
        }
        used_row + 1
    }

    /// Rewrap the whole sequence of rows to `cols`; the serial after the last row of the line that
    /// holds `used_row`.
    pub(super) fn rewrap(&mut self, cols: usize, tracks: &mut Vec<Track>, used_row: u64) -> u64 {
        tracks.sort_by_key(|t| t.serial);
        let mut work = Rewrap {
            next: self.shell(cols),
            line: LineBuf::default(),
            tracks: std::mem::take(tracks),
            ti: 0,
            used_row,
            used_end: 0,
            cur: None,
            col: 0,
        };
        let mut serial = self.first_serial();
        let mut old = std::mem::take(&mut self.pages);
        while let Some(page) = old.pop_front() {
            for row in 0..page.rows.len() {
                work.row(&page, row, serial);
                serial += 1;
            }
        }
        work.flush();
        let used_end = work.used_end.max(work.next.first_serial());
        *tracks = work.tracks;
        // Back to the caller's order: cursor, top, view, pins.
        tracks.sort_by_key(|t| t.id);
        self.adopt(work.next);
        used_end
    }
}

impl Rewrap {
    /// Feed the next row of the old sequence.
    fn row(&mut self, page: &Page, row: usize, serial: u64) {
        let record = page.rows[row];
        let cells = page.cells(row);
        let wrapped = record.wrapped();
        let first_track = self.ti;
        while self.ti < self.tracks.len() && self.tracks[self.ti].serial == serial {
            self.ti += 1;
        }
        let here = first_track..self.ti;
        let width = self.next.cols;
        if self.line.cells.is_empty() && !wrapped {
            // A line of one row: the common case, copied whole when it fits.
            let len = cells
                .iter()
                .rposition(|c| !(c.is_blank_but_tab() || c.is_spacer_head()))
                .map_or(0, |p| p + 1);
            if self.tracks[here.clone()].iter().all(|t| t.col < width) && len <= width {
                self.copy_row(page, row, len, here, serial);
                return;
            }
        }
        // Several rows, or one that doesn't fit: gather the line's cells, page-independent.
        let line = &mut self.line;
        let before = line.cells.len();
        let end = if wrapped {
            cells.len() - usize::from(cells.last().is_some_and(|c| c.is_spacer_head()))
        } else {
            cells.iter().rposition(|c| !(c.is_blank_but_tab() || c.is_spacer_head())).map_or(0, |p| p + 1)
        };
        for (x, cell) in cells[..end].iter().enumerate() {
            let style = line.style_index(page, cell.style_id());
            let mut aux = u32::MAX;
            if cell.has_grapheme() || cell.has_hyperlink() {
                let offset = page.offset(row, x);
                aux = line.aux.len() as u32;
                line.aux.push(Aux {
                    marks: page.graphemes.get(offset).to_vec(),
                    link: page.links.at(offset).cloned(),
                });
            }
            line.cells.push(Pend { cell: cell.with_style(0), style, aux });
        }
        for t in here {
            // A track on the dropped spacer head, or past the trimmed end, sits just after the text.
            let col = self.tracks[t].col;
            let offset = if col < end { before + col } else { before + col.max(end) };
            line.tracked.push((offset, t));
        }
        line.block = line.block.max(record.block);
        line.holds_used_end |= serial == self.used_row;
        if !wrapped {
            self.emit_line();
        }
    }

    /// A one-row line that fits: copy its cells.
    fn copy_row(&mut self, page: &Page, row: usize, len: usize, tracks: std::ops::Range<usize>, serial: u64) {
        self.next.push_row();
        let new_serial = self.next.end - 1;
        let Some(dst) = self.next.pages.back_mut() else { return };
        let dst_row = dst.rows.len() - 1;
        if len > 0 {
            dst.copy_from_page(page, row, dst_row, 0..len);
        }
        dst.set_block(dst_row, page.block(row));
        for t in tracks {
            self.tracks[t].result = (new_serial, self.tracks[t].col);
        }
        if serial == self.used_row {
            self.used_end = new_serial + 1;
        }
    }

    /// Start a new destination row, on the command block `block`.
    fn open_row(&mut self, block: u32) {
        self.next.push_row();
        self.cur = Some(self.next.end - 1);
        self.col = 0;
        if let Some(page) = self.next.pages.back_mut() {
            let last = page.rows.len() - 1;
            page.set_block(last, block);
        }
    }

    /// Lay the gathered line out in rows of the new width.
    fn emit_line(&mut self) {
        let width = self.next.cols;
        let mut line = std::mem::take(&mut self.line);
        line.tracked.sort_unstable();
        self.cur = None;
        let mut tracked = line.tracked.iter().copied().peekable();
        let mut k = 0;
        while k < line.cells.len() {
            let pend = &line.cells[k];
            let unit = if pend.cell.is_wide() && k + 1 < line.cells.len() { 2 } else { 1 };
            if self.cur.is_none() {
                self.open_row(line.block);
            } else if self.col + unit > width {
                self.close_row_wrapped(unit == 2 && self.col == width - 1);
                self.open_row(line.block);
            }
            let Some(serial) = self.cur else { return };
            for part in 0..unit {
                while let Some((_, t)) = tracked.next_if(|&(offset, _)| offset == k + part) {
                    self.tracks[t].result = (serial, self.col + part);
                }
                let pend = &line.cells[k + part];
                self.place(&line, pend, self.col + part);
            }
            self.col += unit;
            k += unit;
        }
        if self.cur.is_none() {
            self.open_row(line.block);
        }
        // Tracks past the text: right after it, on a new row if it filled the last.
        let len = line.cells.len();
        for (offset, t) in tracked {
            let extra = offset.saturating_sub(len);
            let target = self.col + extra;
            if target >= width && extra == 0 {
                self.close_row_wrapped(false);
                self.open_row(line.block);
                let Some(serial) = self.cur else { return };
                self.tracks[t].result = (serial, 0);
            } else {
                let Some(serial) = self.cur else { return };
                self.tracks[t].result = (serial, target.min(width - 1));
            }
        }
        if line.holds_used_end {
            self.used_end = self.next.end;
        }
        self.cur = None;
        self.line = line;
        self.line.clear();
    }

    fn close_row_wrapped(&mut self, spacer_head: bool) {
        let Some(page) = self.next.pages.back_mut() else { return };
        let last = page.rows.len() - 1;
        if spacer_head {
            let width = page.cols();
            page.put(last, width - 1, None, &[], &Style::DEFAULT, None, CellFlags::SPACER_HEAD);
        }
        page.set_wrapped(last, true);
    }

    fn place(&mut self, line: &LineBuf, pend: &Pend, col: usize) {
        let Some(page) = self.next.pages.back_mut() else { return };
        let row = page.rows.len() - 1;
        let style = if pend.style == u32::MAX { &Style::DEFAULT } else { &line.styles[pend.style as usize] };
        let (marks, link): (&[char], Option<&Hyperlink>) = match pend.aux {
            u32::MAX => (&[], None),
            ix => (&line.aux[ix as usize].marks, line.aux[ix as usize].link.as_ref()),
        };
        page.put(row, col, pend.cell.character(), marks, style, link, pend.cell.flag_set());
    }

    /// The old sequence is over: a line that never got its end (the last row was marked wrapped)
    /// ends here.
    fn flush(&mut self) {
        if !self.line.cells.is_empty() || !self.line.tracked.is_empty() {
            self.emit_line();
        }
    }
}
