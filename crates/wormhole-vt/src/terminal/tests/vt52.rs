//! VT52 mode: the VT100 User Guide's table of VT52 sequences ("Valid VT52 Mode Control Sequences"),
//! xterm's ctlseqs ("VT52 Mode"), and what the mode takes away from ANSI mode.

use super::Rig;
use crate::types::Modes;

fn vt52(cols: usize, rows: usize) -> Rig {
    let mut rig = Rig::new(cols, rows);
    rig.feed("\x1b[?2l");
    rig
}

fn in_vt52(rig: &Rig) -> bool {
    rig.term.modes().contains(Modes::VT52)
}

#[test]
fn decanm_reset_enters_vt52_mode_and_esc_less_than_leaves_it() {
    let mut rig = Rig::new(10, 3);
    assert!(!in_vt52(&rig));
    rig.feed("\x1b[?2l");
    assert!(in_vt52(&rig));
    rig.feed("\x1b<");
    assert!(!in_vt52(&rig));
    // ANSI mode again: a CSI is a CSI.
    rig.feed("\x1b[2;3Hx");
    assert_eq!(rig.row(1), "  x");
}

#[test]
fn csi_is_not_read_in_vt52_mode() {
    let mut rig = vt52(10, 3);
    // ESC [ is the VT52's hold-screen request, and what follows is text.
    rig.feed("\x1b[2;3Hx");
    assert_eq!(rig.row(0), "2;3Hx");
    assert_eq!(rig.cursor(), (0, 5));
    // No string begins either.
    rig.feed("\x1b]0;t\x07");
    assert_eq!(rig.term.title(), None);
    assert_eq!(rig.row(0), "2;3Hx0;t");
}

#[test]
fn the_cursor_moves_and_stops_at_the_margins() {
    let mut rig = vt52(5, 3);
    rig.feed("\x1bB\x1bB\x1bB\x1bC\x1bC");
    assert_eq!(rig.cursor(), (2, 2), "down stops at the bottom, right moved twice");
    rig.feed("\x1bA\x1bA\x1bA\x1bD\x1bD\x1bD");
    assert_eq!(rig.cursor(), (0, 0), "up and left stop at the top left");
    rig.feed("\x1bC\x1bC\x1bC\x1bC\x1bC\x1bC");
    assert_eq!(rig.cursor(), (0, 4), "right stops at the right margin");
}

#[test]
fn home_erase_and_reverse_line_feed() {
    let mut rig = vt52(5, 3);
    rig.feed("aaaaa\r\nbbbbb\r\nccccc\x1bH\x1bC\x1bC\x1bK");
    assert_eq!((rig.row(0), rig.row(1)), ("aa".into(), "bbbbb".into()), "ESC K erases to the line's end");
    rig.feed("\x1bB\x1bJ");
    assert_eq!((rig.row(1), rig.row(2)), ("bb".into(), "".into()), "ESC J to the screen's end");
    // ESC I at the top scrolls down; elsewhere it moves up.
    rig.feed("\x1bH\x1bI");
    assert_eq!((rig.row(0), rig.row(1), rig.row(2)), ("".into(), "aa".into(), "bb".into()));
    rig.feed("\x1bB\x1bI");
    assert_eq!(rig.cursor(), (0, 0));
}

#[test]
fn direct_cursor_addressing_takes_two_characters_each_the_number_plus_32() {
    let mut rig = vt52(20, 6);
    // Row 3, column 7 (0-based): '#' is 35 and ''' is 39.
    rig.feed("\x1bY#'X");
    assert_eq!(rig.cursor(), (3, 8));
    assert_eq!(rig.row(3), "       X");
    // VT52 manual 2.4.6: ESC Y 7 SPACE is the first position of the last line of a 24-line screen.
    let mut rig = vt52(80, 24);
    rig.feed("\x1bY7 .\x1bY o,");
    assert_eq!(rig.row(23), ".");
    assert_eq!(rig.row(0).len(), 80);
    assert_eq!(rig.cursor(), (0, 80 - 1), "the comma stays on the last column: no wrap in VT52 mode");
}

#[test]
fn direct_addressing_survives_the_input_being_cut_anywhere() {
    let whole = {
        let mut rig = vt52(20, 6);
        rig.feed("\x1bY#'X");
        (rig.row(3), rig.cursor())
    };
    let bytes = b"\x1bY#'X";
    for cut in 1..bytes.len() {
        let mut rig = vt52(20, 6);
        rig.feed_bytes(&bytes[..cut]);
        rig.feed_bytes(&bytes[cut..]);
        assert_eq!((rig.row(3), rig.cursor()), whole, "cut at {cut}");
    }
    let mut rig = vt52(20, 6);
    for byte in bytes {
        rig.feed_bytes(&[*byte]);
    }
    assert_eq!((rig.row(3), rig.cursor()), whole, "byte by byte");
}

