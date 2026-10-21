//! What the placements' checker refuses. `Grid::check_invariants` calls it after every test, so a
//! sound table is checked all the time; these make the table unsound on purpose, one way at a time.

use super::{NewPlacement, Placement};
use crate::grid::Grid;
use crate::types::{Extent, ImageHandle, SourceRect};

/// A grid of ten columns and four rows (the serial numbers 0 to 3) with one placement in it.
fn holding(top: i64, col: usize, cols: usize, rows: usize) -> Grid {
    let mut grid = Grid::new(10, 4, 0);
    let spec = NewPlacement {
        image: ImageHandle(1),
        image_id: 0,
        id: 0,
        cols,
        rows,
        source: SourceRect { x: 0, y: 0, width: 8, height: 16 },
        offset_x: 0,
        offset_y: 0,
        extent: Extent::Fill,
        z: 0,
    };
    grid.placements.list.push(Placement { spec, top, col });
    grid
}

#[test]
fn a_placement_inside_the_grid_passes() {
    assert_eq!(holding(1, 2, 3, 2).check_placements(), Ok(()));
}

#[test]
fn a_placement_that_covers_no_cell_is_refused() {
    assert!(holding(1, 2, 0, 2).check_placements().is_err(), "no columns");
    assert!(holding(1, 2, 3, 0).check_placements().is_err(), "no rows");
}

#[test]
fn a_placement_that_starts_past_the_last_column_is_refused() {
    assert!(holding(1, 10, 1, 1).check_placements().is_err());
}

#[test]
fn a_placement_with_no_row_in_the_grid_is_refused() {
    assert!(holding(-5, 0, 1, 2).check_placements().is_err(), "all of it above the first row");
    assert!(holding(4, 0, 1, 2).check_placements().is_err(), "all of it below the last");
}
