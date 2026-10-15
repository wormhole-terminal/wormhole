//! The grid allocates nothing in the steady state: writing text, scrolling with a full history
//! (the oldest page is reused for the newest), erasing, and copying the screen into a buffer that
//! has been used once.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.

use wormhole_vt::grid::{Grid, Pen, Region, Style, ViewRow};
use wormhole_vt::types::{Color, NamedColor};

#[path = "common/mod.rs"]
mod common;
use common::alloc::{Counting, allocations};

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn pen(n: u8) -> Pen {
    Pen { style: Style { fg: Color::Indexed(n), ..Style::DEFAULT }, ..Pen::default() }
}

/// Print `lines` lines of colored text at the bottom of the screen, scrolling each up.
fn output(grid: &mut Grid, lines: usize) {
    let region = Region::full(grid.cols(), grid.rows());
    let bg = Color::Named(NamedColor::Background);
    let text = b"the quick brown fox jumps over the lazy dog, again and again and again.";
    for i in 0..lines {
        let y = grid.rows() - 1;
        grid.write_ascii(y, 0, &text[..40], &pen((i % 7) as u8));
        grid.write_ascii(y, 40, &text[40..], &pen(((i + 3) % 5) as u8));
        grid.scroll_up(region, 1, bg);
    }
}

#[test]
fn output_with_a_full_history_allocates_nothing() {
    let mut grid = Grid::new(80, 24, 256 * 1024);
    // Fill the history past its cap twice over, so that pages are being dropped and reused.
    output(&mut grid, 2_000);
    let before = allocations();
    output(&mut grid, 50_000);
    assert_eq!(allocations() - before, 0, "50,000 lines of colored output");
    assert!(grid.check_invariants().is_ok());
}

#[test]
fn erasing_and_editing_allocate_nothing() {
    let mut grid = Grid::new(80, 24, 0);
    let bg = Color::Named(NamedColor::Blue);
    output(&mut grid, 100);
    let region = Region { top: 2, bottom: 20, left: 0, right: 79 };
    let mut edit = |rounds: usize| {
        for i in 0..rounds {
            grid.erase(i % 24, 5..60, bg, false);
            grid.insert_cells(i % 24, 3, 70, 4, bg);
            grid.delete_cells(i % 24, 3, 70, 4, bg);
            grid.scroll_up(region, 1, bg);
            grid.scroll_down(region, 2, bg);
            grid.insert_lines(5, region, 1, bg);
            grid.delete_lines(5, region, 1, bg);
        }
    };
    // The first time a page sees a style it makes room for it; after that it only counts.
    edit(100);
    let before = allocations();
    edit(10_000);
    assert_eq!(allocations() - before, 0);
}

#[test]
fn copying_the_screen_into_a_used_buffer_allocates_nothing() {
    let mut grid = Grid::new(120, 40, 1 << 20);
    output(&mut grid, 8_000);
    let mut rows: Vec<ViewRow> = Vec::new();
    grid.copy_viewport(&mut rows, true);
    let before = allocations();
    for _ in 0..1_000 {
        grid.copy_viewport(&mut rows, true);
        output(&mut grid, 1);
        grid.copy_viewport(&mut rows, false);
    }
    assert_eq!(allocations() - before, 0);
}

/// Characters whose cells keep more than the first code point in a side table: a base with one or
/// more combining marks, an emoji with a skin tone, a flag, a family joined with ZWJs.
const CLUSTERS: &[(char, &[char], u8)] = &[
    ('e', &['\u{301}'], 1),
    ('o', &['\u{302}', '\u{323}'], 1),
    ('\u{1F44D}', &['\u{1F3FD}'], 2),
    ('\u{1F1E9}', &['\u{1F1EA}'], 2),
    ('\u{1F468}', &['\u{200D}', '\u{1F469}', '\u{200D}', '\u{1F467}', '\u{200D}', '\u{1F466}'], 2),
    ('\u{2764}', &['\u{FE0F}'], 1),
];

