//! Printing text where it meets an edge: right of the right margin, at the end of a row, and where
//! a character that adds nothing to the width joins what is before it.

use super::Rig;
use crate::types::{Column, Line, Point};

#[test]
fn text_printed_right_of_the_right_margin_runs_to_the_edge_of_the_screen() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[?69h\x1b[1;5s\x1b[1;8Habcd");
    assert_eq!(rig.row(0), "       abc");
    assert_eq!(rig.row(1), "d", "and goes on at the start of the next row, not at the margin");
    assert_eq!(rig.term.check_invariants(), Ok(()));
}

#[test]
fn a_row_that_wraps_is_marked_so_even_while_another_row_is_double_width() {
    let mut rig = Rig::new(10, 4);
    rig.feed("\x1b[3;1H\x1b#6\x1b[1;1Habcdefghijkl");
    let wrapped = |rig: &Rig, y: i32| rig.term.grid().row_view(Line(y)).map(|row| row.wrapped());
    assert_eq!(wrapped(&rig, 0), Some(true), "it goes on in the next row");
    assert_eq!(rig.row(1), "kl");
    // A double-width row has no soft wrap: what it shows is not joined with the row below.
    rig.feed("\x1b[3;1Huvwxyz");
    assert_eq!(rig.row(2), "uvwxy");
    assert_eq!(wrapped(&rig, 2), Some(false));
    assert_eq!(rig.row(3), "z");
}

/// The column of the first row that U+0301 joins, on a screen of five columns without clusters
/// (mode 2027), after `setup`.
fn mark_joins(setup: &str) -> usize {
    let mut rig = Rig::new(5, 2);
    rig.feed("\x1b[?2027l");
    rig.feed(setup);
    rig.feed("\u{301}");
    let marked = |x: usize| rig.term.grid().marks_at(Point::new(Line(0), Column(x))).contains(&'\u{301}');
    (0..5).find(|&x| marked(x)).expect("a cell has the mark")
}

#[test]
fn a_mark_joins_the_character_before_the_cursor_or_the_one_it_stands_on_when_that_is_the_last() {
    // Autowrap pending after the last column: the character under the cursor.
    assert_eq!(mark_joins("abcde"), 4);
    // Autowrap off, the last column printed: the cursor stays on that character.
    assert_eq!(mark_joins("\x1b[?7labcde"), 4);
    // Otherwise the one before the cursor, wherever the cursor is: here after text, with and without
    // autowrap.
    assert_eq!(mark_joins("ab"), 1);
    assert_eq!(mark_joins("\x1b[?7lab"), 1);
    // A cursor moved onto a character does not make the mark join that one.
    assert_eq!(mark_joins("abc\x1b[1;2H"), 0);
    assert_eq!(mark_joins("\x1b[?7labc\x1b[1;2H"), 0);
}
