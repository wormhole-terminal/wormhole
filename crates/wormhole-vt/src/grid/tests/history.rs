//! The history's cap and what it counts: the cost of a row, the bytes held, and where exactly the
//! cap drops a page. Pages of one row make the arithmetic exact: every row scrolled off is a page.

use super::{BG, full, plain};
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid, Pen};
use crate::types::Hyperlink;

fn ok(grid: &Grid) {
    if let Err(e) = grid.check_invariants() {
        panic!("{e}");
    }
}

/// A grid of sixteen columns and two rows whose pages hold one row each, with a cap high enough
/// that nothing is dropped until a test lowers it. (A row costs more than the bookkeeping of an
/// empty page, so a cap of a few rows is not over it by that alone.)
fn one_row_pages() -> Grid {
    Grid::with_page_rows(16, 2, DEFAULT_SCROLLBACK_BYTES, 1)
}

/// Scroll the top row into the history `n` times.
fn scroll_off(g: &mut Grid, n: usize) {
    let region = full(g);
    for _ in 0..n {
        g.scroll_up(region, 1, BG);
    }
}

/// A pen whose cells are under a long link, which is most of what a page of history weighs.
fn linked() -> Pen {
    let link = Hyperlink { id: "l".into(), uri: format!("https://example.com/{}", "x".repeat(400)).into() };
    Pen { link: Some(link), ..plain() }
}

#[test]
fn a_row_of_history_costs_more_than_its_cells() {
    let g = Grid::new(80, 24, DEFAULT_SCROLLBACK_BYTES);
    // The cell is eight bytes; the row's own record comes on top.
    assert!(g.row_cost() > 80 * 8, "{}", g.row_cost());
    assert!(g.row_cost() < 80 * 8 + 64, "but not by much: {}", g.row_cost());
}

#[test]
fn the_memory_held_covers_the_screen_and_grows_with_the_history() {
    let mut g = Grid::new(80, 24, DEFAULT_SCROLLBACK_BYTES);
    let screen = g.memory_bytes();
    assert!(screen >= 80 * 24 * 8, "the screen's cells alone are {} bytes", 80 * 24 * 8);
    scroll_off(&mut g, 2000);
    assert!(g.memory_bytes() > screen + 1000 * 80 * 8, "{} after 2,000 lines", g.memory_bytes());
}

#[test]
fn a_cap_of_exactly_the_history_keeps_all_of_it() {
    let mut g = one_row_pages();
    scroll_off(&mut g, 3);
    assert_eq!(g.history_len(), 3);
    // The page being filled turns into history row by row, so the cap leaves room for it.
    let room = g.pages.back().map_or(0, |page| page.capacity());
    g.set_scrollback_limit((3 + room) * g.row_cost());
    assert_eq!(g.history_len(), 3, "three rows fit exactly: nothing goes");
    g.set_scrollback_limit((3 + room) * g.row_cost() - 1);
    assert_eq!(g.history_len(), 2, "one byte less and the oldest page goes");
    ok(&g);
}

#[test]
fn the_page_just_above_the_screen_goes_when_the_cap_cannot_hold_it() {
    let mut g = one_row_pages();
    scroll_off(&mut g, 1);
    assert_eq!(g.history_len(), 1);
    // The only history page ends exactly where the screen starts.
    g.set_scrollback_limit(g.row_cost());
    assert_eq!(g.history_len(), 0);
    assert_eq!(g.rows(), 2, "the screen is whole");
    ok(&g);
}

#[test]
fn history_whose_marks_and_links_weigh_exactly_the_cap_stays() {
    let mut g = one_row_pages();
    g.write_ascii(0, 0, b"x", &linked());
    scroll_off(&mut g, 1);
    let weight = g.pages[0].extra_bytes();
    assert!(weight / g.row_cost() > 1, "the link is heavy enough to leave room for the row: {weight}");
    g.set_scrollback_limit(weight);
    assert_eq!(g.history_len(), 1, "as heavy as the cap is not over it");
    g.set_scrollback_limit(weight - 1);
    assert_eq!(g.history_len(), 0, "one byte over");
    ok(&g);
}

#[test]
fn a_history_over_the_cap_for_its_links_drops_pages_until_it_is_under() {
    let mut g = one_row_pages();
    for _ in 0..3 {
        g.write_ascii(0, 0, b"x", &linked());
        scroll_off(&mut g, 1);
    }
    assert_eq!(g.history_len(), 3);
    let weight = g.pages[0].extra_bytes();
    // Room for the rows themselves, but only one and a half pages' worth of links: two pages go.
    g.set_scrollback_limit(weight + weight / 2);
    assert_eq!(g.history_len(), 1);
    ok(&g);
}
