//! What rectangle operations and REP cost, counted (not timed): style lookups, the work that made
//! a full-screen DECFRA thirty times slower per cell than ECH, and a repeat count that only
//! rewrote a full screen. The results are checked against the plain way to the same screen.

use super::Rig;
use crate::grid::style_acquires;
use crate::types::{Column, Line, Point};

const COLS: usize = 200;
const ROWS: usize = 50;

fn rig() -> Rig {
    Rig::with_scrollback(COLS, ROWS, 100)
}

/// The style lookups `bytes` cost on a 200 by 50 screen that `setup` prepared.
fn lookups(rig: &mut Rig, bytes: &str) -> usize {
    let before = style_acquires();
    rig.feed(bytes);
    style_acquires() - before
}

#[test]
fn a_full_screen_fill_looks_its_style_up_once_a_row() {
    let mut rig = rig();
    rig.feed("\x1b[38;2;10;20;30;48;2;1;2;3m");
    let acquires = lookups(&mut rig, "\x1b[88;1;1;50;200$x");
    assert!(acquires <= ROWS, "{acquires} lookups for {ROWS} rows");
    assert_eq!(rig.row(0), "X".repeat(COLS));
    assert_eq!(rig.row(ROWS - 1), "X".repeat(COLS));
}

#[test]
fn a_fill_keeps_its_link_and_its_edges_whole() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\u{4e00}\u{4e00}\u{4e00}\x1b]8;;http://a.example\x07\x1b[88;1;2;1;9$x");
    let link = |x: usize| rig.term.grid().link_at(Point::new(Line(0), Column(x)));
    assert_eq!(link(1).map(|l| l.uri.to_string()).as_deref(), Some("http://a.example"));
    assert_eq!(link(8).map(|l| l.uri.to_string()).as_deref(), Some("http://a.example"));
    assert!(link(0).is_none(), "outside the rectangle");
    assert!(link(9).is_none(), "outside the rectangle");
    assert_eq!(rig.row(0), " XXXXXXXX", "the wide character the fill cut in two is a blank");
}

#[test]
fn a_full_screen_erase_looks_its_style_up_once_a_row() {
    let mut rig = rig();
    rig.feed("\x1b[48;2;1;2;3m");
    let acquires = lookups(&mut rig, "\x1b[1;1;50;200$z");
    assert!(acquires <= ROWS, "{acquires} lookups for {ROWS} rows");
}

#[test]
fn a_selective_erase_looks_up_once_a_run_not_a_cell() {
    let mut rig = rig();
    // One protected cell in every row: each row is two runs the erase reaches.
    for y in 1..=ROWS {
        rig.feed(&format!("\x1b[{y};100H\x1b[1\"qP\x1b[0\"q"));
    }
    rig.feed("\x1b[48;2;1;2;3m");
    let acquires = lookups(&mut rig, "\x1b[1;1;50;200${");
    assert!(acquires <= 2 * ROWS, "{acquires} lookups for {ROWS} rows of two runs");
    assert_eq!(rig.row(0).chars().filter(|&c| c == 'P').count(), 1, "the protected cell stayed");
    assert_eq!(rig.row(0).len(), 100);
}

#[test]
fn a_full_screen_copy_looks_up_once_a_run_not_a_cell() {
    let mut rig = rig();
    // Three runs a row: two looks and the default.
    for y in 1..=ROWS {
        rig.feed(&format!("\x1b[{y};1H\x1b[31m{}\x1b[34m{}\x1b[0m", "r".repeat(60), "b".repeat(60)));
    }
    let acquires = lookups(&mut rig, "\x1b[1;1;50;200;1;1;1;1$v");
    assert!(acquires <= 3 * ROWS, "{acquires} lookups for {ROWS} rows of three runs");
    // The copy onto itself, shifted, overlapping: every cell moved with its look.
    let acquires = lookups(&mut rig, "\x1b[1;1;49;200;1;2;2;1$v");
    assert!(acquires <= 3 * ROWS, "{acquires} lookups");
    assert_eq!(rig.row(0), format!("{}{}", "r".repeat(60), "b".repeat(60)));
    // The column the copy did not reach keeps what it had.
    assert_eq!(rig.row(1), format!("r{}{}", "r".repeat(60), "b".repeat(60)));
    assert_eq!(rig.row(2), format!("r{}{}", "r".repeat(60), "b".repeat(60)));
    // A look went along with the characters: the red cell of row 1 is at column 1 now.
    let mut probe = Rig::new(4, 1);
    probe.feed("\x1b[31mab\x1b[0m\x1b[1;1;1;2;1;1;3;1$v");
    assert_eq!(probe.row(0), "abab");
    let reds = |x: usize| probe.term.grid().style_at(Point::new(Line(0), Column(x))).map(|s| s.fg);
    assert_eq!(reds(0), reds(2));
    assert_eq!(reds(1), reds(3));
}

/// Where the cursor stands and what the screen shows after one more character: the delayed wrap
/// is part of the cursor, and the next character shows which side of the edge it is on.
fn state(rig: &mut Rig) -> (Vec<String>, (usize, usize)) {
    let cursor = rig.cursor();
    rig.feed("Z");
    ((0..5).map(|y| rig.row(y)).collect(), cursor)
}

#[test]
fn a_repeat_count_of_several_screens_ends_where_the_whole_count_would() {
    // The screen is 20 by 5: 200 cells in two screens. The reference prints every copy as text.
    for x0 in [0usize, 7, 19] {
        for n in [65_535usize, 4_000, 400, 217, 201, 200] {
            let mut reference = Rig::with_scrollback(20, 5, 10);
            reference.feed(&format!("\x1b[1;{}H{}", x0 + 1, "a".repeat(n + 1)));
            let mut repeated = Rig::with_scrollback(20, 5, 10);
            repeated.feed(&format!("\x1b[1;{}Ha\x1b[{n}b", x0 + 1));
            assert_eq!(state(&mut repeated), state(&mut reference), "start {x0}, count {n}");
        }
    }
}

#[test]
fn a_big_repeat_count_is_cut_for_the_cost_and_left_alone_where_it_is_not_periodic() {
    let mut rig = Rig::with_scrollback(20, 5, 10);
    rig.feed("a");
    let acquires = lookups(&mut rig, "\x1b[65535b");
    assert!(acquires < 60, "{acquires} style lookups: the screen is rewritten twice, not 3,000 times");
    // A character the rule does not cover prints every copy: a wide one wraps in pairs.
    let mut wide = Rig::with_scrollback(20, 5, 10);
    wide.feed("\u{4e00}\x1b[65535b");
    let mut reference = Rig::with_scrollback(20, 5, 10);
    reference.feed(&"\u{4e00}".repeat(65_536));
    assert_eq!(state(&mut wide), state(&mut reference));
    // Left and right margins of their own: the period is theirs, so nothing is cut.
    let mut margins = Rig::with_scrollback(20, 5, 10);
    margins.feed("\x1b[?69h\x1b[5;15s\x1b[1;5Ha\x1b[65535b");
    let mut reference = Rig::with_scrollback(20, 5, 10);
    reference.feed(&format!("\x1b[?69h\x1b[5;15s\x1b[1;5H{}", "a".repeat(65_536)));
    assert_eq!(state(&mut margins), state(&mut reference));
}
