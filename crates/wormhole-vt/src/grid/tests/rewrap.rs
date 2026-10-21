//! The bookkeeping of a width change: where the rows that hold text end, the last row of the
//! history when it says it is wrapped, the looks of a line that is rewrapped, and the numbers of
//! the pages it makes. (What the text does is in `reflow`.)

use super::{BG, Sim, at, colored, full, plain, screen};
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid};
use crate::types::{Color, LineSize, NamedColor};

fn ok(grid: &Grid) {
    if let Err(e) = grid.check_invariants() {
        panic!("{e}");
    }
}

fn grid(cols: usize, rows: usize) -> Grid {
    Grid::new(cols, rows, DEFAULT_SCROLLBACK_BYTES)
}

/// A grid whose rows are `a`, `b`, `c` ... one letter each, in the first column.
fn lettered(cols: usize, rows: usize) -> Grid {
    let mut g = grid(cols, rows);
    for y in 0..rows {
        g.write_ascii(y, 0, &[b'a' + y as u8], &plain());
    }
    g
}

#[test]
fn a_crop_keeps_the_rows_down_to_the_last_with_text_even_below_the_cursor() {
    // The alternate screen is cropped, not reflowed. The cursor is on the third row and the text
    // goes on below it: of four rows, the last four of six are kept.
    let mut g = lettered(10, 6);
    let cursor = g.resize(8, 4, at(2, 0), false);
    assert_eq!(screen(&g), ["c", "d", "e", "f"]);
    assert_eq!(g.history_len(), 2);
    assert_eq!(cursor, at(0, 0));
    ok(&g);
}

#[test]
fn a_reflow_keeps_the_rows_down_to_the_last_with_text_even_below_the_cursor() {
    let mut g = lettered(10, 6);
    let cursor = g.resize(8, 4, at(2, 0), true);
    assert_eq!(screen(&g), ["c", "d", "e", "f"]);
    assert_eq!(g.history_len(), 2);
    assert_eq!(cursor, at(0, 0));
    ok(&g);
}

#[test]
fn a_double_width_row_that_ends_the_text_is_kept_below_the_cursor() {
    let mut g = lettered(8, 4);
    g.set_line_size(3, LineSize::DoubleWidth);
    let cursor = g.resize(6, 2, at(2, 0), true);
    assert_eq!(screen(&g), ["c", "d"]);
    assert_eq!(cursor, at(0, 0));
    ok(&g);
}

#[test]
fn a_line_over_several_rows_that_ends_the_text_is_kept_below_the_cursor() {
    let mut g = grid(6, 4);
    g.write_ascii(0, 0, b"a", &plain());
    g.write_ascii(1, 0, b"b", &plain());
    g.write_ascii(2, 0, b"cdefgh", &plain());
    g.set_wrapped(2, true);
    g.write_ascii(3, 0, b"ij", &plain());
    let cursor = g.resize(12, 2, at(1, 0), true);
    assert_eq!(screen(&g), ["b", "cdefghij"]);
    assert_eq!(cursor, at(0, 0));
    ok(&g);
}

#[test]
fn blank_rows_that_say_they_are_wrapped_do_not_count_as_text() {
    // A line whose text was erased can leave its rows marked as wrapped.
    let mut g = grid(6, 4);
    g.write_ascii(0, 0, b"a", &plain());
    g.write_ascii(1, 0, b"b", &plain());
    g.set_wrapped(2, true);
    let cursor = g.resize(8, 2, at(1, 0), true);
    assert_eq!(screen(&g), ["a", "b"]);
    assert_eq!(g.history_len(), 0);
    assert_eq!(cursor, at(1, 0));
    ok(&g);
}

