//! The gallery page as the list's items. A plain scrolling div would build every specimen
//! on every wheel tick (twenty of them, 24 ms a frame); the list builds the ones in view. Its
//! items are rows of the page: the filter, a stack of pieces in each column side by side, and a
//! piece across both columns.
//!
//! A specimen comes in pieces when it is tall (`Specimen::parts`). The page was two columns that
//! each ran down on their own; a list can't do that, its rows are as tall as the taller column. So
//! a row takes as many pieces of each column as it needs to come out about even, by the pieces'
//! guessed heights, and the shorter column's gap stays small.

use super::{Area, Column, SPECIMENS, Specimen};

/// One piece of a specimen: `part` of what `Specimen::render` makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Piece {
    pub(super) specimen: usize,
    pub(super) part: usize,
}

/// One item of the gallery's list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Row {
    /// The area filter and what the selected token is drawn on.
    Header,
    /// The left column's pieces beside the right column's; a column with nothing left stays
    /// empty, so the other keeps its half of the page.
    Pair(Vec<Piece>, Vec<Piece>),
    /// A piece across both columns.
    Wide(Piece),
}

impl Piece {
    /// Whether the piece carries on a specimen the one above it began (it has no title).
    pub(super) fn continues(self) -> bool {
        self.part > 0
    }
}

/// The rows of the page for `area`: the header, the left and right columns' specimens in rows that
/// are about even, then those that need both columns' width. Where the gallery is too narrow for
/// two columns everything but the wide ones stands in the left.
pub(super) fn rows(area: Area, two_columns: bool) -> Vec<Row> {
    rows_of(SPECIMENS, area, two_columns)
}

fn rows_of(specimens: &[Specimen], area: Area, two_columns: bool) -> Vec<Row> {
    let (mut left, mut right, mut wide) = (Vec::new(), Vec::new(), Vec::new());
    let shown = specimens.iter().enumerate().filter(|(_, s)| area == Area::All || s.area == Some(area));
    for (ix, specimen) in shown {
        let column = match specimen.column {
            Column::Right if two_columns => &mut right,
            Column::Both => &mut wide,
            _ => &mut left,
        };
        column.extend((0..specimen.parts.len()).map(|part| Piece { specimen: ix, part }));
    }
    let height = |piece: &Piece| specimens[piece.specimen].parts[piece.part];
    let (mut left, mut right) = (left.into_iter().peekable(), right.into_iter().peekable());
    let mut rows = vec![Row::Header];
    while left.peek().is_some() || right.peek().is_some() {
        let (mut l, mut r) = (Vec::new(), Vec::new());
        let (mut lh, mut rh) = (0.0, 0.0);
        if let Some(piece) = left.next() {
            lh += height(&piece);
            l.push(piece);
        }
        if let Some(piece) = right.next() {
            rh += height(&piece);
            r.push(piece);
        }
        // The shorter column takes its next piece while that brings it closer to the other.
        loop {
            let left_short = lh <= rh;
            let (queue, own, other) = if left_short { (&mut left, lh, rh) } else { (&mut right, rh, lh) };
            let Some(next) = queue.peek() else { break };
            let taller = height(next);
            if (own + taller - other).abs() >= other - own {
                break;
            }
            let piece = queue.next().expect("peeked");
            if left_short {
                lh += taller;
                l.push(piece);
            } else {
                rh += taller;
                r.push(piece);
            }
        }
        rows.push(Row::Pair(l, r));
    }
    rows.extend(wide.into_iter().map(Row::Wide));
    rows
}

#[cfg(test)]
mod tests {
    use super::{Area, Piece, Row, SPECIMENS, rows};

    fn pieces(rows: &[Row]) -> Vec<Piece> {
        rows.iter()
            .flat_map(|row| match row {
                Row::Header => vec![],
                Row::Pair(left, right) => left.iter().chain(right).copied().collect(),
                Row::Wide(piece) => vec![*piece],
            })
            .collect()
    }

    fn wanted(area: Area) -> Vec<Piece> {
        let mut all: Vec<Piece> = (0..SPECIMENS.len())
            .filter(|&ix| area == Area::All || SPECIMENS[ix].area == Some(area))
            .flat_map(|specimen| {
                (0..SPECIMENS[specimen].parts.len()).map(move |part| Piece { specimen, part })
            })
            .collect();
        all.sort_by_key(|p| (p.specimen, p.part));
        all
    }

    #[test]
    fn every_piece_is_on_the_page_once() {
        for two_columns in [true, false] {
            let mut shown = pieces(&rows(Area::All, two_columns));
            shown.sort_by_key(|p| (p.specimen, p.part));
            assert_eq!(shown, wanted(Area::All));
        }
    }

    #[test]
    fn an_area_keeps_its_own_specimens_and_the_header_leads() {
        for area in Area::ALL {
            let page = rows(area, true);
            assert_eq!(page[0], Row::Header);
            let mut shown = pieces(&page);
            shown.sort_by_key(|p| (p.specimen, p.part));
            assert_eq!(shown, wanted(area), "{area:?}");
        }
    }

    /// A specimen's pieces run down the page in their order, so it reads as it was written.
    #[test]
    fn a_specimens_pieces_keep_their_order() {
        let page = rows(Area::All, true);
        for specimen in 0..SPECIMENS.len() {
            let order: Vec<usize> = page
                .iter()
                .flat_map(|row| match row {
                    Row::Pair(left, right) => left.iter().chain(right).copied().collect(),
                    Row::Wide(piece) => vec![*piece],
                    Row::Header => vec![],
                })
                .filter(|p| p.specimen == specimen)
                .map(|p| p.part)
                .collect();
            assert!(order.windows(2).all(|w| w[0] < w[1]), "specimen {specimen}: {order:?}");
        }
    }

    #[test]
    fn a_narrow_gallery_has_nothing_in_the_right_column() {
        let page = rows(Area::All, false);
        assert!(page.iter().all(|row| !matches!(row, Row::Pair(_, right) if !right.is_empty())));
    }

    /// The point of balancing: where the page is two columns, a row's shorter side is not left far
    /// behind. Counted in the pieces' guessed heights, over the rows that have both.
    #[test]
    fn the_two_columns_come_out_about_even() {
        let height = |p: &Piece| SPECIMENS[p.specimen].parts[p.part];
        let (mut gap, mut total) = (0.0, 0.0);
        for row in rows(Area::All, true) {
            // Past the end of the shorter column the longer one runs on alone, as it did before.
            if let Row::Pair(left, right) = row
                && !left.is_empty()
                && !right.is_empty()
            {
                let (l, r): (f32, f32) = (left.iter().map(height).sum(), right.iter().map(height).sum());
                gap += (l - r).abs();
                total += l.max(r);
            }
        }
        assert!(gap < total / 4.0, "{gap} of {total} points is empty");
    }

    #[test]
    fn the_columns_stand_side_by_side() {
        let page = rows(Area::All, true);
        // The sidebar's first piece (left) beside the right panel's (right), as the page opens.
        assert_eq!(
            page[1],
            Row::Pair(vec![Piece { specimen: 0, part: 0 }], vec![Piece { specimen: 2, part: 0 }])
        );
    }
}
