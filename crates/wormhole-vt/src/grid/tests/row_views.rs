//! A row read in place (`RowView`): what the repaint writer and the state dump read, and what a
//! rebuild of the rows carries from one row to the next.

use super::{at, plain};
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid, Pen};
use crate::types::{Hyperlink, Line, LineSize};

fn grid(cols: usize, rows: usize) -> Grid {
    Grid::new(cols, rows, DEFAULT_SCROLLBACK_BYTES)
}

#[test]
fn the_links_of_a_view_are_columns_of_its_own_row() {
    let mut g = grid(10, 3);
    let link = Hyperlink { id: "l".into(), uri: "https://example.com/".into() };
    let under = Pen { link: Some(link.clone()), ..plain() };
    g.write_ascii(0, 6, b"zz", &under);
    g.write_ascii(1, 2, b"abc", &under);
    let links = |line: i32| {
        let view = g.row_view(Line(line)).expect("a row of the screen");
        view.links().map(|(from, to, link)| (from, to, link.clone())).collect::<Vec<_>>()
    };
    // The second row is not the first of its page: its columns are not its offsets.
    assert_eq!(links(1), [(2, 5, link.clone())]);
    assert_eq!(links(0), [(6, 8, link)]);
    assert_eq!(links(2), []);
}

#[test]
fn a_view_knows_the_size_wrap_and_block_of_its_row() {
    let mut g = grid(10, 3);
    g.set_line_size(2, LineSize::DoubleHeightTop);
    g.set_wrapped(1, true);
    g.set_row_block(1, 7);
    let view = |line: i32| g.row_view(Line(line)).expect("a row of the screen");
    assert_eq!(view(2).line_size(), LineSize::DoubleHeightTop);
    assert_eq!(view(0).line_size(), LineSize::Single);
    assert_eq!((view(1).wrapped(), view(1).block()), (true, 7));
    assert_eq!((view(0).wrapped(), view(0).block()), (false, 0));
    assert!(g.row_view(Line(3)).is_none());
}

#[test]
fn a_block_without_content_is_no_mark() {
    let mut g = grid(10, 2);
    // The terminal sets the block of a prompt row that has no text yet: nothing says which part of
    // the block is there, so the row has no mark to read.
    g.set_row_block(1, 7);
    assert_eq!(g.row_mark(Line(1)), None);
    assert_eq!(g.row_view(Line(1)).map(|view| view.block()), Some(7));
}

#[test]
fn a_rows_block_survives_a_reflow_that_copies_the_row_whole() {
    let mut g = grid(10, 3);
    g.write_ascii(0, 0, b"ab", &plain());
    g.set_row_block(1, 7);
    g.resize(12, 3, at(0, 0), true);
    let block = |line: i32| g.row_view(Line(line)).map(|view| view.block());
    assert_eq!((block(0), block(1), block(2)), (Some(0), Some(7), Some(0)));
}
