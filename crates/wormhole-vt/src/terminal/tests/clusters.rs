//! Mode 2027 and the zero-width characters without it.

use super::Rig;
use crate::types::{Flags, Line};

fn cell(rig: &Rig, x: usize) -> (String, bool) {
    let point = crate::types::Point::new(Line(0), crate::types::Column(x));
    let grid = rig.term.grid();
    let cell = grid.cell(point).map(|c| c.character()).unwrap_or(None);
    let mut text: String = cell.map(String::from).unwrap_or_default();
    text.extend(grid.marks_at(point));
    (text, grid.cell(point).is_some_and(|c| c.is_wide()))
}

#[test]
fn without_the_mode_marks_attach_and_nothing_gets_wider() {
    let mut rig = Rig::new(10, 2);
    rig.feed("\x1b[?2027l❤\u{fe0f}x👨\u{200d}👩");
    assert_eq!(cell(&rig, 0), ("❤\u{fe0f}".into(), false));
    assert_eq!(cell(&rig, 1).0, "x");
    assert_eq!(
        cell(&rig, 2),
        ("👨\u{200d}".into(), true),
        "the joiner joins; the next emoji is a character of its own"
    );
    assert_eq!(cell(&rig, 4), ("👩".into(), true));
    assert_eq!(rig.cursor(), (0, 6));
}

#[test]
fn a_cluster_may_be_built_across_feeds_but_not_across_a_cursor_move() {
    let mut rig = Rig::new(10, 2);
    rig.feed("\x1b[?2027hu");
    rig.feed("\u{308}");
    assert_eq!(cell(&rig, 0).0, "u\u{308}");
    rig.feed("v\x1b[Dw");
    assert_eq!(
        (cell(&rig, 0).0.as_str(), cell(&rig, 1).0.as_str()),
        ("u\u{308}", "w"),
        "the next character replaces what is there"
    );
    rig.feed("\x1b[1;1H\u{308}");
    assert_eq!(rig.cursor(), (0, 0), "a mark does not move the cursor");
    assert_eq!(
        cell(&rig, 0).0,
        "u\u{308}\u{308}",
        "at the start of a line it joins the cell under the cursor"
    );
}

#[test]
fn a_run_of_ascii_ends_with_a_character_a_mark_can_join() {
    let mut rig = Rig::new(10, 2);
    rig.feed("\x1b[?2027habc\u{301}d");
    assert_eq!((cell(&rig, 2).0.as_str(), cell(&rig, 3).0.as_str()), ("c\u{301}", "d"));
}

#[test]
fn flags_and_keycaps_and_skin_tones_are_one_cluster_each() {
    let mut rig = Rig::new(20, 2);
    rig.feed("\x1b[?2027h🇯🇵🇩🇪1\u{fe0f}\u{20e3}👍🏽");
    assert_eq!(cell(&rig, 0), ("🇯🇵".into(), true));
    assert_eq!(cell(&rig, 2), ("🇩🇪".into(), true));
    assert_eq!(cell(&rig, 4).0, "1\u{fe0f}\u{20e3}");
    assert_eq!(cell(&rig, 6), ("👍🏽".into(), true));
}

