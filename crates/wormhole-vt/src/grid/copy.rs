//! The snapshot copy: rows of the grid as the types a host draws.
//!
//! This runs under the lock the parser needs, so it is the loop the grid is shaped for. A cell is
//! eight bytes whose style is a table lookup, so a row converts in one pass without allocating;
//! blank cells (all-zero words, most of a terminal's area) skip the lookup; what is rare (the
//! characters after a cluster's first, hyperlinks, underline colors) is looked for in a second pass
//! that runs only when the first saw a flag or the page has an underline color at all.

use crate::types::{Cell as DrawCell, Color, Flags, Line, NamedColor, Point, RowExtras};

use super::cell::{Cell, CellFlags};
use super::page::Page;
use super::style::Style;
use super::{Grid, RowMark};

/// One row of the view as [`Terminal::copy_viewport`](crate::Terminal::copy_viewport) fills it:
/// the cells to draw, what they carry besides, and whether the row was copied again by that call.
/// Keep the rows between calls: their buffers are reused.
#[derive(Default)]
#[non_exhaustive]
pub struct ViewRow {
    /// One cell per column.
    pub cells: Vec<DrawCell>,
    /// Zero-width characters, underline colors and hyperlinks of the row's cells.
    pub extras: RowExtras,
    /// The block (OSC 133) the row carries a part of.
    pub mark: Option<RowMark>,
    /// The line goes on in the next row.
    pub wrapped: bool,
    /// The row was copied anew; the others are certainly the same as before.
    pub changed: bool,
}

const BLANK: DrawCell = DrawCell {
    c: ' ',
    fg: Color::Named(NamedColor::Foreground),
    bg: Color::Named(NamedColor::Background),
    flags: Flags::empty(),
};

impl Grid {
    /// Copy the rows of the view (the live screen, or the history if the view is scrolled up) into
    /// `out`, which is resized to the screen's height. Only rows that changed since the last call
    /// are copied, all of them after `force`, a resize, a clear or a move of the view; every row's
    /// `changed` says which. Clears the rows' changed marks.
    pub fn copy_viewport(&mut self, out: &mut Vec<ViewRow>, force: bool) {
        let full = force
            || self.all_dirty
            || out.len() != self.rows
            || out.first().is_some_and(|r| r.cells.len() != self.cols);
        out.resize_with(self.rows, ViewRow::default);
        for (y, view) in out.iter_mut().enumerate() {
            let line = Line(y as i32 - self.offset as i32);
            let Some((page_ix, row)) = self.locate_line(line) else {
                view.cells.clear();
                view.changed = true;
                continue;
            };
            let page = &mut self.pages[page_ix];
            view.wrapped = page.rows[row].wrapped();
            view.changed = page.take_dirty(row) || full;
            if view.changed {
                view.mark = copy_page_row(page, row, &mut view.cells, Some(&mut view.extras));
            }
        }
        self.all_dirty = false;
    }

    /// Which rows of the view changed since the last call, as `damaged[y]`, clearing the marks.
    /// True when everything did (the view moved, a resize, a clear), in which case `damaged` says
    /// nothing.
    pub fn take_damage(&mut self, damaged: &mut [bool]) -> bool {
        let all = std::mem::take(&mut self.all_dirty);
        for (y, flag) in damaged.iter_mut().enumerate().take(self.rows) {
            let line = Line(y as i32 - self.offset as i32);
            *flag = false;
            if let Some((page, row)) = self.locate_line(line) {
                *flag = self.pages[page].take_dirty(row);
            }
        }
        all
    }

    /// The cell at `point` (history included) as `copy_row` gives it; `None` outside the grid.
    pub fn draw_cell(&self, point: Point) -> Option<DrawCell> {
        if point.column.0 >= self.cols {
            return None;
        }
        let (page_ix, row) = self.locate_line(point.line)?;
        let page = &self.pages[page_ix];
        let x = point.column.0;
        let cell = page.cells(row)[x];
        let mut drawn = if cell.is_blank() {
            BLANK
        } else {
            let style = page.styles.styles()[cell.style_id() as usize].style();
            DrawCell {
                c: drawn_char(cell.codepoint()),
                fg: style.fg,
                bg: style.bg,
                flags: style.attrs | cell.layout_flags(),
            }
        };
        // A soft-wrapped row marks its last cell, blank or not, as `copy_row` does.
        if x + 1 == self.cols && page.rows[row].wrapped() {
            drawn.flags |= Flags::WRAPLINE;
        }
        Some(drawn)
    }

