//! Arguments at or past the screen's edge: cut to the screen or ignored, and the screen is never
//! indexed outside (the grid asserts it in debug builds).

use super::{BG, at, plain, screen};
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid, Region, Style};
use crate::types::{Column, Flags, Line, Point};

fn grid(cols: usize, rows: usize) -> Grid {
    Grid::new(cols, rows, DEFAULT_SCROLLBACK_BYTES)
}

fn ok(grid: &Grid) {
    if let Err(e) = grid.check_invariants() {
        panic!("{e}");
    }
}

/// A grid of `cols` columns whose rows are `aaaa..`, `bbbb..`, ... one letter per row.
fn lettered(cols: usize, rows: usize) -> Grid {
    let mut g = grid(cols, rows);
    for y in 0..rows {
        let letter = b'a' + y as u8;
        g.write_ascii(y, 0, &vec![letter; cols], &plain());
    }
    g
}

#[test]
fn the_row_just_past_the_screen_is_not_a_row() {
    // One past the last row is the nearest wrong value: it must be refused, not indexed.
    let mut g = lettered(6, 3);
    g.write_spacer_head(3, &plain());
    g.restyle(3, 0..4, |style| Style { attrs: Flags::BOLD, ..*style });
    g.restyle(usize::MAX, 0..4, |style| Style { attrs: Flags::BOLD, ..*style });
    assert_eq!(screen(&g), ["aaaaaa", "bbbbbb", "cccccc"]);
    assert!(g.style_at(at(2, 0)).is_some_and(|style| style.attrs.is_empty()));
    ok(&g);
}

#[test]
fn an_area_past_the_screen_is_summed_as_the_screen_cut_to_it() {
    let mut g = grid(4, 3);
    g.write_ascii(0, 0, b"ab", &plain());
    g.write_ascii(2, 0, b"c", &plain());
    // a + b + two blanks, a row of blanks, c + three blanks: a blank counts as a space (32).
    let whole = Region { top: 0, bottom: 2, left: 0, right: 3 };
    assert_eq!(g.checksum(whole), 97 + 98 + 32 * 2 + 32 * 4 + 99 + 32 * 3);
    let beyond = Region { top: 0, bottom: usize::MAX, left: 0, right: usize::MAX };
    assert_eq!(g.checksum(beyond), g.checksum(whole));
    // Starting past the last row or column is the last one.
    let below = Region { top: 99, bottom: 120, left: 0, right: 3 };
    assert_eq!(g.checksum(below), g.checksum(Region { top: 2, bottom: 2, left: 0, right: 3 }));
    let right = Region { top: 0, bottom: 0, left: 99, right: 120 };
    assert_eq!(g.checksum(right), g.checksum(Region { top: 0, bottom: 0, left: 3, right: 3 }));
}

#[test]
fn a_resize_to_the_same_size_only_puts_the_cursor_on_the_screen() {
    let mut g = grid(10, 4);
    let c = g.resize(10, 4, Point::new(Line(99), Column(99)), true);
    assert_eq!(c, at(3, 9), "the last cell, not one past it");
    let c = g.resize(10, 4, Point::new(Line(-5), Column(2)), true);
    assert_eq!(c, at(0, 2));
    ok(&g);
}

#[test]
fn a_scroll_region_past_the_screen_is_the_screen_to_its_edge() {
    // Past the bottom and past the right edge, the region is cut to the screen: this one is the
    // whole screen, which scrolls into the history.
    let mut g = lettered(5, 3);
    g.scroll_up(Region { top: 0, bottom: usize::MAX, left: 0, right: usize::MAX }, 1, BG);
    assert_eq!(screen(&g), ["bbbbb", "ccccc", ""]);
    assert_eq!(g.history_len(), 1);
    ok(&g);

    // Only the bottom is past the screen.
    let mut g = lettered(5, 3);
    g.scroll_up(Region { top: 1, bottom: usize::MAX, left: 0, right: 4 }, 1, BG);
    assert_eq!(screen(&g), ["aaaaa", "ccccc", ""]);
    ok(&g);

    // Only the right edge is: still whole rows (a region short of the width moves part of them),
    // which a scroll that starts at the top shows by what it puts in the history.
    let mut g = lettered(5, 3);
    g.scroll_up(Region { top: 0, bottom: 2, left: 0, right: usize::MAX }, 1, BG);
    assert_eq!(screen(&g), ["bbbbb", "ccccc", ""]);
    assert_eq!(g.history_len(), 1, "a full-width scroll from the top row feeds the history");
    ok(&g);
}