#[test]
fn a_cluster_that_widens_at_the_edge_of_a_margin_moves_whole() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[?2027h\x1b[1;10H❤\u{fe0f}x");
    // At the last column VS16 needs two cells: the cluster goes to the next line.
    assert_eq!(rig.row(0), "");
    assert_eq!(rig.row(1), "❤\u{fe0f}x");
    assert!(rig.term.is_wrapped(Line(0)));
    // With autowrap off it stays one cell wide: there is nowhere to go.
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[?2027h\x1b[?7l\x1b[1;10H❤\u{fe0f}x");
    assert_eq!(rig.row(1), "");
    assert_eq!(cell(&rig, 9).0, "x");
}

#[test]
fn a_wide_character_at_the_last_column_wraps_leaving_a_blank_and_obeys_autowrap() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[1;10H日");
    assert_eq!((rig.row(0), rig.row(1)), (String::new(), "日".into()));
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[?7l\x1b[1;10H日x");
    assert_eq!(cell(&rig, 9).0, "x", "without autowrap the wide character has no place and is dropped");
}

#[test]
fn zalgo_stays_bounded() {
    let mut rig = Rig::new(10, 2);
    rig.feed(&format!("a{}", "\u{301}".repeat(5000)));
    let marks = rig.term.grid().marks_at(crate::types::Point::new(Line(0), crate::types::Column(0))).len();
    assert_eq!(marks, crate::grid::MAX_MARKS);
}

#[test]
fn a_wide_character_that_does_not_fit_without_autowrap_leaves_no_trace() {
    // DECAWM off, one cell left: the wide character is dropped (the engine's convention, as
    // xterm leaves the cell alone), so REP has nothing new to repeat and no cluster is open.
    for clusters in ["h", "l"] {
        let mut rig = Rig::new(5, 2);
        rig.feed(&format!("\x1b[?7l\x1b[?2027{clusters}abcd\u{4e2d}"));
        assert_eq!(cell(&rig, 4).0, "", "{clusters}: nothing was printed");
        rig.feed("\x1b[b");
        assert_eq!(cell(&rig, 4).0, "d", "{clusters}: REP repeats the last character that was printed");
    }
}

/// The flags of the cell at `(y, x)` as the host draws it.
fn flags(rig: &Rig, y: i32, x: usize) -> Flags {
    let point = crate::types::Point::new(Line(y), crate::types::Column(x));
    rig.term.cell(point).map(|cell| cell.flags).unwrap_or_default()
}

#[test]
fn a_heart_made_wide_in_the_last_column_leaves_a_spacer_head_and_moves_to_the_next_row() {
    let mut rig = Rig::new(5, 3);
    // U+2764 is narrow until VS16 makes it an emoji: it no longer fits where it was.
    rig.feed("abcd\u{2764}\u{fe0f}");
    assert_eq!(rig.row(0), "abcd", "the heart left its cell");
    assert_eq!(rig.row(1), "\u{2764}\u{fe0f}");
    assert!(flags(&rig, 0, 4).contains(Flags::LEADING_WIDE_CHAR_SPACER), "the row's end is a spacer head");
    assert_eq!(rig.term.check_invariants(), Ok(()));
}

#[test]
fn a_heart_made_wide_at_the_right_margin_leaves_that_cell_blank_and_no_spacer_head() {
    let mut rig = Rig::new(8, 3);
    rig.feed("\x1b[?69h\x1b[1;5s");
    rig.feed("abcd\u{2764}\u{fe0f}");
    assert_eq!(rig.row(0), "abcd", "the heart left the margin's last cell blank");
    assert_eq!(rig.row(1), "\u{2764}\u{fe0f}");
    // The row ends at the margin, not at the screen's edge: there is no spacer head to put.
    assert!(!flags(&rig, 0, 7).contains(Flags::LEADING_WIDE_CHAR_SPACER));
    assert!(!flags(&rig, 0, 4).contains(Flags::LEADING_WIDE_CHAR_SPACER));
    assert_eq!(rig.term.check_invariants(), Ok(()));
}

#[test]
fn a_character_overwriting_the_last_column_without_autowrap_heads_its_cluster() {
    // DECAWM off: the character lands on the last cell, and a mark that follows joins it there.
    // The same for an ASCII run and for a character that is not ASCII.
    for text in ["abcde", "abcd\u{e9}", "abcdefg"] {
        let mut rig = Rig::new(5, 2);
        rig.feed("\x1b[?7l\x1b[?2027h");
        rig.feed(text);
        rig.feed("\u{308}");
        let last = text.chars().last().map(String::from).unwrap_or_default();
        assert_eq!(cell(&rig, 4).0, format!("{last}\u{308}"), "{text:?}: the mark joins the last column");
        assert_eq!(cell(&rig, 3).0, "d", "{text:?}: and not the column before");
    }
}
