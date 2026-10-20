//! A copy keeps tabs: HT marks the blank cell it starts on, and a gap that is still blank is one
//! `\t` in the copied text; once a program wrote into the gap, the copy says what is there.

use crate::terminal::tests::Rig;
use crate::types::SelectionType;
use crate::types::{Column, Line, Point, Side};

fn copy_row(rig: &mut Rig, row: i32) -> String {
    let cols = rig.term.columns();
    rig.term.start_selection(SelectionType::Simple, Point::new(Line(row), Column(0)), Side::Left);
    rig.term.update_selection(Point::new(Line(row), Column(cols - 1)), Side::Right);
    rig.term.selection_text().unwrap_or_default()
}

#[test]
fn a_blank_tab_gap_is_copied_as_a_tab() {
    let mut rig = Rig::new(30, 3);
    rig.feed("a\tb\t\tc");
    assert_eq!(copy_row(&mut rig, 0), "a\tb\t\tc");
    assert_eq!(rig.row(0), "a       b               c", "the screen is unchanged");
}

#[test]
fn a_gap_cut_short_by_the_screens_edge_is_spaces() {
    let mut rig = Rig::new(10, 3);
    rig.feed("12345678\tx\r\n123456789\ty");
    // The tab from column 8 stops at the last column, before a stop: the gap isn't a full one.
    assert_eq!(copy_row(&mut rig, 0), "12345678 x");
    assert_eq!(copy_row(&mut rig, 1), "123456789y", "a tab that goes nowhere marks nothing");
}

#[test]
fn a_gap_a_program_wrote_into_is_copied_as_what_is_there() {
    let mut rig = Rig::new(30, 3);
    rig.feed("a\tb\r\na\tb\x1b[2;4H \x1b[2;1H");
    assert_eq!(copy_row(&mut rig, 1), "a       b", "a space typed into the gap");
    rig.feed("\x1b[1;1Ha\tb\x1b[1;5HX");
    assert_eq!(copy_row(&mut rig, 0), "a   X   b", "a letter typed into the gap");
}

#[test]
fn spaces_typed_after_a_tab_stay_spaces() {
    let mut rig = Rig::new(30, 3);
    rig.feed("a\t  b");
    assert_eq!(copy_row(&mut rig, 0), "a\t  b");
}

#[test]
fn erasing_the_marked_cell_takes_the_tab_but_erasing_inside_the_gap_does_not() {
    let mut rig = Rig::new(30, 3);
    rig.feed("a\tb\x1b[1;3H\x1b[2X");
    assert_eq!(copy_row(&mut rig, 0), "a\tb", "the gap is still blank");
    rig.feed("\x1b[1;2H\x1b[X");
    assert_eq!(copy_row(&mut rig, 0), "a       b", "the marked cell is blank again, and plain");
}

#[test]
fn a_selection_ending_inside_a_gap_copies_the_cells_it_covers() {
    let mut rig = Rig::new(30, 3);
    rig.feed("a\tb");
    rig.term.start_selection(SelectionType::Simple, Point::new(Line(0), Column(0)), Side::Left);
    rig.term.update_selection(Point::new(Line(0), Column(3)), Side::Right);
    assert_eq!(rig.term.selection_text().as_deref(), Some("a   "));
}

#[test]
fn a_tab_hop_from_cht_marks_each_stop() {
    let mut rig = Rig::new(30, 3);
    rig.feed("a\x1b[2Ib");
    assert_eq!(copy_row(&mut rig, 0), "a\t\tb");
}

#[test]
fn a_searches_text_still_has_one_character_per_cell() {
    let mut rig = Rig::new(30, 3);
    rig.feed("a\tb");
    assert_eq!(
        rig.term.grid().span_text(Point::new(Line(0), Column(0)), Point::new(Line(0), Column(8))),
        "a       b"
    );
}

#[test]
fn a_picture_printed_as_placeholders_is_not_copied() {
    let mut rig = Rig::new(30, 3);
    rig.feed("ok \u{10EEEE}\u{0305}\u{030D}\u{10EEEE}\u{0305}\u{030E} done");
    assert_eq!(copy_row(&mut rig, 0), "ok  done");
}
