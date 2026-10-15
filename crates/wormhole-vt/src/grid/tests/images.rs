//! Picture placements in the grid: moving with the text, erasing, reflow and the history.

use super::{BG, Sim, at, full, plain};
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid, NewPlacement, Region};
use crate::types::{Extent, ImageHandle, ImagePlacement, Line, SourceRect};

fn ok(grid: &Grid) {
    if let Err(e) = grid.check_invariants() {
        panic!("{e}");
    }
}

fn handle(n: u64) -> ImageHandle {
    ImageHandle(n)
}

fn new(image: u64, cols: usize, rows: usize) -> NewPlacement {
    NewPlacement {
        image: handle(image),
        image_id: 0,
        id: 0,
        cols,
        rows,
        source: SourceRect { x: 0, y: 0, width: cols as u32 * 8, height: rows as u32 * 16 },
        offset_x: 0,
        offset_y: 0,
        extent: Extent::Fill,
        z: 0,
    }
}

fn shown(grid: &Grid) -> Vec<ImagePlacement> {
    let mut out = Vec::new();
    grid.copy_placements(&mut out, (8, 16));
    out
}

/// (image, line, column, cols, rows) of what is shown.
fn spots(grid: &Grid) -> Vec<(u64, i32, usize, usize, usize)> {
    shown(grid).iter().map(|p| (p.image.get(), p.line.0, p.column, p.cols, p.rows)).collect()
}

fn grid(cols: usize, rows: usize) -> Grid {
    Grid::new(cols, rows, DEFAULT_SCROLLBACK_BYTES)
}

#[test]
fn a_placement_is_where_it_was_put() {
    let mut g = grid(20, 6);
    assert!(g.place_image(Line(1), 3, new(7, 4, 2)));
    assert_eq!(spots(&g), [(7, 1, 3, 4, 2)]);
    assert!(g.has_placements());
    // Outside the grid: refused.
    assert!(!g.place_image(Line(0), 20, new(8, 1, 1)));
    assert!(!g.place_image(Line(6), 0, new(8, 1, 1)));
    assert_eq!(g.placement_count(), 1);
    ok(&g);
}

#[test]
fn a_placement_scrolls_into_the_history_with_its_text() {
    let mut g = grid(20, 4);
    g.write_ascii(1, 0, b"beside", &plain());
    g.place_image(Line(1), 8, new(1, 3, 2));
    let region = full(&g);
    g.scroll_up(region, 1, BG);
    assert_eq!(spots(&g), [(1, 0, 8, 3, 2)]);
    g.scroll_up(region, 1, BG);
    assert_eq!(spots(&g), [(1, -1, 8, 3, 2)], "its first row is in the history, its second still shows");
    g.scroll_up(region, 1, BG);
    assert!(spots(&g).is_empty(), "both rows are in the history, out of the live view");
    g.scroll_view(crate::types::Scroll::Top);
    assert_eq!(spots(&g), [(1, 1 - 4, 8, 3, 2)].map(|(i, _, c, w, h)| (i, -2, c, w, h)));
    assert_eq!(g.placement_count(), 1);
    ok(&g);
}

#[test]
fn a_placement_goes_when_all_of_its_rows_leave_the_history() {
    let mut g = Grid::new(10, 3, 0);
    g.place_image(Line(0), 0, new(1, 2, 2));
    let region = full(&g);
    // Without history, scrolling shifts the screen: the picture rides up and half of it is lost.
    g.scroll_up(region, 1, BG);
    assert_eq!(spots(&g), [(1, -1, 0, 2, 2)]);
    g.scroll_up(region, 1, BG);
    assert!(!g.has_placements(), "its last row left too");
    ok(&g);

    let mut g = Grid::new(10, 3, 4 * 1024);
    g.place_image(Line(0), 0, new(1, 2, 1));
    for _ in 0..1000 {
        let region = full(&g);
        g.scroll_up(region, 1, BG);
    }
    assert!(!g.has_placements(), "dropped with its history page");
    ok(&g);
}

