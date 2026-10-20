//! What a row's cells carry beyond a [`Cell`](super::Cell): zero-width characters, an underline
//! color of their own, a hyperlink.
//!
//! Almost no cell has any of it, and the renderer's loop visits every cell of every changed row.
//! So it is not in the cell (a pointer there would make a cell 24 bytes, and copying a row would
//! clone one per cell that has it) but in a short list beside the row, sorted by column.
//! A row without any is the common case and costs one `is_empty` check; a row with some is read
//! without allocating: [`RowExtras::cell`] is a binary search over the few cells that have
//! something.
//!
//! The zero-width characters of all the row's cells sit back to back in one buffer, each cell
//! naming its slice, so copying a row makes no allocation once the row's buffers are reused,
//! however many cells have marks. Hyperlinks are runs: a link across 80 cells is one entry, not
//! 80.

use std::ops::Range;

use super::color::Color;
use super::hyperlink::Hyperlink;

/// What one cell has beyond its [`Cell`](super::Cell).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellExtra<'a> {
    /// Characters that combine with the cell's own (accents, variation selectors, the joiners of
    /// an emoji sequence), in the order they were printed.
    pub zerowidth: &'a [char],
    /// SGR 58, when it is not the text's color.
    pub underline_color: Option<Color>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    col: u32,
    underline_color: Option<Color>,
    /// Where this cell's zero-width characters start in the buffer, and how many.
    marks: u32,
    len: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LinkSpan {
    first: u32,
    end: u32,
    link: Hyperlink,
}

/// The side data of one row; see the module docs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowExtras {
    /// By column, one entry per cell that has anything.
    cells: Vec<Entry>,
    marks: Vec<char>,
    /// By first column, not overlapping.
    links: Vec<LinkSpan>,
}

impl RowExtras {
    /// Whether no cell of the row has anything beyond its [`Cell`](super::Cell).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && self.links.is_empty()
    }

    /// Bytes the row holds on the heap, for a host or the engine that keeps many rows: the lists,
    /// and the text of each link counted as if it were the row's own (rows share a link's text,
    /// so this is an upper bound).
    pub(crate) fn heap_bytes(&self) -> usize {
        let links: usize = self.links.iter().map(|span| span.link.id.len() + span.link.uri.len()).sum();
        self.cells.capacity() * size_of::<Entry>()
            + self.marks.capacity() * size_of::<char>()
            + self.links.capacity() * size_of::<LinkSpan>()
            + links
    }

    /// Forget everything, keeping the buffers for the next row copied into this one.
    pub fn clear(&mut self) {
        self.cells.clear();
        self.marks.clear();
        self.links.clear();
    }

    /// What the cell at `col` has (nothing, for most).
    #[inline]
    pub fn cell(&self, col: usize) -> CellExtra<'_> {
        if self.cells.is_empty() {
            return CellExtra::default();
        }
        self.find(col).map_or_else(CellExtra::default, |entry| self.view(entry))
    }

    /// The zero-width characters of the cell at `col`.
    #[inline]
    pub fn zerowidth(&self, col: usize) -> &[char] {
        self.cell(col).zerowidth
    }

    /// The underline color of the cell at `col`, if it has one of its own.
    #[inline]
    pub fn underline_color(&self, col: usize) -> Option<Color> {
        self.cell(col).underline_color
    }

    /// The hyperlink the cell at `col` is under.
    pub fn hyperlink(&self, col: usize) -> Option<&Hyperlink> {
        let ix = self.links.partition_point(|span| span.end as usize <= col);
        self.links.get(ix).filter(|span| span.first as usize <= col).map(|span| &span.link)
    }

    /// The cells that have something, left to right, with their columns.
    pub fn cells(&self) -> impl Iterator<Item = (usize, CellExtra<'_>)> {
        self.cells.iter().map(|entry| (entry.col as usize, self.view(entry)))
    }

    /// The hyperlinks on the row, left to right, with the columns `[first, end)` each covers.
    pub fn links(&self) -> impl Iterator<Item = (Range<usize>, &Hyperlink)> {
        self.links.iter().map(|span| (span.first as usize..span.end as usize, &span.link))
    }

    /// Add what the cell at `col` has, to the right of every cell added before. An engine copying
    /// a row calls this for the cells that have anything, in column order.
    pub fn push_cell(&mut self, col: usize, underline_color: Option<Color>, zerowidth: &[char]) {
        debug_assert!(self.cells.last().is_none_or(|last| (last.col as usize) < col), "columns ascend");
        if underline_color.is_none() && zerowidth.is_empty() {
            return;
        }
        let marks = self.marks.len() as u32;
        self.marks.extend_from_slice(zerowidth);
        self.cells.push(Entry { col: col as u32, underline_color, marks, len: zerowidth.len() as u32 });
    }

    /// Add a hyperlink over the columns `[first, end)`, to the right of every one added before.
    pub fn push_link(&mut self, first: usize, end: usize, link: Hyperlink) {
        debug_assert!(self.links.last().is_none_or(|last| (last.end as usize) <= first), "links ascend");
        if first < end {
            self.links.push(LinkSpan { first: first as u32, end: end as u32, link });
        }
    }

    /// Add one zero-width character to the cell at `col`, wherever that is in the row (tests and
    /// tools build rows in any order; an engine uses [`Self::push_cell`]).
    pub fn push_zerowidth(&mut self, col: usize, c: char) {
        let ix = self.entry_for(col);
        let entry = &mut self.cells[ix];
        self.marks.insert((entry.marks + entry.len) as usize, c);
        entry.len += 1;
        for later in &mut self.cells[ix + 1..] {
            later.marks += 1;
        }
    }

    /// Set (or, with `None`, take away) the underline color of the cell at `col`.
    pub fn set_underline_color(&mut self, col: usize, color: Option<Color>) {
        let ix = self.entry_for(col);
        self.cells[ix].underline_color = color;
        if color.is_none() && self.cells[ix].len == 0 {
            self.cells.remove(ix);
        }
    }

    fn find(&self, col: usize) -> Option<&Entry> {
        let ix = self.cells.partition_point(|entry| (entry.col as usize) < col);
        self.cells.get(ix).filter(|entry| entry.col as usize == col)
    }

    fn view<'a>(&'a self, entry: &Entry) -> CellExtra<'a> {
        let start = entry.marks as usize;
        let zerowidth = &self.marks[start..start + entry.len as usize];
        CellExtra { zerowidth, underline_color: entry.underline_color }
    }

    /// The index of the entry for `col`, made (empty) if the cell has none yet.
    fn entry_for(&mut self, col: usize) -> usize {
        let ix = self.cells.partition_point(|entry| (entry.col as usize) < col);
        if self.cells.get(ix).is_some_and(|entry| entry.col as usize == col) {
            return ix;
        }
        let marks = self.cells.get(ix).map_or(self.marks.len() as u32, |next| next.marks);
        self.cells.insert(ix, Entry { col: col as u32, underline_color: None, marks, len: 0 });
        ix
    }
}

