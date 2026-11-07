//! Which rows a view shows when folds are in it: a walk over the grid's lines in which a collapsed
//! fold is one marker row, and an expanded one a marker row above its own lines. Pure: the folds
//! come as resolved spans, so every rule here is tested on plain line numbers.
//!
//! The view is anchored at its bottom line, as the engine's `display_offset` is, so no folds in
//! view is exactly the grid's own rows. Scrolling is counted in shown rows (a fold of 124 lines is
//! one step), which is why scrolling converts through [`step`].

use wormhole_vt::Line;

use super::FoldSpan;

/// One shown row: a grid line, or the marker of the fold at that index in the spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shown {
    Line(Line),
    Marker(usize),
}

/// The spans, sorted by `first` and without overlap, as the walk reads them.
pub(crate) struct Walk<'a> {
    spans: &'a [FoldSpan],
    topmost: Line,
    bottommost: Line,
}

impl<'a> Walk<'a> {
    pub(crate) fn new(spans: &'a [FoldSpan], topmost: Line, bottommost: Line) -> Self {
        Self { spans, topmost, bottommost }
    }

    /// The span that holds `line`.
    fn span_at(&self, line: Line) -> Option<usize> {
        let at = self.spans.partition_point(|span| span.first <= line).checked_sub(1)?;
        (self.spans[at].last >= line).then_some(at)
    }

    /// What shows `line`: its own row, or the marker of the collapsed fold it is hidden in.
    pub(crate) fn item_at(&self, line: Line) -> Shown {
        match self.span_at(line) {
            Some(ix) if !self.spans[ix].expanded => Shown::Marker(ix),
            _ => Shown::Line(line),
        }
    }

    /// The grid line an item stands for (a marker: its fold's first line).
    pub(crate) fn line_of(&self, item: Shown) -> Line {
        match item {
            Shown::Line(line) => line,
            Shown::Marker(ix) => self.spans[ix].first,
        }
    }

    /// The row above `item`, if the grid has one.
    fn up(&self, item: Shown) -> Option<Shown> {
        let above = match item {
            Shown::Marker(ix) => self.spans[ix].first - 1,
            Shown::Line(line) => match self.span_at(line) {
                // The first line of an expanded fold sits under its marker.
                Some(ix) if self.spans[ix].first == line => return Some(Shown::Marker(ix)),
                _ => line - 1,
            },
        };
        (above >= self.topmost).then(|| self.item_at(above))
    }

    /// The row below `item`, if the grid has one.
    fn down(&self, item: Shown) -> Option<Shown> {
        let below = match item {
            Shown::Marker(ix) if self.spans[ix].expanded => return Some(Shown::Line(self.spans[ix].first)),
            Shown::Marker(ix) => self.spans[ix].last + 1,
            Shown::Line(line) => line + 1,
        };
        if below > self.bottommost {
            return None;
        }
        // A fold whose first line this is shows its marker first.
        match self.spans.binary_search_by(|span| span.first.cmp(&below)) {
            Ok(ix) => Some(Shown::Marker(ix)),
            Err(_) => Some(self.item_at(below)),
        }
    }

    /// `rows` shown rows ending at the one that shows `bottom`, top first. When the history is not
    /// long enough above to fill them, the view is topped up from below.
    pub(crate) fn view(&self, bottom: Line, rows: usize, out: &mut Vec<Shown>) {
        out.clear();
        let last = self.item_at(bottom);
        let mut item = Some(last);
        while let Some(shown) = item.filter(|_| out.len() < rows) {
            out.push(shown);
            item = self.up(shown);
        }
        out.reverse();
        let mut item = out.last().and_then(|&shown| self.down(shown));
        while let Some(shown) = item.filter(|_| out.len() < rows) {
            out.push(shown);
            item = self.down(shown);
        }
    }