#[test]
fn erasing_a_cell_under_a_placement_deletes_it_but_other_cells_do_not() {
    let mut g = grid(20, 6);
    g.place_image(Line(1), 4, new(1, 4, 3));
    g.erase(1, 0..4, BG, false);
    g.erase(1, 8..20, BG, false);
    g.erase(0, 0..20, BG, false);
    g.erase(4, 0..20, BG, false);
    assert_eq!(g.placement_count(), 1, "beside and around it");
    g.erase(2, 7..8, BG, false);
    assert!(!g.has_placements(), "one cell under it");
    ok(&g);
}

#[test]
fn every_erase_that_reaches_a_placement_deletes_it() {
    type Erase = fn(&mut Grid);
    let cases: [(&str, Erase); 6] = [
        ("erase_rows", |g| g.erase_rows(2..3, BG, false)),
        ("insert_cells", |g| g.insert_cells(2, 0, 19, 1, BG)),
        ("delete_cells", |g| g.delete_cells(2, 6, 19, 1, BG)),
        ("whole screen", |g| g.erase_rows(0..6, BG, false)),
        ("erase_rows over a row of it", |g| g.erase_rows(3..6, BG, false)),
        ("erase of the row's end", |g| g.erase(3, 5..20, BG, false)),
    ];
    for (name, erase) in cases {
        let mut g = grid(20, 6);
        g.place_image(Line(2), 4, new(1, 4, 2));
        erase(&mut g);
        assert!(!g.has_placements(), "{name}");
        ok(&g);
    }
    // Cells shifted to the left of it, or above it, are not under it.
    let mut g = grid(20, 6);
    g.place_image(Line(2), 4, new(1, 4, 2));
    g.insert_cells(2, 0, 3, 1, BG);
    g.delete_cells(1, 0, 19, 3, BG);
    assert_eq!(g.placement_count(), 1);
}

#[test]
fn selective_erase_and_printing_leave_a_placement_alone() {
    let mut g = grid(20, 6);
    g.place_image(Line(1), 4, new(1, 4, 3));
    g.erase_rows(0..6, BG, true);
    g.erase(2, 0..20, BG, true);
    g.write_ascii(2, 3, b"text over it", &plain());
    assert_eq!(g.placement_count(), 1);
    assert_eq!(g.row_text(Line(2), true), "   text over it");
    ok(&g);
}

#[test]
fn a_part_of_the_screen_scrolling_moves_the_placements_inside_it() {
    let mut g = Grid::new(20, 8, 0);
    let region = Region { top: 2, bottom: 6, left: 0, right: 19 };
    g.place_image(Line(4), 0, new(1, 2, 2));
    g.scroll_up(region, 1, BG);
    assert_eq!(spots(&g), [(1, 3, 0, 2, 2)]);
    g.scroll_down(region, 2, BG);
    assert_eq!(spots(&g), [(1, 5, 0, 2, 2)]);
    // Pushed out of the region at the bottom.
    g.scroll_down(region, 1, BG);
    assert!(!g.has_placements());
    ok(&g);
}

#[test]
fn a_placement_across_the_edge_of_a_scrolled_part_is_deleted() {
    for (line, rows) in [(1, 3), (5, 3), (0, 8)] {
        let mut g = Grid::new(20, 8, 0);
        let region = Region { top: 2, bottom: 6, left: 0, right: 19 };
        g.place_image(Line(line), 0, new(1, 2, rows));
        g.scroll_up(region, 1, BG);
        assert!(!g.has_placements(), "line {line}, {rows} rows");
        ok(&g);
    }
    // One entirely outside is not touched.
    let mut g = Grid::new(20, 8, 0);
    let region = Region { top: 2, bottom: 6, left: 0, right: 19 };
    g.place_image(Line(0), 0, new(1, 2, 2));
    g.place_image(Line(7), 0, new(2, 2, 1));
    g.scroll_up(region, 3, BG);
    assert_eq!(spots(&g), [(1, 0, 0, 2, 2), (2, 7, 0, 2, 1)]);
}

#[test]
fn inserting_and_deleting_lines_move_the_placements_below() {
    let mut g = Grid::new(20, 8, 0);
    let region = full(&g);
    g.place_image(Line(5), 0, new(1, 2, 2));
    g.insert_lines(2, region, 1, BG);
    assert_eq!(spots(&g), [(1, 6, 0, 2, 2)]);
    g.delete_lines(1, region, 2, BG);
    assert_eq!(spots(&g), [(1, 4, 0, 2, 2)]);
    // A deleted line inside the picture's own rows cuts it.
    g.delete_lines(4, region, 1, BG);
    assert!(!g.has_placements());
    ok(&g);
}