#[test]
fn the_last_row_is_kept_when_it_says_the_line_goes_on() {
    let mut g = grid(6, 2);
    g.write_ascii(0, 0, b"x", &plain());
    g.write_ascii(1, 0, b"yz", &plain());
    // Nothing follows it, and no point of the grid is on it.
    g.set_wrapped(1, true);
    g.resize(8, 2, at(0, 0), true);
    assert_eq!(screen(&g), ["x", "yz"]);
    ok(&g);
}

#[test]
fn a_tab_mark_after_the_text_is_not_part_of_the_line() {
    let mut g = grid(10, 2);
    g.write_ascii(0, 0, b"ab", &plain());
    g.mark_tab(0, 5);
    assert!(g.live_cell(0, 5).is_some_and(|cell| cell.is_tab_start()), "the gap is marked");
    g.resize(12, 2, at(1, 0), true);
    assert_eq!(screen(&g)[0], "ab");
    assert!(!g.live_cell(0, 5).is_some_and(|cell| cell.is_tab_start()), "it marked a gap, not text");
    ok(&g);
}

#[test]
fn a_pin_on_a_double_width_row_is_kept_inside_a_narrower_screen() {
    let mut g = grid(10, 2);
    g.write_ascii(0, 0, b"abcdefghij", &plain());
    g.set_line_size(0, LineSize::DoubleWidth);
    let pin = g.pin(at(0, 9));
    g.resize(4, 2, at(1, 0), true);
    assert_eq!(g.pin_point(pin), Some(at(0, 3)), "the last column of the row, which is cut there");
    ok(&g);
}

#[test]
fn a_rewrapped_line_keeps_the_look_of_each_cell() {
    let mut sim = Sim::new(6, 4, DEFAULT_SCROLLBACK_BYTES);
    for (n, text) in [(1, "ab"), (2, "cd"), (3, "ef")] {
        sim.pen = colored(n);
        sim.print(text);
    }
    sim.pen = plain();
    sim.print("gh");
    sim.grid.resize(12, 4, at(1, 2), true);
    let looks: Vec<_> = (0..8).map(|x| sim.grid.style_at(at(0, x)).map(|style| style.fg)).collect();
    let indexed = |n| Some(Color::Indexed(n));
    let default = Some(Color::Named(NamedColor::Foreground));
    assert_eq!(
        looks,
        [indexed(1), indexed(1), indexed(2), indexed(2), indexed(3), indexed(3), default, default]
    );
    sim.check();
}

#[test]
fn the_looks_of_two_pages_with_the_same_number_are_two_looks_in_a_rewrapped_line() {
    let mut sim = Sim::new(6, 4, DEFAULT_SCROLLBACK_BYTES);
    // A page of one row: the line is on two pages, and each gave its first look the number 1.
    sim.grid = Grid::with_page_rows(6, 4, DEFAULT_SCROLLBACK_BYTES, 1);
    sim.pen = colored(1);
    sim.print("aaaaaa");
    sim.pen = colored(2);
    sim.print("bb");
    sim.grid.resize(12, 4, at(1, 2), true);
    let looks: Vec<_> = (0..8).map(|x| sim.grid.style_at(at(0, x)).map(|style| style.fg)).collect();
    let indexed = |n| Some(Color::Indexed(n));
    assert_eq!(
        looks,
        [indexed(1), indexed(1), indexed(1), indexed(1), indexed(1), indexed(1), indexed(2), indexed(2)]
    );
    sim.check();
}

#[test]
fn the_pages_a_rewrap_makes_are_numbered_after_the_ones_before() {
    let mut g = Grid::with_page_rows(10, 2, DEFAULT_SCROLLBACK_BYTES, 1);
    for _ in 0..3 {
        let region = full(&g);
        g.scroll_up(region, 1, BG);
    }
    let before: Vec<u64> = g.pages.iter().map(|page| page.uid).collect();
    g.resize(8, 2, at(0, 0), true);
    for page in &g.pages {
        assert!(!before.contains(&page.uid), "page {} has the number of one that was dropped", page.uid);
    }
    ok(&g);
}
