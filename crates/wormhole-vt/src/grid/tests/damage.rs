//! What changes the next snapshot has to copy, and what does not: a call that changes nothing
//! damages nothing, and the flags of a row survive the ones that are changed.

use super::BG;
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid, Region};
use crate::types::{Line, LineSize};

/// Take the damage: whether everything changed, and which rows did.
fn damage(grid: &mut Grid) -> (bool, Vec<bool>) {
    let mut rows = vec![false; grid.rows()];
    let all = grid.take_damage(&mut rows);
    (all, rows)
}

#[test]
fn scrolling_or_shifting_lines_by_none_damages_nothing() {
    let mut g = Grid::new(6, 5, DEFAULT_SCROLLBACK_BYTES);
    let region = Region { top: 1, bottom: 3, left: 0, right: 5 };
    // A new grid has everything to be copied; take that first.
    assert_eq!(damage(&mut g), (true, vec![true; 5]));
    g.scroll_down(region, 0, BG);
    g.insert_lines(2, region, 0, BG);
    g.delete_lines(2, region, 0, BG);
    assert_eq!(damage(&mut g), (false, vec![false; 5]));
}

#[test]
fn ending_a_soft_wrap_damages_the_row_and_keeps_its_size() {
    let mut g = Grid::new(6, 3, DEFAULT_SCROLLBACK_BYTES);
    g.set_line_size(1, LineSize::DoubleWidth);
    damage(&mut g);
    g.set_wrapped(1, false);
    assert_eq!(damage(&mut g), (false, vec![false, true, false]));
    assert_eq!(g.line_size(Line(1)), LineSize::DoubleWidth);
}

#[test]
fn giving_a_row_that_is_still_to_be_copied_a_size_leaves_it_to_be_copied() {
    let mut g = Grid::new(6, 3, DEFAULT_SCROLLBACK_BYTES);
    // Every row of a new grid is dirty; a size is a change on top of that, not a copy.
    g.set_line_size(0, LineSize::DoubleWidth);
    assert_eq!(damage(&mut g).1, [true; 3]);
}