#[test]
fn margins_at_the_sides_delete_the_placements_they_shift() {
    let mut g = Grid::new(20, 8, 0);
    let region = Region { top: 0, bottom: 7, left: 5, right: 10 };
    g.place_image(Line(2), 0, new(1, 2, 2));
    g.place_image(Line(2), 6, new(2, 2, 2));
    g.scroll_up(region, 1, BG);
    assert_eq!(spots(&g), [(1, 2, 0, 2, 2)], "the one beside the margins stays, the one inside goes");
}

#[test]
fn rows_under_a_scroll_region_ride_down_with_the_text() {
    let mut g = grid(20, 6);
    let region = Region { top: 0, bottom: 3, left: 0, right: 19 };
    g.place_image(Line(1), 0, new(1, 2, 2));
    g.place_image(Line(4), 0, new(2, 2, 2));
    g.scroll_up(region, 1, BG);
    // The first scrolled into the history with its text, the second is still on the last rows.
    assert_eq!(spots(&g), [(1, 0, 0, 2, 2), (2, 4, 0, 2, 2)]);
    ok(&g);
    let mut g = grid(20, 6);
    g.place_image(Line(3), 0, new(1, 2, 2));
    g.scroll_up(region, 1, BG);
    assert!(!g.has_placements(), "across the region's bottom edge");
}

#[test]
fn the_screen_cleared_into_the_history_takes_the_pictures_with_it() {
    let mut g = grid(20, 6);
    g.place_image(Line(1), 0, new(1, 2, 3));
    assert_eq!(g.used_rows(), 4);
    g.scroll_screen_into_history(BG);
    assert!(spots(&g).is_empty(), "above the live screen now");
    g.scroll_view(crate::types::Scroll::Top);
    assert_eq!(spots(&g).len(), 1);
    g.erase_rows(0..6, BG, false);
    assert_eq!(g.placement_count(), 1, "the history is not erased");
    g.clear_scrollback();
    assert!(!g.has_placements());
    ok(&g);
}

#[test]
fn a_reflow_carries_the_top_row_with_its_text() {
    let mut sim = Sim::new(20, 6, DEFAULT_SCROLLBACK_BYTES);
    sim.print("a long line of text that wraps over rows\n");
    sim.print("above\n");
    let top = sim.y as i32;
    sim.grid.place_image(Line(top), 2, new(1, 3, 2));
    sim.print("\n\n");
    let cursor = at(sim.y as i32, sim.x);
    let before = spots(&sim.grid);
    assert_eq!(before, [(1, top, 2, 3, 2)]);
    let moved = sim.grid.resize(40, 6, cursor, true);
    ok(&sim.grid);
    // The wrapped line joined: everything below moved up by one row, the picture with "above".
    assert_eq!(spots(&sim.grid), [(1, top - 1, 2, 3, 2)]);
    sim.grid.resize(10, 6, moved, true);
    ok(&sim.grid);
    let after = spots(&sim.grid);
    assert_eq!(after.len(), 1);
    assert_eq!((after[0].2, after[0].3, after[0].4), (2, 3, 2), "its size in cells is kept");
}

#[test]
fn a_placement_that_no_longer_fits_across_is_cut_or_dropped() {
    let mut g = grid(20, 6);
    g.place_image(Line(1), 4, new(1, 8, 2));
    g.place_image(Line(3), 15, new(2, 3, 1));
    let c = g.resize(10, 6, at(5, 0), true);
    assert_eq!(spots(&g), [(1, 1, 4, 6, 2)], "cut at the new edge; the other started outside");
    let shown = shown(&g);
    assert_eq!(shown[0].source.width, 48, "the picture is cut in step: 6 of 8 cells of 64 pixels");
    ok(&g);
    g.resize(20, 6, c, true);
    assert_eq!(spots(&g), [(1, 1, 4, 8, 2)], "widening shows it whole again");
    ok(&g);
}

