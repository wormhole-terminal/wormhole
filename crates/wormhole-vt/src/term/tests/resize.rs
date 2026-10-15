//! Resizing, DECCOLM, the history cap.

use super::Rig;
use crate::term::Event;
use crate::types::Line;

#[test]
fn a_cursor_waiting_to_wrap_still_waits_after_the_row_widens_or_narrows() {
    let mut rig = Rig::new(10, 4);
    rig.feed("abcdefghij");
    rig.term.resize(20, 4);
    assert_eq!(rig.cursor(), (0, 10), "after the text now");
    rig.feed("k");
    assert_eq!(rig.row(0), "abcdefghijk");
    let mut rig = Rig::new(10, 4);
    rig.feed("abcdefghij");
    rig.term.resize(10, 6);
    rig.feed("k");
    assert_eq!(
        (rig.row(0), rig.row(1)),
        ("abcdefghij".into(), "k".into()),
        "a height change leaves the wait alone"
    );
}

#[test]
fn the_alternate_screen_is_cropped_and_the_main_one_reflowed() {
    let mut rig = Rig::new(10, 4);
    rig.feed("abcdefghijkl\x1b[?1049h\x1b[1;1HXXXXXXXXXX");
    rig.term.resize(5, 4);
    assert_eq!(rig.row(0), "XXXXX", "cropped");
    assert!(!rig.term.is_wrapped(Line(0)));
    rig.feed("\x1b[?1049l");
    assert_eq!(
        (rig.row(0), rig.row(1), rig.row(2)),
        ("abcde".into(), "fghij".into(), "kl".into()),
        "reflowed"
    );
}

#[test]
fn the_cursor_saved_for_the_main_screen_follows_its_text_through_a_resize() {
    let mut rig = Rig::new(10, 4);
    rig.feed("abcdefghijkl\x1b[?1049h");
    // 1049 saved the cursor at the end of "kl" (row 2, column 2); narrowing moves that text.
    rig.term.resize(5, 4);
    rig.feed("\x1b[?1049l");
    assert_eq!(rig.cursor(), (2, 2), "after \"kl\" on its new row");
}

#[test]
fn a_resize_ends_the_scrolling_region_and_gives_new_columns_tab_stops() {
    let mut rig = Rig::new(10, 6);
    rig.feed("\x1b[2;4r\x1b[3g");
    rig.term.resize(24, 6);
    rig.feed("\x1b[6;1H\n");
    assert_eq!(rig.term.history_size(), 1, "the region is gone: the whole screen scrolled");
    rig.feed("\x1b[1;1H\tx");
    assert_eq!(rig.row(0), format!("{}x", " ".repeat(16)), "the stops of the columns that came");
}

#[test]
fn a_resize_to_the_same_size_does_nothing() {
    let mut rig = Rig::new(10, 4);
    rig.feed("abcdefghij");
    rig.term.drain_events().for_each(drop);
    rig.term.resize(10, 4);
    rig.feed("k");
    assert_eq!(rig.row(1), "k");
}

#[test]
fn deccolm_needs_mode_40_and_asks_the_host_for_the_width() {
    let mut rig = Rig::new(100, 5);
    rig.feed("text\x1b[?3h");
    assert_eq!(rig.term.columns(), 100, "not allowed");
    assert_eq!(rig.row(0), "text");
    rig.feed("\x1b[?40h\x1b[?3h");
    assert_eq!(rig.term.columns(), 132);
    assert_eq!(rig.row(0), "", "the screen is cleared");
    assert_eq!(rig.cursor(), (0, 0));
    assert!(rig.events().contains(&Event::ColumnsRequested(132)));
    rig.feed("keep\x1b[?95h\x1b[?3l");
    assert_eq!(rig.term.columns(), 80);
    assert_eq!(rig.row(0), "keep", "DECNCSM keeps the screen");
    assert!(rig.events().contains(&Event::ColumnsRequested(80)));
}

#[test]
fn the_history_keeps_at_least_the_lines_asked_for_and_not_much_more() {
    for lines in [0, 3, 50, 1000] {
        let mut rig = Rig::new(40, 5);
        rig.term.set_scrollback_lines(lines);
        for i in 0..5000 {
            rig.feed(&format!("line {i}\r\n"));
        }
        let kept = rig.term.history_size();
        assert!(kept >= lines, "{kept} lines kept of {lines} asked");
        // Whole pages go at once; a page is at most 128 KiB of cells.
        assert!(kept <= lines + 3 * 128 * 1024 / (40 * 8), "{kept} lines kept of {lines} asked");
        if lines > 0 {
            let newest = rig.term.row_text(Line(-1), true);
            assert_eq!(newest, "line 4995");
        }
    }
}

#[test]
fn a_new_terminal_clamps_a_silly_size() {
    let term = crate::term::Terminal::new(0, 0, 0);
    assert!(term.columns() >= 2 && term.screen_lines() >= 1);
    let mut rig = Rig::new(10, 4);
    rig.term.resize(1_000_000, 1_000_000);
    assert!(rig.term.columns() <= crate::grid::MAX_COLS && rig.term.screen_lines() <= crate::grid::MAX_ROWS);
    rig.feed("still works\r\n");
}

#[test]
fn the_history_limit_belongs_to_the_main_screen_whichever_one_shows() {
    let mut rig = Rig::new(10, 4);
    rig.feed("\x1b[?1049h");
    rig.term.resize(12, 5);
    assert_eq!(rig.term.grid().scrollback_limit(), 0, "the alternate screen has no history");
    rig.term.set_scrollback_lines(7);
    assert_eq!(rig.term.grid().scrollback_limit(), 0);
    rig.feed("\x1b[?1049l");
    assert_eq!(rig.term.grid().scrollback_limit(), super::super::history_bytes(7, 12));
}
