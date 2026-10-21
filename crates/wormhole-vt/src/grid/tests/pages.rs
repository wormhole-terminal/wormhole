//! The page list: how many rows a new page holds, how pages are numbered and when the page kept
//! for the next one is reused.

use super::{BG, full};
use crate::grid::page::Page;
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid};

#[test]
fn a_page_holds_the_rows_the_history_and_the_width_call_for() {
    // About eight pages in the limit, at most 16,384 cells each: 10 MiB at 80 columns is 204 rows.
    assert_eq!(Grid::new(80, 24, DEFAULT_SCROLLBACK_BYTES).page_capacity(), 204);
    // Without history a page is exactly the screen.
    assert_eq!(Grid::new(80, 24, 0).page_capacity(), 24);
    // With it, the screen's rows come first where the page's share would be fewer.
    assert_eq!(Grid::new(4096, 10, DEFAULT_SCROLLBACK_BYTES).page_capacity(), 10);
    // A page holds fewer than 65,535 cells, whatever the screen asks for.
    assert_eq!(Grid::new(8192, 10, DEFAULT_SCROLLBACK_BYTES).page_capacity(), 7);
}

#[test]
fn a_page_size_asked_for_is_cut_to_what_a_page_can_hold() {
    assert_eq!(Grid::with_page_rows(80, 24, DEFAULT_SCROLLBACK_BYTES, 5).page_capacity(), 5);
    assert_eq!(Grid::with_page_rows(80, 24, DEFAULT_SCROLLBACK_BYTES, 1000).page_capacity(), 819);
}

#[test]
fn every_page_has_a_number_of_its_own() {
    let mut g = Grid::with_page_rows(10, 2, DEFAULT_SCROLLBACK_BYTES, 1);
    for _ in 0..3 {
        let region = full(&g);
        g.scroll_up(region, 1, BG);
    }
    let numbers: Vec<u64> = g.pages.iter().map(|page| page.uid).collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5]);
}

#[test]
fn a_spare_page_is_reused_only_when_it_has_the_size_of_a_new_one() {
    let mut g = Grid::with_page_rows(10, 2, DEFAULT_SCROLLBACK_BYTES, 1);
    // Wrong in the width and the rows, in the rows only, in the width only.
    for (cols, rows) in [(7, 3), (10, 3), (7, 1)] {
        g.spare = Some(Page::new(99, 0, cols, rows));
        g.add_page();
        let page = g.pages.back().expect("a page was added");
        assert_eq!((page.cols(), page.capacity()), (10, 1), "a spare of {cols} by {rows} is no use");
    }
}