#[test]
fn a_cut_pixel_placement_keeps_its_scale() {
    let mut g = grid(10, 4);
    let mut p = new(1, 4, 1);
    p.source = SourceRect { x: 0, y: 0, width: 100, height: 20 };
    p.extent = Extent::Pixels { width: 30, height: 16 };
    g.place_image(Line(0), 8, p);
    let shown = shown(&g);
    assert_eq!((shown[0].cols, shown[0].extent), (2, Extent::Pixels { width: 16, height: 16 }));
    assert_eq!(shown[0].source.width, 100 * 16 / 30);
}

#[test]
fn shrinking_the_height_cuts_the_rows_that_went() {
    let mut g = grid(10, 8);
    g.place_image(Line(5), 0, new(1, 2, 3));
    g.place_image(Line(7), 0, new(2, 2, 1));
    g.resize(10, 6, at(0, 0), true);
    ok(&g);
    let kept = spots(&g);
    assert!(kept.iter().all(|s| s.1 + s.4 as i32 <= 6 || s.1 < 0), "{kept:?}");
}

#[test]
fn deleting_by_a_test_over_the_hits_reports_the_pictures() {
    let mut g = grid(20, 6);
    let mut a = new(1, 2, 2);
    a.z = 5;
    g.place_image(Line(0), 0, a);
    g.place_image(Line(3), 5, new(2, 2, 2));
    let mut removed = Vec::new();
    let n = g.delete_placements(|hit| hit.z == 5 || hit.covers(3, 5), &mut removed);
    assert_eq!(n, 2);
    assert_eq!(removed, [handle(1), handle(2)]);
    assert!(!g.has_placements());

    g.place_image(Line(0), 0, new(3, 2, 4));
    let hits: Vec<_> = {
        let mut seen = Vec::new();
        g.delete_placements(
            |hit| {
                seen.push((hit.line, hit.rows, hit.column, hit.cols, hit.on_screen()));
                false
            },
            &mut removed,
        );
        seen
    };
    assert_eq!(hits, [(0, 4, 0, 2, true)]);
}

#[test]
fn a_placement_with_an_id_replaces_the_one_with_that_id() {
    let mut g = grid(20, 6);
    let with_id = |image, id| NewPlacement { id, ..new(image, 2, 1) };
    g.place_image(Line(0), 0, with_id(1, 4));
    g.place_image(Line(1), 0, with_id(1, 4));
    g.place_image(Line(2), 0, with_id(1, 5));
    g.place_image(Line(3), 0, with_id(2, 4));
    g.place_image(Line(4), 0, with_id(1, 0));
    g.place_image(Line(5), 0, with_id(1, 0));
    assert_eq!(spots(&g).len(), 5, "same image and id: replaced; no id: always new");
    assert!(g.is_placed(handle(2)));
    assert!(!g.is_placed(handle(9)));
}

#[test]
fn the_oldest_placements_go_past_the_cap() {
    let mut g = grid(20, 6);
    for i in 0..crate::grid::MAX_PLACEMENTS as u64 + 10 {
        g.place_image(Line(0), 0, new(i + 1, 1, 1));
    }
    assert_eq!(g.placement_count(), crate::grid::MAX_PLACEMENTS);
    assert!(!g.is_placed(handle(1)));
    assert!(g.is_placed(handle(crate::grid::MAX_PLACEMENTS as u64 + 10)));
}

#[test]
fn drawing_order_is_by_z_then_age_and_the_view_picks_what_it_shows() {
    let mut g = grid(20, 3);
    let z = |image, z| NewPlacement { z, ..new(image, 1, 1) };
    g.place_image(Line(0), 0, z(1, 0));
    g.place_image(Line(0), 1, z(2, -3));
    g.place_image(Line(0), 2, z(3, 0));
    assert_eq!(shown(&g).iter().map(|p| p.image.get()).collect::<Vec<_>>(), [2, 1, 3]);
    // Scroll the picture off the live screen and view the history.
    let region = full(&g);
    g.scroll_up(region, 3, BG);
    assert!(shown(&g).is_empty(), "not in the view");
    g.scroll_view(crate::types::Scroll::Top);
    assert_eq!(shown(&g).len(), 3);
}

#[test]
fn a_reset_deletes_every_placement() {
    let mut g = grid(20, 6);
    g.place_image(Line(0), 0, new(1, 1, 1));
    g.reset();
    assert!(!g.has_placements());
}