#[test]
fn an_address_past_the_screen_leaves_that_axis_alone() {
    let mut rig = vt52(10, 4);
    rig.feed("\x1bY\x21\x21");
    assert_eq!(rig.cursor(), (1, 1));
    // The row is out of range: only the column moves (vttest draws a box with such addresses).
    rig.feed("\x1bY\x7e\x25");
    assert_eq!(rig.cursor(), (1, 5));
    rig.feed("\x1bY\x23\x7e");
    assert_eq!(rig.cursor(), (3, 5));
}

#[test]
fn an_address_as_far_as_the_screen_is_wide_or_tall_is_past_it() {
    let mut rig = vt52(10, 4);
    rig.feed("\x1bY\x21\x21");
    assert_eq!(rig.cursor(), (1, 1));
    // Row 4 on a screen of rows 0 to 3 and column 10 on one of columns 0 to 9: each is out.
    rig.feed("\x1bY\x24\x23");
    assert_eq!(rig.cursor(), (1, 3), "the row was past the last, the column 3");
    rig.feed("\x1bY\x23\x2a");
    assert_eq!(rig.cursor(), (3, 3), "the row 3, the column was past the last");
    // The last row and column themselves are in.
    rig.feed("\x1bY\x23\x29");
    assert_eq!(rig.cursor(), (3, 9));
}

#[test]
fn a_character_beyond_ascii_is_a_coordinate_past_the_screen() {
    let mut rig = vt52(10, 4);
    // The row is out of range, so the cursor stays on its row; the column is 3.
    rig.feed("\x1bY\u{e9}#X");
    assert_eq!((rig.row(0), rig.cursor()), ("   X".into(), (0, 4)));
    // And as the column it keeps the column.
    rig.feed("\x1bY\x22\u{e9}Y");
    assert_eq!((rig.row(2), rig.cursor()), ("    Y".into(), (2, 5)));
}

#[test]
fn controls_and_an_escape_act_inside_a_direct_address() {
    let mut rig = vt52(10, 4);
    rig.feed("\x1bY\n\x20\x20X");
    // The LF moved down (and was no coordinate); the two blanks were the row and the column.
    assert_eq!((rig.row(0), rig.cursor()), ("X".into(), (0, 1)));
    // An ESC cancels it: the character after the new sequence is text.
    let mut rig = vt52(10, 4);
    rig.feed("\x1bY#\x1bHZ");
    assert_eq!(rig.row(0), "Z");
}

#[test]
fn identify_answers_a_vt52_emulated_by_a_vt100() {
    let mut rig = vt52(10, 3);
    rig.feed("\x1bZ");
    assert_eq!(rig.replies(), "\x1b/Z");
    // In ANSI mode ESC Z is DECID and answers as DA1.
    rig.feed("\x1b<\x1bZ");
    assert!(rig.replies().starts_with("\x1b[?"));
}

#[test]
fn graphics_mode_uses_the_special_graphics_set_until_it_is_left() {
    let mut rig = vt52(10, 3);
    rig.feed("\x1bFqx\x1bGqx");
    assert_eq!(rig.row(0), "\u{2500}\u{2502}qx");
}

#[test]
fn the_keypad_modes_are_set_and_reset() {
    let mut rig = vt52(10, 3);
    rig.feed("\x1b=");
    assert!(rig.term.modes().contains(Modes::APP_KEYPAD));
    rig.feed("\x1b>");
    assert!(!rig.term.modes().contains(Modes::APP_KEYPAD));
}

#[test]
fn the_printer_and_other_ansi_escapes_are_ignored() {
    let mut rig = vt52(10, 3);
    rig.feed("\x1b]\x1bV\x1bW\x1bX\x1b^\x1b_\x1b7\x1b8\x1bD\x1bM\x1bEab");
    assert_eq!(rig.row(0), "ab", "nothing moved, nothing printed");
    assert_eq!(rig.cursor(), (0, 2));
}

#[test]
fn there_is_no_wraparound_and_the_last_column_is_overwritten() {
    let mut rig = vt52(5, 3);
    rig.feed("abcdefg");
    assert_eq!(rig.row(0), "abcdg");
    assert_eq!(rig.row(1), "");
    assert_eq!(rig.cursor(), (0, 4));
}