    /// Copy the row at `line` (history included) into `cells`, and what its cells carry beside
    /// them into `extras` when given. Returns the block mark the row carries. Doesn't touch the
    /// changed marks.
    pub fn copy_row(
        &self,
        line: Line,
        cells: &mut Vec<DrawCell>,
        extras: Option<&mut RowExtras>,
    ) -> Option<RowMark> {
        let Some((page, row)) = self.locate_line(line) else {
            cells.clear();
            if let Some(extras) = extras {
                extras.clear();
            }
            return None;
        };
        copy_page_row(&self.pages[page], row, cells, extras)
    }
}

/// The character a cell's code point is drawn as. ASCII, nearly all text, needs no validity check;
/// an empty cell (an erased background, a spacer) is a space.
#[inline]
fn drawn_char(cp: u32) -> char {
    if cp < 0x80 {
        char::from(cp as u8 | (u8::from(cp == 0) * 0x20))
    } else {
        char::from_u32(cp).unwrap_or(' ')
    }
}

/// Convert row `row` of `page`.
fn copy_page_row(
    page: &Page,
    row: usize,
    out: &mut Vec<DrawCell>,
    extras: Option<&mut RowExtras>,
) -> Option<RowMark> {
    let src = page.cells(row);
    if out.len() != src.len() {
        out.clear();
        out.resize(src.len(), BLANK);
    }
    let table = page.styles.styles();
    // The OR of the words of the cells that aren't blank: whether any carries a flag worth a look.
    let mut seen = 0u64;
    // The style of the previous cell: text comes in runs of one look, so the table is read when it
    // changes. A blank cell (the zero word, most of the screen) is a constant.
    let mut last = u64::MAX;
    let (mut fg, mut bg, mut attrs) = (BLANK.fg, BLANK.bg, Flags::empty());
    for (dst, &cell) in out.iter_mut().zip(src) {
        if cell.is_blank() {
            *dst = BLANK;
            continue;
        }
        seen |= cell.bits();
        let key = cell.style_bits();
        if key != last {
            let style = table[cell.style_id() as usize].style();
            (fg, bg, attrs) = (style.fg, style.bg, style.attrs);
            last = key;
        }
        *dst = DrawCell { c: drawn_char(cell.codepoint()), fg, bg, flags: attrs | cell.layout_flags() };
    }
    if let Some(last) = out.last_mut().filter(|_| page.rows[row].wrapped()) {
        last.flags |= Flags::WRAPLINE;
    }
    if let Some(extras) = extras {
        extras.clear();
        let flagged = CellFlags::in_word(seen).intersects(CellFlags::GRAPHEME | CellFlags::HYPERLINK);
        if flagged || page.styles.has_colored_underlines() {
            fill_extras(page, row, src, extras);
        }
    }
    page.row_mark(row)
}

/// The rare things of a row: marks, underline colors, hyperlink runs.
fn fill_extras(page: &Page, row: usize, cells: &[Cell], extras: &mut RowExtras) {
    for (x, cell) in cells.iter().enumerate() {
        let underline: Option<Color> = match cell.style_id() {
            0 => None,
            id => page.styles.get(id).underline_color,
        };
        let marks: &[char] = if cell.has_grapheme() { page.graphemes.get(page.offset(row, x)) } else { &[] };
        extras.push_cell(x, underline, marks);
    }
    // The page keeps links as runs, so the row's are read off, not found cell by cell.
    let first = page.offset(row, 0);
    for (lo, hi, id) in page.links.within(first, first + cells.len() as u32) {
        extras.push_link((lo - first) as usize, (hi - first) as usize, page.links.link(id).clone());
    }
}

impl Style {
    /// The cell this style gives a character, for tests comparing against the copy.
    #[cfg(test)]
    pub(crate) fn draw(&self, c: char) -> DrawCell {
        DrawCell { c, fg: self.fg, bg: self.bg, flags: self.attrs }
    }
}
