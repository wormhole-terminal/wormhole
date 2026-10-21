//! What a host reads back besides the screen's cells: the lines to restore, what a search saw at
//! a place, and whether a picture is placed.

use super::Rig;
use crate::types::{Column, Line, Point};

fn at(line: i32, column: usize) -> Point {
    Point::new(Line(line), Column(column))
}

#[test]
fn the_lines_to_restore_are_the_last_ones_asked_for_and_no_more() {
    let mut rig = Rig::new(10, 3);
    rig.feed("a\r\nb\r\nc");
    let lines = |rig: &Rig, n: usize| rig.term.restore_lines(n).expect("the main screen");
    assert_eq!(lines(&rig, 1), ["c"]);
    assert_eq!(lines(&rig, 2), ["b", "c"]);
    assert_eq!(lines(&rig, 3), ["a", "b", "c"]);
    assert_eq!(lines(&rig, 99), ["a", "b", "c"], "no more than there are");
    // History counts too.
    rig.feed("\r\nd\r\ne");
    assert_eq!(lines(&rig, 3), ["c", "d", "e"]);
    assert_eq!(lines(&rig, 4), ["b", "c", "d", "e"]);
}

#[test]
fn a_span_is_what_the_cells_there_spell() {
    let mut rig = Rig::new(10, 4);
    rig.feed("find the needle here");
    // "needle" is at columns 9..: row 0 "find the n", row 1 "eedle here".
    assert!(rig.term.span_is(at(0, 9), at(1, 4), "needle"));
    assert!(!rig.term.span_is(at(0, 9), at(1, 4), "needlx"));
    assert!(!rig.term.span_is(at(0, 9), at(1, 4), "needl"));
    assert!(rig.term.span_is(at(0, 0), at(0, 3), "find"));
}

#[test]
fn the_snapshot_says_when_a_picture_is_placed() {
    let mut rig = Rig::new(10, 3);
    assert!(!rig.term.has_placements());
    // One raw RGB pixel, shown at the cursor; `q=2` keeps the answer back.
    rig.feed("\x1b_Ga=T,f=24,s=1,v=1,i=1,q=2;AAAA\x1b\\");
    assert!(rig.term.has_placements());
    let mut placed = Vec::new();
    rig.term.placements(&mut placed);
    assert_eq!(placed.len(), 1);
}
