use wormhole_vt::{Column, Line, Point, Scroll, SelectionType, Side};

use super::{FoldSpan, MIN_HIDDEN, Refused, Request};
use crate::ScreenSnapshot;
use crate::testing::TestTerm;

const FINISHED: Request = Request { finished: true, failed: false };
const UNMARKED: Request = Request { finished: false, failed: false };

/// 10 × 4 with history, after 30 lines "L0" … "L29": the screen shows L26 … L29 (grid lines 0 to 3, the
/// cursor on 3), L25 is grid line -1.
fn filled() -> TestTerm {
    let mut term = TestTerm::new(10, 4, 100);
    let text: Vec<String> = (0..30).map(|n| format!("L{n}")).collect();
    term.feed(text.join("\r\n").as_bytes());
    term
}

fn spans(term: &mut TestTerm) -> Vec<FoldSpan> {
    let mut out = Vec::new();
    term.screen_mut().fold_spans(&mut out);
    out
}

fn fold(term: &mut TestTerm, first: i32, last: i32) -> Result<u64, Refused> {
    term.screen_mut().fold_add(Line(first), Line(last), FINISHED)
}

#[test]
fn the_live_screen_and_the_rows_near_the_cursor_are_never_folded() {
    let mut term = filled();
    assert_eq!(fold(&mut term, -10, 3), Ok(1));
    assert_eq!(spans(&mut term)[0].last, Line(-1), "a finished block is cut where the screen begins");
    assert_eq!(fold(&mut term, 0, 3), Err(Refused::Size), "nothing of the live screen is left to fold");
    // Rows with no end mark need a whole screen between them and the cursor (line 3, 4 rows).
    let id = term.screen_mut().fold_add(Line(-30), Line(-12), UNMARKED);
    assert_eq!(id, Ok(2));
    assert_eq!(term.screen_mut().fold_add(Line(-3), Line(-1), UNMARKED), Err(Refused::Size));
    assert_eq!(fold(&mut term, -9, -5), Err(Refused::Overlaps));
    assert!(MIN_HIDDEN > 2 && fold(&mut term, -50, -49).is_err_and(|e| e == Refused::Size));
}

#[test]
fn a_fold_follows_its_rows_and_hides_them_as_one() {
    let mut term = filled();
    fold(&mut term, -12, -4).unwrap();
    term.feed(b"\r\nmore\r\nmore");
    let span = spans(&mut term)[0];
    assert_eq!((span.first, span.last, span.hidden()), (Line(-14), Line(-6), 9), "it scrolled with the text");

    term.scroll_display(Scroll::Delta(8));
    let mut snapshot = ScreenSnapshot::default();
    term.snapshot(&mut snapshot);
    let marker = snapshot.rows().iter().position(|row| row.fold().is_some()).expect("a marker row");
    assert_eq!(snapshot.rows().len(), 4);
    assert_eq!(snapshot.rows()[marker].fold().map(FoldSpan::hidden), Some(9));
    assert_eq!(snapshot.row_of_line(Line(-10)), None, "a hidden line has no row");
    assert_eq!(snapshot.row_of_line(snapshot.rows()[marker + 1].line()), Some(marker + 1));
}

#[test]
fn copying_across_a_fold_gives_the_hidden_text() {
    let mut term = filled();
    fold(&mut term, -12, -4).unwrap();
    term.start_selection(SelectionType::Simple, Point::new(Line(-14), Column(0)), Side::Left);
    term.update_selection(Point::new(Line(-2), Column(1)), Side::Right);
    let copied = term.screen().selection_text().expect("selected");
    assert!(copied.contains("L18") && copied.contains("L23"), "the folded rows are in it: {copied}");
    let mut snapshot = ScreenSnapshot::default();
    term.scroll_display(Scroll::Delta(9));
    term.snapshot(&mut snapshot);
    let marker = snapshot.rows().iter().position(|row| row.fold().is_some()).unwrap();
    assert_eq!(snapshot.selected_columns(marker), Some((0, 9)), "the marker shows the selection");
}

#[test]
fn scrolling_counts_shown_rows() {
    let mut term = filled();
    fold(&mut term, -12, -4).unwrap();
    term.screen_mut().scroll_shown_rows(7);
    assert_eq!(term.display_offset(), 7, "3 lines, then the fold's last line (-4) is the bottom row");
    term.screen_mut().scroll_shown_rows(1);
    assert_eq!(term.display_offset(), 16, "one step takes the whole fold");
    term.screen_mut().scroll_shown_rows(-1);
    assert_eq!(term.display_offset(), 7);
}

#[test]
fn a_fold_dissolves_when_its_rows_leave_the_history_or_reach_the_screen() {
    let mut term = filled();
    fold(&mut term, -12, -4).unwrap();
    term.clear_history();
    assert!(spans(&mut term).is_empty(), "clearing the history takes it");

    let mut term = filled();
    fold(&mut term, -12, -4).unwrap();
    term.resize(10, 20);
    assert!(spans(&mut term).is_empty(), "a taller screen pulled its rows back under the program's hands");

    let mut term = filled();
    fold(&mut term, -12, -4).unwrap();
    term.feed("\r\nx".repeat(20_000).as_bytes());
    assert!(term.history_size() < 20_000, "the history is capped");
    assert!(spans(&mut term).is_empty(), "the history dropped them");
    assert!(!term.screen_mut().has_folds(), "and their pins were let go of");
}

#[test]
fn a_fold_reflows_with_its_text() {
    let mut term = TestTerm::new(10, 4, 200);
    let text: Vec<String> = (0..30).map(|n| format!("line {n:02}xx")).collect();
    term.feed(text.join("\r\n").as_bytes());
    fold(&mut term, -12, -4).unwrap();
    let before = spans(&mut term)[0].hidden();
    term.resize(5, 4);
    let after = spans(&mut term)[0].hidden();
    assert!(after >= before * 2 - 2, "wrapped rows are hidden too: {before} rows became {after}");
}

#[test]
fn what_needs_the_person_is_never_under_a_fold() {
    let mut term = filled();
    fold(&mut term, -20, -8).unwrap();
    fold(&mut term, -6, -3).unwrap();
    term.screen_mut().fold_set_needs_you(true);
    let open: Vec<bool> = spans(&mut term).iter().map(|span| span.expanded).collect();
    assert_eq!(open, [false, true], "the one in the last two screens opens");
    assert_eq!(fold(&mut term, -30, -25), Err(Refused::NeedsYou));
    term.screen_mut().fold_set_needs_you(false);
    assert!(
        spans(&mut term).iter().all(|span| !span.expanded),
        "and closes again unless the person opened it"
    );
}

#[test]
fn a_full_screen_program_or_mouse_reporting_has_no_folds() {
    let mut term = filled();
    let id = fold(&mut term, -12, -4).unwrap();
    term.feed(b"\x1b[?1049h");
    assert_eq!(fold(&mut term, -20, -15), Err(Refused::NotNow));
    assert!(spans(&mut term).is_empty());
    term.feed(b"\x1b[?1049l\x1b[?1000h");
    assert!(spans(&mut term).is_empty(), "mouse reporting is by screen row");
    term.feed(b"\x1b[?1000l");
    assert_eq!(spans(&mut term).iter().map(|span| span.id).collect::<Vec<_>>(), [id], "the fold waited");
    assert!(term.screen_mut().fold_toggle(id));
}
