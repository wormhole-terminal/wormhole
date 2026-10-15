//! Rectangles of the live screen: the operations behind DECCARA, DECRARA, DECCRA, DECFRA, DECERA,
//! DECSERA and the checksum of DECRQCRA.
//!
//! The terminal state works out which cells a sequence means (origin mode, defaults, clipping) and
//! hands a [`Region`] that lies inside the screen; nothing here reads the margins. Cells that a
//! rectangle cuts out of a wide character are left as blanks, as everywhere in the grid.

use std::ops::Range;

use crate::types::Color;

use super::{Grid, Pen, Region, Style};

impl Grid {
    /// Change the look of the cells `x` of live row `y` to what `change` makes of it; the
    /// characters stay.
    pub fn restyle(&mut self, y: usize, x: Range<usize>, change: impl Fn(&Style) -> Style) {
        if y < self.rows && x.start < x.end.min(self.cols) {
            let (page, row) = self.page_row_mut(y);
            page.restyle(row, x, change);
        }
    }

    /// DECFRA: write `c`, one cell wide, in every cell of `area` with `pen`'s look. Pictures under
    /// the area are deleted.
    pub fn fill_rect(&mut self, area: Region, c: char, pen: &Pen) {
        let area = self.inside(area);
        self.placements_erased(area.top..area.bottom + 1, area.left..area.right + 1);
        for y in area.top..=area.bottom {
            for x in area.left..=area.right {
                self.write_cluster(y, x, c, &[], 1, pen);
            }
        }
    }

    /// DECERA and DECSERA: blank every cell of `area` with `bg` as the background; `selective`
    /// leaves the protected ones.
    pub fn erase_rect(&mut self, area: Region, bg: Color, selective: bool) {
        let area = self.inside(area);
        for y in area.top..=area.bottom {
            self.erase(y, area.left..area.right + 1, bg, selective);
        }
    }

    /// DECCRA: copy the cells of `from` so that its top left corner lands on `to_x`, `to_y`. What
    /// does not fit on the screen is left out; the source and the target may overlap (the result is
    /// as if the source had been read whole first). Looks, links and marks go with the characters.
    pub fn copy_rect(&mut self, from: Region, to_y: usize, to_x: usize) {
        let from = self.inside(from);
        if to_y >= self.rows || to_x >= self.cols {
            return;
        }
        let height = (from.bottom - from.top + 1).min(self.rows - to_y);
        let width = (from.right - from.left + 1).min(self.cols - to_x);
        self.placements_erased(to_y..to_y + height, to_x..to_x + width);
        let (mut cells, mut marks) =
            (std::mem::take(&mut self.copied), std::mem::take(&mut self.copied_marks));
        // Moving down, the bottom row goes first: a row is read before anything is written to it.
        let downwards = to_y > from.top;
        for k in 0..height {
            let k = if downwards { height - 1 - k } else { k };
            cells.clear();
            marks.clear();
            let (page, row) = self.locate_y(from.top + k);
            self.pages[page].snapshot(row, from.left..from.left + width, &mut cells, &mut marks);
            let (page, row) = self.locate_y(to_y + k);
            self.pages[page].store(row, to_x, &cells, &marks);
        }
        (self.copied, self.copied_marks) = (cells, marks);
    }

    /// DECRQCRA: the 16-bit sum of the code points of the cells of `area`, a blank counting as a
    /// space and the second half of a wide character as nothing. Looks do not count.
    pub fn checksum(&self, area: Region) -> u16 {
        let area = self.inside(area);
        let mut sum = 0u32;
        for y in area.top..=area.bottom {
            let (page, row) = self.locate_y(y);
            for cell in &self.pages[page].cells(row)[area.left..=area.right] {
                sum = sum.wrapping_add(match cell.codepoint() {
                    _ if cell.is_spacer_tail() => 0,
                    0 => u32::from(b' '),
                    cp => cp,
                });
            }
        }
        sum as u16
    }

    /// `area` cut to the screen, so that nothing below indexes outside it.
    fn inside(&self, area: Region) -> Region {
        let (last_x, last_y) = (self.cols - 1, self.rows - 1);
        let (top, left) = (area.top.min(last_y), area.left.min(last_x));
        Region { top, left, bottom: area.bottom.clamp(top, last_y), right: area.right.clamp(left, last_x) }
    }
}
