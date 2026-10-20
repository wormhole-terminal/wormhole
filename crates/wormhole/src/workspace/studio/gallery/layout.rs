//! The gallery page: two columns that run down on their own, as they always did, and the
//! specimens that need both columns' width under them. Building all twenty specimens on every
//! wheel tick cost 24 ms, so a piece far from the viewport is a placeholder as tall as the piece
//! was when it was last built, and the layout stays what it was.
//!
//! A tall specimen comes in pieces (`Specimen::parts`), so that the part of it in view is built
//! and the rest is not. Where each piece lies is worked out here from what is known of the
//! heights above it: measured when the piece was last built, guessed (`Specimen::parts`) before.

use std::collections::HashMap;
use std::ops::Range;

use super::{Area, Column, SPECIMENS, Specimen};

/// One piece of a specimen: `part` of what `Specimen::render` makes.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) struct Piece {
    pub(super) specimen: usize,
    pub(super) part: usize,
}

/// The page's three stacks of pieces, each in page order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Columns {
    pub(super) left: Vec<Piece>,
    pub(super) right: Vec<Piece>,
    /// Under both columns.
    pub(super) wide: Vec<Piece>,
}

/// The pieces of `area`'s specimens. Where the gallery is too narrow for two columns everything
/// but the wide ones stands in the left.
pub(super) fn columns(area: Area, two_columns: bool) -> Columns {
    columns_of(SPECIMENS, area, two_columns)
}

fn columns_of(specimens: &[Specimen], area: Area, two_columns: bool) -> Columns {
    let mut page = Columns::default();
    let shown = specimens.iter().enumerate().filter(|(_, s)| area == Area::All || s.area == Some(area));
    for (ix, specimen) in shown {
        let column = match specimen.column {
            Column::Right if two_columns => &mut page.right,
            Column::Both => &mut page.wide,
            _ => &mut page.left,
        };
        column.extend((0..specimen.parts.len()).map(|part| Piece { specimen: ix, part }));
    }
    page
}

/// Under the header and above the columns, until a measure says: the page's inset, the filter row
/// and the gap under it.
pub(super) const COLUMNS_TOP: f32 = 80.0;
/// A specimen's label, its gap and the frame's inset above its first piece.
const BEFORE: f32 = 32.0;
/// The frame's inset under its last piece.
const AFTER: f32 = 4.0;
/// Between a column's specimens, and between the two columns and the wide ones.
pub(super) const SPECIMEN_GAP: f32 = 24.0;
pub(super) const BLOCK_GAP: f32 = 16.0;

/// What has been measured of the page as it was built, in points from the top of its content.
#[derive(Default)]
pub(super) struct Measures {
    /// Where the page's content begins on the window, which the probes subtract.
    pub(super) origin: f32,
    heights: HashMap<Piece, f32>,
    /// The tops of the two columns and of the wide ones.
    blocks: [Option<f32>; 2],
}

impl Measures {
    /// Forget it all: the page is a different one (another area, another width).
    pub(super) fn reset(&mut self) {
        self.heights.clear();
        self.blocks = [None; 2];
    }

    /// A piece's height as it was built; whether that is news.
    pub(super) fn height_of(&mut self, piece: Piece, height: f32) -> bool {
        self.heights.insert(piece, height).is_none_or(|was| (was - height).abs() > 0.5)
    }

    /// The top of the columns (0) or of the wide specimens (1); whether that is news.
    pub(super) fn block_top(&mut self, block: usize, top: f32) -> bool {
        self.blocks[block].replace(top).is_none_or(|was| (was - top).abs() > 0.5)
    }

    /// How tall a piece is: as built, else as guessed.
    pub(super) fn height(&self, piece: Piece) -> f32 {
        self.heights.get(&piece).copied().unwrap_or(SPECIMENS[piece.specimen].parts[piece.part])
    }

    pub(super) fn columns_top(&self) -> f32 {
        self.blocks[0].unwrap_or(COLUMNS_TOP)
    }