#[test]
fn ansi_modes_are_suspended_and_come_back() {
    let mut rig = Rig::new(5, 3);
    // Insert mode, new-line mode and origin mode in ANSI; DECCKM, which VT52 mode does not touch.
    rig.feed("\x1b[4h\x1b[20h\x1b[?6h\x1b(0\x1b[?1h");
    rig.feed("\x1b[?2l");
    let m = rig.term.modes();
    assert!(!m.intersects(Modes::INSERT | Modes::NEW_LINE | Modes::ORIGIN | Modes::AUTOWRAP));
    assert!(m.contains(Modes::APP_CURSOR));
    // The graphics set ANSI mode had is not in force: `q` is a `q`.
    rig.feed("q\x1b<");
    assert_eq!(rig.row(0), "q");
    let m = rig.term.modes();
    assert!(
        m.contains(Modes::INSERT | Modes::NEW_LINE | Modes::ORIGIN | Modes::AUTOWRAP | Modes::APP_CURSOR)
    );
    rig.feed("\x1b[1;1H\x1b[2Kq");
    assert_eq!(rig.row(0), "\u{2500}", "G0 is the graphics set again");
}

#[test]
fn modes_that_were_off_stay_off_when_vt52_mode_is_left() {
    let mut rig = Rig::new(5, 3);
    // Autowrap off, and insert, new line and origin mode as they start (off).
    rig.feed("\x1b[?7l\x1b[?2l\x1b<");
    let m = rig.term.modes();
    assert!(!m.intersects(Modes::INSERT | Modes::NEW_LINE | Modes::ORIGIN | Modes::AUTOWRAP));
    rig.feed("abcdefg");
    assert_eq!(rig.row(0), "abcdg", "still no wrap");
}

#[test]
fn eight_bit_controls_are_not_controls_in_vt52_mode() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b G\x1b[?2l");
    rig.feed_bytes(&[0x9b, b'A']);
    // The byte is not a CSI: it is not a character either, and the A is text.
    assert_eq!(rig.row(0).trim_start_matches('\u{fffd}'), "A");
}

#[test]
fn leaving_takes_the_terminal_to_vt100_mode_with_seven_bit_controls() {
    let mut rig = Rig::new(10, 3);
    // 8-bit controls on, then VT52 mode in and out.
    rig.feed("\x1b G\x1b[?2l\x1b<");
    rig.feed("\x1b[c");
    assert_eq!(rig.replies(), "\x1b[?1;2c", "the VT100's answer, in 7-bit form");
    rig.feed("\x1b G\x1b[6n");
    assert_eq!(rig.replies(), "\x1b[1;1R", "S8C1T is a VT200 function");
}

#[test]
fn a_full_reset_leaves_vt52_mode() {
    let mut rig = vt52(10, 3);
    rig.feed("\x1bFa\x1bc");
    assert!(!in_vt52(&rig));
    rig.feed("\x1b[1;1Hq");
    assert_eq!(rig.row(0), "q", "the graphics set did not survive");
    // And so does the host's.
    let mut rig = vt52(10, 3);
    rig.term.reset();
    assert!(!in_vt52(&rig));
}

#[test]
fn leaving_vt52_mode_twice_or_entering_it_twice_is_harmless() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b<\x1b<");
    assert!(!in_vt52(&rig));
    rig.feed("\x1b[?4h\x1b[?2l\x1b[?2l");
    assert!(in_vt52(&rig));
    rig.feed("\x1b<");
    assert!(rig.term.modes().contains(Modes::SMOOTH_SCROLL), "a mode VT52 mode leaves alone");
}

#[test]
fn text_and_controls_work_as_before() {
    let mut rig = vt52(10, 3);
    rig.feed("ab\x08c\tx\r\nhi");
    assert_eq!(rig.row(0), "ac      x");
    assert_eq!(rig.row(1), "hi");
}

#[test]
fn a_repaint_of_a_terminal_in_vt52_mode_makes_the_same_terminal() {
    let mut a = Rig::new(10, 4);
    a.feed("\x1b[4h\x1b[?6h\x1b[1;31mred\x1b[?2l\x1bFq\x1bY\"$x");
    assert!(in_vt52(&a));
    let mut bytes = Vec::new();
    let repainted = a.term.repaint(&mut bytes).expect("ready");
    let mut b = Rig::new(10, 4);
    b.term.seed(&repainted.seeds);
    b.feed_bytes(&bytes);
    assert_eq!(a.term.dump_state().diff(&b.term.dump_state(), 4), "");
    // Both go on the same way.
    let tail = b"\x1bC\x1bGz\x1bH\x1b<\x1b[Hq";
    a.feed_bytes(tail);
    b.feed_bytes(tail);
    assert_eq!(a.term.dump_state().diff(&b.term.dump_state(), 4), "");
}

#[test]
fn a_repaint_waits_for_a_direct_address() {
    let mut rig = vt52(10, 4);
    rig.feed("\x1bY#");
    let mut bytes = Vec::new();
    assert!(rig.term.repaint(&mut bytes).is_err());
    rig.feed("'");
    assert!(rig.term.repaint(&mut bytes).is_ok());
}
