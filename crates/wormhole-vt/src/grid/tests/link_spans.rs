//! How far a hyperlink reaches from a cell: over the rows its line was wrapped across, and no
//! further than its own cells.

use super::{at, plain};
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid, Pen};
use crate::types::Hyperlink;

fn grid(cols: usize, rows: usize) -> Grid {
    Grid::new(cols, rows, DEFAULT_SCROLLBACK_BYTES)
}

fn under(uri: &str) -> Pen {
    Pen { link: Some(Hyperlink { id: uri.into(), uri: uri.into() }), ..plain() }
}

/// The first and last cell of the link at (`line`, `col`), as (line, column) pairs.
fn reach(grid: &Grid, line: i32, col: usize) -> ((i32, usize), (i32, usize)) {
    let span = grid.link_span(at(line, col)).expect("a cell under a link");
    ((span.start.line.0, span.start.column.0), (span.end.line.0, span.end.column.0))
}

#[test]
fn a_link_that_starts_inside_its_row_does_not_reach_back_to_the_row_before() {
    let mut g = grid(10, 4);
    let link = under("https://a");
    // Row 0 ends under the link and goes on in row 1, where the link begins again after two
    // other cells: those two are a gap, and the pieces are two spans.
    g.write_ascii(0, 7, b"xyz", &link);
    g.set_wrapped(0, true);
    g.write_ascii(1, 2, b"cd", &link);
    assert_eq!(reach(&g, 1, 3), ((1, 2), (1, 3)));
    // The same the other way: a link that stops short of its row's end is not joined to the next.
    g.write_ascii(2, 2, b"abc", &link);
    g.set_wrapped(2, true);
    g.write_ascii(3, 0, b"xy", &link);
    assert_eq!(reach(&g, 2, 3), ((2, 2), (2, 4)));
}

#[test]
fn a_link_does_not_reach_over_a_row_that_was_not_wrapped() {
    let mut g = grid(10, 3);
    let link = under("https://a");
    // The line was ended there (a newline), so the link that ends the row and the one that
    // begins the next are two.
    g.write_ascii(0, 7, b"xyz", &link);
    g.write_ascii(1, 0, b"abc", &link);
    assert_eq!(reach(&g, 1, 1), ((1, 0), (1, 2)));
    assert_eq!(reach(&g, 0, 8), ((0, 7), (0, 9)));
}

#[test]
fn a_link_wrapped_across_rows_is_found_from_each_of_them() {
    let mut g = grid(10, 3);
    let link = under("https://a");
    g.write_ascii(0, 7, b"xyz", &link);
    g.set_wrapped(0, true);
    g.write_ascii(1, 0, b"abc", &link);
    assert_eq!(reach(&g, 0, 8), ((0, 7), (1, 2)));
    assert_eq!(reach(&g, 1, 1), ((0, 7), (1, 2)));
}

#[test]
fn a_link_that_comes_twice_in_a_row_is_two_runs_and_a_cell_is_in_one() {
    let mut g = grid(12, 3);
    let link = under("https://a");
    // The row is not the first of its page, so its offsets are not its columns.
    g.write_ascii(1, 0, b"ab", &link);
    g.write_ascii(1, 3, b"cd", &plain());
    g.write_ascii(1, 6, b"ef", &link);
    assert_eq!(reach(&g, 1, 7), ((1, 6), (1, 7)));
    assert_eq!(reach(&g, 1, 1), ((1, 0), (1, 1)));
}
