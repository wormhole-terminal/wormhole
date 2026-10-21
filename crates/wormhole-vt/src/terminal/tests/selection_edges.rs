//! A selection at the edges of the screen, a bracket's two ends, and the history under it going
//! away: the cases `selection` leaves out.

use super::Rig;
use crate::types::{Column, Line, Point, SelectionType, Side};

fn at(line: i32, column: usize) -> Point {
    Point::new(Line(line), Column(column))
}

fn select(rig: &mut Rig, kind: SelectionType, from: (i32, usize, Side), to: (i32, usize, Side)) {
    rig.term.start_selection(kind, at(from.0, from.1), from.2);
    rig.term.update_selection(at(to.0, to.1), to.2);
}

const L: Side = Side::Left;
const R: Side = Side::Right;

#[test]
fn a_selection_may_start_on_the_last_row_of_the_screen() {
    let mut rig = Rig::new(10, 3);
    rig.feed("abc\r\ndef\r\nghi");
    select(&mut rig, SelectionType::Simple, (2, 0, L), (2, 2, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("ghi"));
    select(&mut rig, SelectionType::Block, (2, 1, L), (2, 2, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("hi"));
    select(&mut rig, SelectionType::Semantic, (2, 1, L), (2, 1, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("ghi"));
    select(&mut rig, SelectionType::Lines, (2, 0, L), (2, 0, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("ghi\n"));
}

#[test]
fn a_bracket_selection_runs_from_the_opening_bracket_whichever_end_was_clicked() {
    let mut rig = Rig::new(20, 4);
    rig.feed("f(a, (b), c) tail");
    let range = |rig: &Rig| rig.term.selection_range().map(|r| (r.start, r.end));
    for column in [1, 11] {
        select(&mut rig, SelectionType::Semantic, (0, column, L), (0, column, L));
        assert_eq!(range(&rig), Some((at(0, 1), at(0, 11))), "a click at column {column}");
    }
}

#[test]
fn a_selection_whose_start_left_the_history_while_its_end_stayed_is_gone() {
    let mut rig = Rig::new(10, 4);
    // Fill the history to its limit (a 10-column line costs 96 bytes, and the limit is a few
    // hundred kilobytes), so that the oldest rows leave as new ones arrive.
    rig.feed(&"x\r\n".repeat(30_000));
    let oldest = -(rig.term.history_len() as i32);
    select(&mut rig, SelectionType::Simple, (oldest, 0, L), (3, 3, R));
    assert!(rig.term.selection_range().is_some());
    // The end is on the screen, newer than a whole history: a little output at a time, the start's
    // row is the first to go.
    let mut gone = false;
    for _ in 0..1500 {
        rig.feed(&"x\r\n".repeat(20));
        if rig.term.selection_range().is_none() {
            gone = true;
            break;
        }
    }
    assert!(gone, "the start left the history");
    assert!(!rig.term.take_selection(), "it was dropped with its start, not kept for its end");
}
