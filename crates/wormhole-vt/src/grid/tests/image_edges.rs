//! Placements at the edges: of a scrolled part, of the rows that stay after a resize, of a range
//! that was erased and of the view, and what a host asks of them.

use super::images::{grid, new, ok, shown, spots};
use super::{BG, at, full};
use crate::grid::{Grid, NewPlacement, Region};
use crate::types::{Extent, Line, Scroll, SourceRect};

#[test]
fn shifting_cells_deletes_the_pictures_down_to_the_last_cell_it_moves() {
    type Shift = fn(&mut Grid);
    let cases: [(&str, Shift); 2] = [
        ("insert_cells", |g| g.insert_cells(2, 0, 4, 1, BG)),
        ("delete_cells", |g| g.delete_cells(2, 0, 4, 1, BG)),
    ];
    for (name, shift) in cases {
        let mut g = grid(20, 6);
        // One cell, in column 4: the last one the shift reaches.
        g.place_image(Line(2), 4, new(1, 1, 1));
        shift(&mut g);
        assert!(!g.has_placements(), "{name}");
        ok(&g);
    }
}

#[test]
fn a_picture_deleted_by_an_erase_damages_the_screen_and_one_left_alone_does_not() {
    let mut g = grid(20, 6);
    g.place_image(Line(1), 8, new(1, 2, 1));
    let mut rows = [false; 6];
    g.take_damage(&mut rows);
    g.erase(3, 0..4, BG, false);
    assert!(!g.take_damage(&mut rows), "a row changed, and the picture is where it was");
    assert_eq!(rows, [false, false, false, true, false, false]);
    g.erase(1, 8..9, BG, false);
    assert!(g.take_damage(&mut rows), "a picture went");
}

#[test]
fn shrinking_the_height_cuts_a_picture_to_the_rows_that_stay_and_its_source_with_it() {
    let mut g = grid(10, 6);
    // Rows 1 to 4, two cells wide, first stretched over its cells and then at its own size.
    g.place_image(Line(1), 0, new(1, 2, 4));
    let pixels = NewPlacement { extent: Extent::Pixels { width: 16, height: 64 }, ..new(2, 2, 4) };
    g.place_image(Line(1), 4, pixels);
    g.resize(10, 3, at(0, 0), true);
    ok(&g);
    assert_eq!(spots(&g), [(1, 1, 0, 2, 2), (2, 1, 4, 2, 2)]);
    let heights: Vec<u32> = shown(&g).iter().map(|p| p.source.height).collect();
    assert_eq!(heights, [32, 64], "half of the picture stays; one with pixels of its own is not scaled");
}

#[test]
fn a_picture_that_ends_where_the_screen_does_is_not_cut() {
    let mut g = grid(10, 6);
    // A source height of 0 is the picture's whole height: cutting would make it one pixel.
    let whole = NewPlacement { source: SourceRect { x: 0, y: 0, width: 16, height: 0 }, ..new(1, 2, 2) };
    g.place_image(Line(1), 0, whole);
    g.resize(10, 3, at(0, 0), true);
    assert_eq!(spots(&g), [(1, 1, 0, 2, 2)]);
    assert_eq!(shown(&g)[0].source.height, 0);
}

#[test]
fn a_picture_taller_than_the_screen_keeps_its_rows_when_the_width_changes() {
    let mut g = grid(10, 4);
    g.place_image(Line(0), 0, new(1, 2, 10));
    g.resize(8, 4, at(0, 0), true);
    ok(&g);
    assert_eq!(spots(&g), [(1, 0, 0, 2, 10)], "the rows below the screen are still its own");
}

#[test]
fn a_picture_beside_the_margins_stays_and_one_touching_them_goes() {
    let mut g = Grid::new(20, 8, 0);
    let region = Region { top: 0, bottom: 7, left: 5, right: 10 };
    g.place_image(Line(2), 2, new(1, 3, 1)); // up to the margin's first column
    g.place_image(Line(2), 0, new(2, 8, 1)); // over it
    g.place_image(Line(4), 10, new(3, 1, 1)); // in the margin's last column
    g.place_image(Line(6), 12, new(4, 2, 1)); // past it
    g.scroll_up(region, 1, BG);
    assert_eq!(spots(&g), [(1, 2, 2, 3, 1), (4, 6, 12, 2, 1)]);
    ok(&g);
}