    /// The line `rows` shown rows above (`up`) or below the row that shows `from`, as far as the
    /// grid goes. A marker counts as its fold's last line, the one the view is anchored at.
    pub(crate) fn step(&self, from: Line, rows: usize, up: bool) -> Line {
        let mut item = self.item_at(from);
        for _ in 0..rows {
            let next = if up { self.up(item) } else { self.down(item) };
            match next {
                Some(next) => item = next,
                None => break,
            }
        }
        match item {
            Shown::Line(line) => line,
            Shown::Marker(ix) => self.spans[ix].last,
        }
    }
}

/// Whether a picture covering grid lines `top..=bottom` runs over a marker: a collapsed fold it overlaps,
/// or the marker row an open fold puts into the picture's middle. It is clipped away (not drawn), since
/// the rows it would cover are no longer the grid's rows in a row.
pub(crate) fn crosses_marker(spans: &[FoldSpan], top: Line, bottom: Line) -> bool {
    spans.iter().any(|span| {
        if span.expanded {
            span.first > top && span.first <= bottom
        } else {
            span.first <= bottom && top <= span.last
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(first: i32, last: i32, expanded: bool) -> FoldSpan {
        FoldSpan { id: 1, first: Line(first), last: Line(last), expanded, failed: false, owner: None }
    }

    fn shown(spans: &[FoldSpan], bottom: i32, rows: usize) -> Vec<String> {
        let walk = Walk::new(spans, Line(-20), Line(5));
        let mut out = Vec::new();
        walk.view(Line(bottom), rows, &mut out);
        out.iter()
            .map(|item| match item {
                Shown::Line(line) => line.0.to_string(),
                Shown::Marker(ix) => format!("M{ix}"),
            })
            .collect()
    }

    #[test]
    fn without_folds_the_view_is_the_grids_own_rows() {
        assert_eq!(shown(&[], 2, 4), ["-1", "0", "1", "2"]);
    }

    #[test]
    fn a_collapsed_fold_is_one_row_and_an_expanded_one_has_a_marker_above_its_lines() {
        let spans = [span(-8, -4, false)];
        assert_eq!(shown(&spans, 0, 5), ["M0", "-3", "-2", "-1", "0"]);
        let spans = [span(-8, -6, true)];
        assert_eq!(shown(&spans, -4, 6), ["M0", "-8", "-7", "-6", "-5", "-4"]);
    }

    #[test]
    fn the_view_is_anchored_at_its_bottom_even_inside_a_fold() {
        let spans = [span(-8, -4, false)];
        assert_eq!(shown(&spans, -6, 3), ["-10", "-9", "M0"]);
    }

    #[test]
    fn a_short_history_tops_the_view_up_from_below() {
        let spans = [span(-20, -2, false)];
        let walk = Walk::new(&spans, Line(-20), Line(1));
        let mut out = Vec::new();
        walk.view(Line(-19), 4, &mut out);
        assert_eq!(
            out,
            [Shown::Marker(0), Shown::Line(Line(-1)), Shown::Line(Line(0)), Shown::Line(Line(1))]
        );
    }

    #[test]
    fn a_picture_that_runs_over_a_marker_is_clipped() {
        let (collapsed, open) = ([span(-8, -4, false)], [span(-8, -4, true)]);
        let across = |spans: &[FoldSpan], top, bottom| crosses_marker(spans, Line(top), Line(bottom));
        assert!(across(&collapsed, -10, -7) && across(&collapsed, -5, -2) && across(&collapsed, -7, -6));
        assert!(!across(&collapsed, -12, -9) && !across(&collapsed, -3, 0), "above and below");
        assert!(across(&open, -10, -8), "the marker would sit in the middle of it");
        assert!(!across(&open, -8, -5) && !across(&open, -6, -4), "inside the open fold, below its marker");
    }

    #[test]
    fn stepping_counts_shown_rows() {
        let spans = [span(-60, -4, false)];
        let walk = Walk::new(&spans, Line(-100), Line(5));
        assert_eq!(walk.step(Line(0), 4, true), Line(-4), "three lines, then the fold: its last line");
        assert_eq!(walk.step(Line(0), 5, true), Line(-61), "the next row above the fold");
        assert_eq!(walk.step(Line(-61), 1, false), Line(-4), "back onto the fold");
        assert_eq!(walk.step(Line(-4), 1, false), Line(-3));
    }
}
