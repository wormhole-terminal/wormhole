//! How far a hyperlink reaches from a cell: what hovering and clicking a link needs.
//!
//! A hyperlink (OSC 8) covers the cells printed under it, which may be several rows when its text
//! wrapped. The page keeps a link as runs of cells, so a row's part of it is found without
//! visiting cells; the walk across rows is bounded because the text is a program's.

use crate::types::{Column, Hyperlink, Line, Point};

use super::Grid;

/// The most rows a link is followed over, each way.
const MAX_LINK_ROWS: usize = 16;

/// A hyperlink and the cells it covers around a point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkSpan {
    pub link: Hyperlink,
    /// The first cell of the link.
    pub start: Point,
    /// The last cell of the link.
    pub end: Point,
}

impl Grid {
    /// The hyperlink at `point` and the cells it covers, following it over soft-wrapped rows
    /// (up to [`MAX_LINK_ROWS`] each way). `None` for a cell that is under no link.
    pub fn link_span(&self, point: Point) -> Option<LinkSpan> {
        let link = self.link_at(point)?;
        let (mut first, mut last) = self.link_run(point.line, point.column.0, &link)?;
        let (mut start_line, mut end_line) = (point.line, point.line);
        for _ in 0..MAX_LINK_ROWS {
            if first > 0 || start_line <= self.topmost() || !self.is_wrapped(start_line - 1) {
                break;
            }
            let above = start_line - 1;
            // The run has to reach the row's end, or the link only touches the wrap by chance.
            match self.link_run(above, self.cols - 1, &link) {
                Some((lo, hi)) if hi + 1 == self.cols => (start_line, first) = (above, lo),
                _ => break,
            }
        }
        for _ in 0..MAX_LINK_ROWS {
            if last + 1 < self.cols || end_line >= self.bottommost() || !self.is_wrapped(end_line) {
                break;
            }
            let below = end_line + 1;
            match self.link_run(below, 0, &link) {
                Some((0, hi)) => (end_line, last) = (below, hi),
                _ => break,
            }
        }
        Some(LinkSpan {
            link,
            start: Point::new(start_line, Column(first)),
            end: Point::new(end_line, Column(last)),
        })
    }

    /// The first and last column of the run of `link` that holds column `col` of `line`.
    fn link_run(&self, line: Line, col: usize, link: &Hyperlink) -> Option<(usize, usize)> {
        let (page_ix, row) = self.locate_line(line)?;
        let page = &self.pages[page_ix];
        let first = page.offset(row, 0);
        page.links
            .within(first, first + self.cols as u32)
            .find(|&(lo, hi, id)| {
                (lo - first) as usize <= col && col < (hi - first) as usize && page.links.link(id) == link
            })
            .map(|(lo, hi, _)| ((lo - first) as usize, (hi - first) as usize - 1))
    }
}