#[test]
fn a_margin_that_is_at_one_edge_of_the_screen_is_still_a_margin() {
    // Margins from the left edge, and to the right edge: not the whole width, so pictures
    // beside them are not scrolled with the rows.
    for (left, right, column) in [(0, 10, 12), (5, 19, 0)] {
        let mut g = Grid::new(20, 8, 0);
        g.place_image(Line(3), column, new(1, 2, 1));
        g.scroll_up(Region { top: 0, bottom: 7, left, right }, 1, BG);
        assert_eq!(spots(&g), [(1, 3, column, 2, 1)], "margins {left} to {right}");
    }
}

#[test]
fn a_picture_that_starts_on_the_last_row_of_a_scrolled_part_moves_with_it() {
    let mut g = Grid::new(20, 8, 0);
    let region = Region { top: 2, bottom: 6, left: 0, right: 19 };
    g.place_image(Line(6), 0, new(1, 2, 1));
    g.scroll_up(region, 1, BG);
    assert_eq!(spots(&g), [(1, 5, 0, 2, 1)]);
}

#[test]
fn a_hit_says_which_rows_and_columns_a_picture_covers_from_the_screens_top_row() {
    let mut g = grid(10, 4);
    let region = full(&g);
    // Two lines in the history: the screen's top row is not the first row there ever was.
    g.scroll_up(region, 2, BG);
    g.place_image(Line(1), 2, new(1, 3, 2));
    let mut seen = Vec::new();
    g.delete_placements(
        |hit| {
            let lines: Vec<bool> = (0..5).map(|line| hit.spans_line(line)).collect();
            let columns: Vec<bool> = (1..=5).map(|column| hit.spans_column(column)).collect();
            seen.push((hit.line, lines, columns));
            false
        },
        &mut Vec::new(),
    );
    // Rows 1 and 2, columns 2 to 4: the cells next to them on every side are not covered.
    let lines = vec![false, true, true, false, false];
    let columns = vec![false, true, true, true, false];
    assert_eq!(seen, [(1, lines, columns)]);
}

#[test]
fn a_picture_that_ends_just_above_the_screen_is_not_on_it() {
    let mut g = grid(10, 4);
    g.place_image(Line(0), 0, new(1, 2, 2));
    let region = full(&g);
    g.scroll_up(region, 2, BG);
    // One row of this one is in the history and one is on the screen.
    g.place_image(Line(-1), 4, new(2, 2, 2));
    let mut on_screen = Vec::new();
    g.delete_placements(
        |hit| {
            on_screen.push(hit.on_screen());
            false
        },
        &mut Vec::new(),
    );
    assert_eq!(on_screen, [false, true]);
}

#[test]
fn a_picture_just_below_the_scrolled_back_view_is_not_shown() {
    let mut g = grid(10, 4);
    let region = full(&g);
    g.scroll_up(region, 3, BG);
    g.place_image(Line(1), 0, new(1, 2, 1));
    g.scroll_view(Scroll::Delta(3));
    assert_eq!(g.display_offset(), 3);
    assert!(spots(&g).is_empty(), "the view ends on line 0, and the picture is on line 1");
    g.scroll_view(Scroll::Delta(-1));
    assert_eq!(spots(&g), [(1, 1, 0, 2, 1)]);
}

#[test]
fn a_pixel_sized_picture_that_the_edge_does_not_cut_keeps_its_size() {
    let mut g = grid(10, 4);
    let mut p = new(1, 2, 1);
    // Two cells hold 16 pixels, and the picture is 30 wide: it overflows its cells, which is its own
    // business, and nothing cuts it.
    p.source = SourceRect { x: 0, y: 0, width: 100, height: 20 };
    p.extent = Extent::Pixels { width: 30, height: 16 };
    g.place_image(Line(0), 0, p);
    let placements = shown(&g);
    let placement = &placements[0];
    assert_eq!(placement.cols, 2);
    assert_eq!(placement.extent, Extent::Pixels { width: 30, height: 16 });
    assert_eq!(placement.source.width, 100);
}
