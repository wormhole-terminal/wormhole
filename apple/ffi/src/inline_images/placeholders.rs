//! Kitty's Unicode placeholders (`U=1`): the picture is not placed in the grid, its cells name it.
//! Each cell says which tile of which virtual placement it shows (the engine reads that off the
//! cell: foreground colour = picture id, underline colour = placement id, combining marks = tile row
//! and column). Here consecutive tiles of a row become one placement record, so the view draws them
//! as it draws any placement, stretched over the placement's cells as the GPUI app does
//! (`render/images.rs`, `run_paint`).

use wormhole_term::images::{Placeholder, Tiles, placeholder_at};
use wormhole_term::types::{Extent, ImageHandle, ImagePlacement, Line, SourceRect};
use wormhole_term::{Pty, ScreenSnapshot};

/// Consecutive cells of one row showing consecutive tiles of one placement.
struct Run {
    col: usize,
    cells: usize,
    tile: Placeholder,
}

impl Run {
    fn continues(&self, col: usize, p: &Placeholder) -> bool {
        self.col + self.cells == col
            && self.tile.image_id == p.image_id
            && self.tile.placement_id == p.placement_id
            && self.tile.row == p.row
            && u32::from(self.tile.col) + self.cells as u32 == u32::from(p.col)
    }
}

/// The placements the snapshot's placeholder cells draw, in viewport order. Empty (and nothing
/// allocated) for the screens that have none.
pub(crate) fn from_cells(pty: &Pty, snapshot: &ScreenSnapshot) -> Vec<ImagePlacement> {
    let mut out = Vec::new();
    for (row, line) in snapshot.rows().iter().enumerate() {
        let mut runs: Vec<Run> = Vec::new();
        for (col, cell) in line.cells().iter().enumerate() {
            let Some(tile) = placeholder_at(cell, line.extras().cell(col)) else { continue };
            match runs.last_mut() {
                Some(run) if run.continues(col, &tile) => run.cells += 1,
                _ => runs.push(Run { col, cells: 1, tile }),
            }
        }
        for run in runs {
            let Some(tiles) = pty.virtual_placement(run.tile.image_id, run.tile.placement_id) else {
                continue;
            };
            let Some(image) = pty.images().get(tiles.key) else { continue };
            let line = Line(row as i32 - snapshot.display_offset as i32);
            out.extend(placement(&run, &tiles, (image.width, image.height), line));
        }
    }
    out
}

/// The part of the picture a run's tiles cover, laid over the run's cells.
fn placement(run: &Run, tiles: &Tiles, (width, height): (u32, u32), line: Line) -> Option<ImagePlacement> {
    let (tile_col, tile_row) = (u32::from(run.tile.col), u32::from(run.tile.row));
    if tile_col >= tiles.cols || tile_row >= tiles.rows {
        return None;
    }
    let cells = (run.cells as u32).min(tiles.cols - tile_col);
    // Edges rounded, not sizes, so neighbouring tiles share an edge and leave no seam.
    let edge =
        |tile: u32, tiles: u32, pixels: u32| (u64::from(tile) * u64::from(pixels) / u64::from(tiles)) as u32;
    let (x0, x1) = (edge(tile_col, tiles.cols, width), edge(tile_col + cells, tiles.cols, width));
    let (y0, y1) = (edge(tile_row, tiles.rows, height), edge(tile_row + 1, tiles.rows, height));
    Some(ImagePlacement {
        image: ImageHandle::from_raw(u64::from(tiles.key)),
        image_id: run.tile.image_id,
        placement_id: run.tile.placement_id,
        line,
        column: run.col,
        cols: cells as usize,
        rows: 1,
        source: SourceRect { x: x0, y: y0, width: (x1 - x0).max(1), height: (y1 - y0).max(1) },
        offset_x: 0,
        offset_y: 0,
        extent: Extent::Fill,
        z: 0,
    })
}

#[cfg(test)]
mod tests {
    use wormhole_term::images::key_of;

    use super::*;

    fn run(col: usize, cells: usize, row: u16, tile_col: u16) -> Run {
        Run { col, cells, tile: Placeholder { image_id: 5, placement_id: 0, row, col: tile_col } }
    }

    #[test]
    fn a_run_takes_its_tiles_share_of_the_picture() {
        // A 100 x 60 picture over 4 x 3 tiles: the second row's tiles 1 and 2.
        let tiles = Tiles { key: 9, cols: 4, rows: 3 };
        let p = placement(&run(7, 2, 1, 1), &tiles, (100, 60), Line(2)).expect("inside");
        assert_eq!((p.source.x, p.source.y, p.source.width, p.source.height), (25, 20, 50, 20));
        assert_eq!((p.column, p.cols, p.rows, p.line), (7, 2, 1, Line(2)));
        assert_eq!(key_of(p.image), 9);
        // Cells past the picture's last tile are not drawn; a tile outside it draws nothing.
        let p = placement(&run(0, 5, 0, 2), &tiles, (100, 60), Line(0)).expect("clipped");
        assert_eq!((p.source.x, p.source.width, p.cols), (50, 50, 2));
        assert!(placement(&run(0, 1, 3, 0), &tiles, (100, 60), Line(0)).is_none());
    }

    #[test]
    fn placeholders_of_a_real_terminal_become_placements() {
        use wormhole_term::testing::TestTerm;
        // Picture id 5 as the foreground colour, tiles named by the diacritics for row 0, columns 0 and 1.
        let mut term = TestTerm::new(10, 4, 100);
        term.feed(b"\x1b[38;5;5m\xf4\x8e\xbb\xae\xcc\x85\xcc\x85\xf4\x8e\xbb\xae\xcc\x85\xcc\x8d\x1b[0m");
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot);
        let cells = &snapshot.rows()[0];
        let found: Vec<_> =
            (0..2).filter_map(|c| placeholder_at(&cells.cells()[c], cells.extras().cell(c))).collect();
        assert_eq!(
            found.iter().map(|p| (p.image_id, p.row, p.col)).collect::<Vec<_>>(),
            [(5, 0, 0), (5, 0, 1)]
        );
        assert!(found[0].col == 0 && run(0, 1, 0, 0).continues(1, &found[1]));
    }
}
