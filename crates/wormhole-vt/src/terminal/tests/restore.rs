//! Putting back what a report said: DECRSTS (the terminal state), DECRSPS (the cursor information
//! and the tab stops) and DECAUPSS (the user-preferred set), and the reports that show the result.

use super::Rig;
use crate::types::Modes;

/// A terminal state report for a screen of 20 by 6 with these margins (`top;bottom;left;right`),
/// the insert mode and autowrap set, and a stop in no column: restoring it sets the insert mode.
fn state(margins: &str) -> String {
    format!("\x1bP1$p1;1;1;@;@;@;0;2;@;BBBB!!{margins}!4!7\x1b\\")
}

/// The cursor information report of a cursor at the corner with this flags character, in the form
/// DECRSPS reads and DECRQPSR answers.
fn cursor_report(flags: char, sizes: char, designators: &str) -> String {
    format!("1;1;1;@;@;{flags};0;2;{sizes};{designators}")
}

#[test]
fn a_terminal_state_report_with_margins_that_do_not_fit_the_screen_is_refused_whole() {
    let mut rig = Rig::new(20, 6);
    for margins in [
        "0;5;1;20", "5;3;1;20", "3;3;1;20", "2;7;1;20", "2;5;0;20", "2;5;7;7", "2;5;9;3", "2;5;1;21", "2;5;1",
    ] {
        rig.feed(&state(margins));
        assert!(!rig.term.modes().contains(Modes::INSERT), "{margins}: nothing was put back");
    }
    rig.feed(&state("2;5;1;20"));
    assert!(rig.term.modes().contains(Modes::INSERT), "and one that fits is");
    rig.feed("\x1bP$qr\x1b\\");
    assert_eq!(rig.replies(), "\x1bP1$r2;5r\x1b\\");
}

#[test]
fn a_cursor_report_read_back_puts_the_single_shift_it_had_back() {
    // The flags character's second bit is a single shift of G2 waiting, the third one of G3.
    let mut rig = Rig::new(20, 5);
    for flags in ['B', 'D', '@'] {
        let report = cursor_report(flags, '@', "BBBB");
        rig.feed(&format!("\x1bP1$t{report}\x1b\\\x1b[1$w"));
        assert_eq!(rig.replies(), format!("\x1bP1$u{report}\x1b\\"), "flags {flags}");
    }
    // A single shift that was waiting goes when the report has none.
    rig.feed("\x1bN");
    rig.feed(&format!("\x1bP1$t{}\x1b\\\x1b[1$w", cursor_report('@', '@', "BBBB")));
    assert_eq!(rig.replies(), format!("\x1bP1$u{}\x1b\\", cursor_report('@', '@', "BBBB")));
}

#[test]
fn the_user_preferred_set_can_be_chosen_back_and_designates_into_a_96_character_slot() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[&u");
    assert_eq!(rig.replies(), "\x1bP0!u%5\x1b\\", "DEC Supplemental, to begin with");
    rig.feed("\x1bP1!uA\x1b\\\x1b[&u");
    assert_eq!(rig.replies(), "\x1bP1!uA\x1b\\");
    rig.feed("\x1bP0!u%5\x1b\\\x1b[&u");
    assert_eq!(rig.replies(), "\x1bP0!u%5\x1b\\", "and back");
    // `ESC - <` and its kin put the preferred set in G1 to G3, whichever it is: here DEC
    // Supplemental into G1, then (96 characters, the third bit of the sizes) Latin-1 into G2.
    rig.feed("\x1b-<\x1b[1$w");
    assert_eq!(rig.replies(), format!("\x1bP1$u{}\x1b\\", cursor_report('@', '@', "B%5BB")));
    rig.feed("\x1bP1!uA\x1b\\\x1b.<\x1b[1$w");
    assert_eq!(rig.replies(), format!("\x1bP1$u{}\x1b\\", cursor_report('@', 'D', "B%5AB")));
}

#[test]
fn every_national_set_is_reported_by_the_final_characters_that_designate_it() {
    // DECRQPSR's designators: the sets (DECNRCM on) as DECRSPS reads them back.
    for (designate, reported) in [
        ("4", "4"),
        ("C", "5"),
        ("R", "R"),
        ("Q", "9"),
        ("K", "K"),
        ("Y", "Y"),
        ("`", "E"),
        ("%6", "%6"),
        ("Z", "Z"),
        ("H", "7"),
        ("=", "="),
    ] {
        let mut rig = Rig::new(20, 5);
        rig.feed(&format!("\x1b[?42h\x1b({designate}\x1b[1$w"));
        let want = format!("\x1bP1$u{}\x1b\\", cursor_report('@', '@', &format!("{reported}BBB")));
        assert_eq!(rig.replies(), want, "ESC ( {designate}");
    }
}

#[test]
fn a_tab_stop_report_as_long_as_a_very_wide_screen_needs_is_read_back() {
    // A stop in every one of 300 columns: 1,091 characters, more than a kilobyte, far less than the
    // most a report may be.
    let mut rig = Rig::new(300, 3);
    let stops: Vec<String> = (1..=300).map(|column| column.to_string()).collect();
    rig.feed(&format!("\x1bP2$t{}\x1b\\\r\tx", stops.join("/")));
    assert_eq!(rig.row(0), " x", "the next stop is the next column, not the eighth");
}
