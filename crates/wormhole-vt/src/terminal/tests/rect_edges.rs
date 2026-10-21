//! Rectangle editing at its edges: attributes given twice, rectangles past the page, the stream's
//! default end and the characters DECFRA fills with. The conformance cases (`rect.vtt`,
//! `rect-attributes.vtt`) hold the rest.

use super::Rig;
use crate::types::{Column, Flags, Line, Point};

fn bold(rig: &Rig, line: i32, column: usize) -> bool {
    rig.term.cell(Point::new(Line(line), Column(column))).is_some_and(|cell| cell.flags.contains(Flags::BOLD))
}

/// The bold cells of the screen, as `row:column` pairs.
fn bold_cells(rig: &Rig) -> Vec<(i32, usize)> {
    let (cols, rows) = rig.term.size();
    (0..rows as i32).flat_map(|y| (0..cols).map(move |x| (y, x))).filter(|&(y, x)| bold(rig, y, x)).collect()
}

#[test]
fn an_attribute_changed_twice_in_one_sequence_is_changed() {
    let mut rig = Rig::new(5, 2);
    rig.feed("abc\x1b[1;1;1;3;1;1$r");
    assert_eq!(bold_cells(&rig), [(0, 0), (0, 1), (0, 2)], "bold and bold again is bold");
    rig.feed("\x1b[1;1;1;3;22;22$r");
    assert_eq!(bold_cells(&rig), [], "and so is clearing it twice");
}

#[test]
fn a_rectangle_below_or_right_of_the_page_is_cut_to_its_last_row_and_column() {
    let mut rig = Rig::new(5, 3);
    rig.feed("aaaaa\r\nbbbbb\r\nccccc\x1b[2*x");
    rig.feed("\x1b[99;1;99;99;1$r");
    assert_eq!(bold_cells(&rig), [(2, 0), (2, 1), (2, 2), (2, 3), (2, 4)]);
    rig.feed("\x1b[0m\x1b[1;1;99;99;22$r");
    rig.feed("\x1b[1;99;99;99;1$r");
    assert_eq!(bold_cells(&rig), [(0, 4), (1, 4), (2, 4)]);
}

#[test]
fn a_stream_without_a_last_row_runs_to_its_right_edge_on_the_last_row_of_the_page() {
    let mut rig = Rig::new(5, 3);
    rig.feed("aaaaa\r\nbbbbb\r\nccccc");
    // From the first cell to the third column of the last row; the bottom row is left out.
    rig.feed("\x1b[1;1;;3;1$r");
    let mut expected: Vec<(i32, usize)> = (0..2).flat_map(|y| (0..5).map(move |x| (y, x))).collect();
    expected.extend([(2, 0), (2, 1), (2, 2)]);
    assert_eq!(bold_cells(&rig), expected);
}

#[test]
fn decfra_fills_with_any_character_one_cell_wide() {
    let mut rig = Rig::new(6, 3);
    // U+0100, a letter of one cell, and U+00A1, from the upper half of the VT510 set.
    rig.feed("\x1b[256;1;1;1;3$x\x1b[161;2;1;2;2$x");
    assert_eq!(rig.row(0), "\u{100}\u{100}\u{100}");
    assert_eq!(rig.row(1), "\u{a1}\u{a1}");
    // Nothing for a wide character (U+4E00), a combining mark (U+0301), a C1 control or DEL.
    for (code, what) in
        [(20224, "a wide character"), (769, "a combining mark"), (133, "a C1 control"), (127, "DEL")]
    {
        rig.feed(&format!("\x1b[{code};3;1;3;3$x"));
        assert_eq!(rig.row(2), "", "{what}");
    }
}