#[cfg(test)]
mod tests {
    use super::{Hyperlink, RowExtras};
    use crate::types::{Color, NamedColor};

    #[test]
    fn a_row_without_extras_answers_nothing_without_looking() {
        let extras = RowExtras::default();
        assert!(extras.is_empty());
        assert_eq!(extras.zerowidth(3), &[] as &[char]);
        assert_eq!(extras.underline_color(3), None);
        assert!(extras.hyperlink(3).is_none());
    }

    #[test]
    fn cells_pushed_in_order_are_found_by_column() {
        let red = Color::Named(NamedColor::Red);
        let mut extras = RowExtras::default();
        extras.push_cell(1, None, &['\u{301}']);
        extras.push_cell(4, Some(red), &[]);
        extras.push_cell(7, Some(red), &['a', 'b']);
        extras.push_cell(9, None, &[]);
        let columns: Vec<usize> = extras.cells().map(|(col, _)| col).collect();
        assert_eq!(columns, [1, 4, 7], "a cell with nothing is not kept");
        assert_eq!(extras.zerowidth(1), &['\u{301}']);
        assert_eq!(extras.zerowidth(7), &['a', 'b']);
        assert_eq!(extras.underline_color(4), Some(red));
        assert_eq!(extras.underline_color(1), None);
        assert_eq!(extras.zerowidth(2), &[] as &[char]);
        assert_eq!(extras.zerowidth(100), &[] as &[char]);
    }

    #[test]
    fn marks_added_in_any_order_land_on_their_cells() {
        let mut extras = RowExtras::default();
        extras.push_zerowidth(5, 'x');
        extras.push_zerowidth(2, 'a');
        extras.push_zerowidth(5, 'y');
        extras.push_zerowidth(2, 'b');
        extras.push_zerowidth(9, 'z');
        assert_eq!(extras.zerowidth(2), &['a', 'b']);
        assert_eq!(extras.zerowidth(5), &['x', 'y']);
        assert_eq!(extras.zerowidth(9), &['z']);
        // The same row built in order is equal: a row is its content, not how it was made.
        let mut in_order = RowExtras::default();
        in_order.push_cell(2, None, &['a', 'b']);
        in_order.push_cell(5, None, &['x', 'y']);
        in_order.push_cell(9, None, &['z']);
        assert_eq!(extras, in_order);
    }

    #[test]
    fn an_underline_color_can_be_set_and_taken_away() {
        let blue = Color::Indexed(4);
        let mut extras = RowExtras::default();
        extras.set_underline_color(3, Some(blue));
        extras.push_zerowidth(3, '\u{301}');
        assert_eq!((extras.underline_color(3), extras.zerowidth(3)), (Some(blue), &['\u{301}'][..]));
        extras.set_underline_color(3, None);
        assert_eq!(extras.zerowidth(3), &['\u{301}'], "the marks stay");
        extras.set_underline_color(8, Some(blue));
        extras.set_underline_color(8, None);
        assert_eq!(extras.cells().count(), 1, "a cell left with nothing goes");
    }

    #[test]
    fn a_hyperlink_covers_a_run_of_columns() {
        let link = |uri: &str| Hyperlink { id: "a".into(), uri: uri.into() };
        let mut extras = RowExtras::default();
        extras.push_link(2, 6, link("https://a"));
        extras.push_link(6, 8, link("https://b"));
        assert!(!extras.is_empty());
        assert!(extras.hyperlink(1).is_none());
        assert_eq!(extras.hyperlink(2).map(|l| &*l.uri), Some("https://a"));
        assert_eq!(extras.hyperlink(5).map(|l| &*l.uri), Some("https://a"));
        assert_eq!(extras.hyperlink(6).map(|l| &*l.uri), Some("https://b"));
        assert!(extras.hyperlink(8).is_none());
        assert_eq!(extras.links().map(|(range, _)| range).collect::<Vec<_>>(), [2..6, 6..8]);
    }

    #[test]
    fn clearing_keeps_the_buffers() {
        let mut extras = RowExtras::default();
        extras.push_cell(0, None, &['a'; 40]);
        let capacity = extras.marks.capacity();
        extras.clear();
        assert!(extras.is_empty());
        extras.push_cell(0, None, &['b'; 40]);
        assert_eq!(extras.marks.capacity(), capacity, "the next row reuses the buffer");
    }
}