    /// The top of the wide specimens, given where the two columns end.
    pub(super) fn wide_top(&self, columns_bottom: f32) -> f32 {
        self.blocks[1].unwrap_or(columns_bottom + BLOCK_GAP)
    }
}

/// Where a column's pieces lie, from `top` down, and where it ends.
pub(super) fn extents(column: &[Piece], top: f32, height: impl Fn(Piece) -> f32) -> (Vec<Range<f32>>, f32) {
    let mut at = top;
    let mut previous: Option<Piece> = None;
    let spans = column
        .iter()
        .map(|&piece| {
            at += match previous {
                Some(before) if before.specimen == piece.specimen => SPECIMENS[piece.specimen].part_gap,
                Some(_) => AFTER + SPECIMEN_GAP + BEFORE,
                None => BEFORE,
            };
            previous = Some(piece);
            let span = at..at + height(piece);
            at = span.end;
            span
        })
        .collect();
    (spans, if previous.is_some() { at + AFTER } else { top })
}

/// Which of `spans` meet `window`.
pub(super) fn meeting(spans: &[Range<f32>], window: &Range<f32>) -> Vec<bool> {
    spans.iter().map(|s| s.start < window.end && s.end > window.start).collect()
}

#[cfg(test)]
mod tests {
    use super::{Area, Columns, Piece, SPECIMENS, columns, extents, meeting};

    fn all(page: &Columns) -> Vec<Piece> {
        let mut all: Vec<Piece> =
            [&page.left, &page.right, &page.wide].into_iter().flatten().copied().collect();
        all.sort_by_key(|p| (p.specimen, p.part));
        all
    }

    fn wanted(area: Area) -> Vec<Piece> {
        (0..SPECIMENS.len())
            .filter(|&ix| area == Area::All || SPECIMENS[ix].area == Some(area))
            .flat_map(|specimen| {
                (0..SPECIMENS[specimen].parts.len()).map(move |part| Piece { specimen, part })
            })
            .collect()
    }

    #[test]
    fn every_piece_is_on_the_page_once() {
        for two_columns in [true, false] {
            assert_eq!(all(&columns(Area::All, two_columns)), wanted(Area::All));
        }
    }

    #[test]
    fn an_area_keeps_its_own_specimens() {
        for area in Area::ALL {
            assert_eq!(all(&columns(area, true)), wanted(area), "{area:?}");
        }
    }

    #[test]
    fn the_columns_are_what_the_table_says() {
        let page = columns(Area::All, true);
        assert_eq!(page.left[0], Piece { specimen: 0, part: 0 }, "the sidebar leads the left");
        assert_eq!(page.right[0], Piece { specimen: 2, part: 0 }, "the panel leads the right");
        assert!(page.wide.iter().all(|p| SPECIMENS[p.specimen].title != "Sidebar"));
        assert!(columns(Area::All, false).right.is_empty(), "a narrow gallery has one column");
    }

    #[test]
    fn pieces_lie_one_under_the_other_with_their_gaps() {
        let page = columns(Area::All, true);
        let (spans, bottom) = extents(&page.left, 100.0, |_| 50.0);
        assert!(spans[0].start > 100.0, "the first specimen's label is above it");
        assert!(spans.windows(2).all(|w| w[0].end < w[1].start), "no overlaps");
        let last = spans.last().expect("a piece");
        assert!(bottom > last.end);
        assert_eq!(extents(&[], 100.0, |_| 1.0), (vec![], 100.0));
    }

    #[test]
    fn only_what_meets_the_window_is_built() {
        let page = columns(Area::All, true);
        let (spans, _) = extents(&page.left, 80.0, |p| SPECIMENS[p.specimen].parts[p.part]);
        let near = meeting(&spans, &(0.0..900.0));
        assert!(near[0] && !near[near.len() - 1]);
        assert!(near.iter().filter(|n| **n).count() < near.len() / 2);
    }
}