/// Print `lines` lines of clusters at the bottom of the screen, scrolling each up. Some marks are
/// written with the cluster, some added after it, as the terminal does when they arrive later.
fn cluster_output(grid: &mut Grid, lines: usize) {
    let region = Region::full(grid.cols(), grid.rows());
    let bg = Color::Named(NamedColor::Background);
    for i in 0..lines {
        let y = grid.rows() - 1;
        let mut x = 0;
        for k in 0..20 {
            let (c, marks, width) = CLUSTERS[(i + k) % CLUSTERS.len()];
            let pen = pen(((i + k) % 5) as u8);
            if k % 4 == 3 {
                // The marks one by one.
                grid.write_cluster(y, x, c, &[], width, &pen);
                for &mark in marks {
                    grid.push_mark(y, x, mark);
                }
            } else {
                grid.write_cluster(y, x, c, marks, width, &pen);
            }
            x += width as usize;
        }
        grid.write_ascii(y, x, b" plain text between the clusters", &pen(1));
        grid.scroll_up(region, 1, bg);
    }
}

#[test]
fn clusters_with_marks_allocate_nothing_in_the_steady_state() {
    let mut grid = Grid::new(80, 24, 256 * 1024);
    // Fill the history past its cap many times over, so that pages are being dropped and reused and
    // every page's buffers for marks have reached the size the lines need.
    cluster_output(&mut grid, 30_000);
    let before = allocations();
    cluster_output(&mut grid, 30_000);
    assert_eq!(allocations() - before, 0, "30,000 lines of emoji, combining marks and ZWJ sequences");
    assert!(grid.check_invariants().is_ok());
}

#[test]
fn editing_clusters_allocates_nothing() {
    let mut grid = Grid::new(80, 24, 0);
    let bg = Color::Named(NamedColor::Blue);
    for _ in 0..30 {
        cluster_output(&mut grid, 1);
    }
    let region = Region { top: 2, bottom: 20, left: 0, right: 79 };
    let mut edit = |rounds: usize| {
        for i in 0..rounds {
            // Insert and delete shift marked cells along the row, and copy_within moves rows with them.
            grid.insert_cells(i % 24, 3, 70, 4, bg);
            grid.delete_cells(i % 24, 3, 70, 4, bg);
            grid.scroll_up(region, 1, bg);
            grid.scroll_down(region, 2, bg);
            grid.insert_lines(5, region, 1, bg);
            grid.delete_lines(5, region, 1, bg);
            grid.erase(i % 24, 10..40, bg, false);
            cluster_output(&mut grid, 1);
        }
    };
    edit(3_000);
    let before = allocations();
    edit(5_000);
    assert_eq!(allocations() - before, 0);
    assert!(grid.check_invariants().is_ok());
}

#[test]
fn copying_clusters_across_pages_and_reflowing_them_stays_valid() {
    // Not an allocation count: reflow and copies between pages build rows cell by cell, and must
    // keep every cluster's marks.
    let mut grid = Grid::with_page_rows(40, 12, 1 << 20, 5);
    cluster_output(&mut grid, 300);
    let marks = |grid: &Grid| -> usize {
        (-(grid.history_len() as i32)..grid.rows() as i32)
            .map(|y| {
                grid.row_text(wormhole_vt::types::Line(y), true).chars().filter(|c| *c == '\u{301}').count()
            })
            .sum()
    };
    let before = marks(&grid);
    assert!(before > 0);
    let region = Region { top: 1, bottom: 10, left: 0, right: 39 };
    grid.scroll_up(region, 3, Color::Named(NamedColor::Background));
    assert!(grid.check_invariants().is_ok());
    let at = wormhole_vt::types::Point::new(wormhole_vt::types::Line(11), wormhole_vt::types::Column(0));
    grid.resize(60, 12, at, true);
    assert!(grid.check_invariants().is_ok());
}
