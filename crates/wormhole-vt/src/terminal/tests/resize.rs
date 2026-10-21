//! Resizing, DECCOLM, the history cap.

use super::Rig;
use crate::terminal::{Event, Policy};
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
    assert_eq!(rig.term.history_len(), 1, "the region is gone: the whole screen scrolled");
    rig.feed("\x1b[1;1H\tx");
    assert_eq!(rig.row(0), format!("{}x", " ".repeat(16)), "the stops of the columns that came");
}

#[test]
fn a_resize_to_the_same_size_does_nothing() {
    let mut rig = Rig::new(10, 4);
    rig.feed("abcdefghij");
    rig.term.events().for_each(drop);
    rig.term.resize(10, 4);
    rig.feed("k");
    assert_eq!(rig.row(1), "k");
}

#[test]
fn deccolm_clears_and_homes_but_keeps_the_width_without_window_operations() {
    // VT510 DECCOLM: clear the screen, reset the margins, home the cursor. The width is the
    // window's, which a program may change only with `Policy::window_ops`.
    let mut rig = Rig::new(100, 5);
    rig.feed("text\x1b[2;4r\x1b[?40h\x1b[?3h");
    assert_eq!((rig.term.cols(), rig.term.rows()), (100, 5));
    assert_eq!(rig.row(0), "", "cleared");
    assert_eq!(rig.cursor(), (0, 0));
    assert!(rig.term.modes().contains(crate::types::Modes::COLUMNS_132), "still recorded");
    assert!(!rig.events().iter().any(|event| matches!(event, Event::ColumnsRequested(_))));
    rig.feed("\x1b[?3l");
    assert_eq!(rig.term.cols(), 100);
    assert!(!rig.term.modes().contains(crate::types::Modes::COLUMNS_132));
}

#[test]
fn a_stream_of_deccolm_toggles_neither_resizes_nor_touches_the_history() {
    // The audit's storm: `?40h` once, then `?3h ?3l` over and over reflowed the whole history twice
    // a toggle. The width not changing is what shows no reflow ran: a reflow needs a new width.
    let mut rig = Rig::with_scrollback(80, 24, 10_000);
    for i in 0..10_000 {
        rig.feed(&format!("line {i}\r\n"));
    }
    let history = rig.term.history_len();
    assert!(history > 9_000, "{history}");
    rig.events();
    let mut storm = String::from("\x1b[?40h");
    for _ in 0..2_000 {
        storm.push_str("\x1b[?3h\x1b[?3l");
    }
    rig.feed(&storm);
    assert_eq!((rig.term.cols(), rig.term.rows()), (80, 24));
    assert_eq!(rig.term.history_len(), history);
    let events = rig.events();
    assert!(
        !events.iter().any(|event| matches!(event, Event::ColumnsRequested(_))),
        "no resize was asked for: {} events",
        events.len()
    );
}

#[test]
fn deccolm_toggles_with_window_operations_reflow_once_a_change_and_not_at_all_when_the_width_matches() {
    let mut rig = Rig::new(80, 5);
    rig.term.set_policy(crate::terminal::Policy { window_ops: true, ..Default::default() });
    rig.feed("\x1b[?40h\x1b[?3l");
    assert!(
        !rig.events().iter().any(|event| matches!(event, Event::ColumnsRequested(_))),
        "80 columns asked of an 80 column screen: nothing to ask the host"
    );
    rig.feed("\x1b[?3h");
    assert_eq!(rig.term.cols(), 132);
    assert!(rig.events().contains(&Event::ColumnsRequested(132)));
}

#[test]
fn deccolm_needs_mode_40_and_asks_the_host_for_the_width() {
    let mut rig = Rig::new(100, 5);
    rig.term.set_policy(crate::terminal::Policy { window_ops: true, ..Default::default() });
    rig.feed("text\x1b[?3h");
    assert_eq!(rig.term.cols(), 100, "not allowed");
    assert_eq!(rig.row(0), "text");
    rig.feed("\x1b[?40h\x1b[?3h");
    assert_eq!(rig.term.cols(), 132);
    assert_eq!(rig.row(0), "", "the screen is cleared");
    assert_eq!(rig.cursor(), (0, 0));
    assert!(rig.events().contains(&Event::ColumnsRequested(132)));
    rig.feed("keep\x1b[?95h\x1b[?3l");
    assert_eq!(rig.term.cols(), 80);
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
        let kept = rig.term.history_len();
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
fn the_history_cap_is_the_lines_at_their_width_and_three_pages_of_slack() {
    use crate::terminal::screen::history::history_bytes;
    // A cell is eight bytes and a row's record sixteen; the grid drops whole pages of up to 128 KiB,
    // so the cap allows three more.
    assert_eq!(history_bytes(0, 80), 3 * 128 * 1024);
    assert_eq!(history_bytes(100, 80), 100 * (80 * 8 + 16) + 3 * 128 * 1024);
    assert_eq!(history_bytes(usize::MAX, 80), usize::MAX, "no overflow");
}

#[test]
fn a_saved_cursor_stays_on_a_screen_that_shrinks() {
    let mut rig = Rig::new(10, 5);
    rig.feed("\x1b[5;10H\x1b7");
    rig.term.resize(6, 3);
    assert_eq!(rig.term.check_invariants(), Ok(()));
    rig.feed("\x1b8X");
    assert_eq!(rig.row(2), "     X", "restored at the last cell of the smaller screen");
}

#[test]
fn a_full_reset_goes_back_to_80_columns_only_from_the_132_a_program_asked_for() {
    let allowed = || crate::terminal::Policy { window_ops: true, ..Default::default() };
    // DECCOLM to 132 columns, then RIS: the width goes back, and the host is told.
    let mut rig = Rig::new(80, 5);
    rig.term.set_policy(allowed());
    rig.feed("\x1b[?40h\x1b[?3h");
    assert_eq!(rig.term.cols(), 132);
    rig.events();
    rig.feed("\x1bc");
    assert_eq!(rig.term.cols(), 80);
    assert!(rig.events().contains(&Event::ColumnsRequested(80)));
    // A screen DECCOLM never touched stays as the window has it, whatever its width.
    let mut rig = Rig::new(100, 5);
    rig.term.set_policy(allowed());
    rig.feed("\x1bc");
    assert_eq!(rig.term.cols(), 100);
    assert!(!rig.events().iter().any(|event| matches!(event, Event::ColumnsRequested(_))));
    // DECCOLM without the window operations only records the mode: there is no width to undo.
    let mut rig = Rig::new(100, 5);
    rig.feed("\x1b[?40h\x1b[?3h\x1bc");
    assert_eq!(rig.term.cols(), 100);
}

#[test]
fn a_new_terminal_clamps_a_silly_size() {
    let term = crate::terminal::Terminal::builder().size(0, 0).scrollback(0).build();
    assert!(term.cols() >= 2 && term.rows() >= 1);
    // One side at a time: both at their limit is a 2 GB screen, more than a test should take.
    let mut rig = Rig::new(10, 4);
    rig.term.resize(1_000_000, 3);
    assert_eq!((rig.term.cols(), rig.term.rows()), (crate::grid::MAX_COLS, 3));
    rig.feed("still works\r\n");
    rig.term.resize(3, 1_000_000);
    assert_eq!((rig.term.cols(), rig.term.rows()), (3, crate::grid::MAX_ROWS));
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
    assert_eq!(rig.term.grid().scrollback_limit(), crate::terminal::screen::history::history_bytes(7, 12));
}

#[test]
fn a_restored_cursor_only_has_a_wrap_due_on_the_last_column() {
    // DECSC on the last column with the wrap due, then a wider screen: DECRC puts the cursor back
    // where it was, mid-row, and the next character goes there.
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[1;10Hz\x1b7");
    rig.term.resize(20, 3);
    rig.feed("\x1b8Y");
    assert_eq!(rig.row(0), "         Y");
    assert_eq!(rig.term.check_invariants(), Ok(()));
    // DECRSPS saying a wrap is due at a column that is not the last.
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1bP1$t1;1;1;@;@;H;0;2;@;BBBB\x1b\\AB");
    assert_eq!(rig.row(0), "AB");
    assert_eq!(rig.term.check_invariants(), Ok(()));
}

#[test]
fn a_tab_after_the_text_is_not_text_when_the_lines_rewrap() {
    // The tab leaves an invisible mark on the cell it started from; narrower, the line still
    // fits one row and the next line stays right under it.
    let mut rig = Rig::new(10, 3);
    rig.feed("ab\t\r\nc");
    rig.term.resize(2, 3);
    assert_eq!((rig.row(0), rig.row(1)), ("ab".to_string(), "c".to_string()));
    assert_eq!(rig.cursor(), (1, 1));
}

#[test]
fn a_decolm_request_is_one_for_the_host_and_resets_to_80_when_it_is_left() {
    // xterm ctlseqs mode 3 (DECCOLM) needs mode 40; the terminal asks the host for 132 columns.
    let mut rig = Rig::new(100, 5);
    rig.term.set_policy(Policy { window_ops: true, ..Policy::default() });
    rig.feed("\x1b[?40h\x1b[?3h");
    assert!(rig.events().contains(&Event::ColumnsRequested(132)));
    rig.feed("\x1b[?3l");
    assert!(rig.events().contains(&Event::ColumnsRequested(80)));
}
