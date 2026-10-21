//! Which sequence is which: the final bytes, markers, intermediates, parameters and conformance
//! levels that make a sequence one function and not another that looks like it.

use super::Rig;
use crate::types::KittyFlags;

#[test]
fn a_clear_screen_keeps_the_history_unless_the_history_was_just_cleared() {
    // What a clear screen after the screen was filled leaves in the history: `ED 3` first means the
    // user wants it all gone, but only as long as nothing but a cursor move or an erase follows.
    let history_after = |between: &str| {
        let mut rig = Rig::new(10, 3);
        rig.feed("one\r\ntwo\x1b[3J");
        rig.feed(between);
        rig.feed("\x1b[2J");
        rig.term.history_len()
    };
    assert_eq!(history_after(""), 0);
    assert_eq!(history_after("\x1b[H"), 0);
    assert_eq!(history_after("\x1b[K"), 0);
    // Any other sequence ends that, and so does the private form of an erase.
    assert_eq!(history_after("\x1b[m"), 2, "SGR");
    assert_eq!(history_after("\x1b[?0J"), 2, "DECSED");
    assert_eq!(history_after("\x1b[?0K"), 2, "DECSEL");
    assert_eq!(history_after("\x1b[?25h"), 2, "a mode");
}

#[test]
fn scroll_down_with_more_parameters_is_xterms_mouse_tracking_and_ignored() {
    let mut rig = Rig::new(5, 3);
    rig.feed("a\r\nb\x1b[1;2;3;4;5T");
    assert_eq!((rig.row(0), rig.row(1)), ("a".into(), "b".into()));
    rig.feed("\x1b[T");
    assert_eq!((rig.row(0), rig.row(1), rig.row(2)), ("".into(), "a".into(), "b".into()));
}

#[test]
fn device_attributes_are_asked_for_with_parameter_zero_only() {
    let mut rig = Rig::new(5, 3);
    rig.feed("\x1b[1c\x1b[2c");
    assert_eq!(rig.replies(), "");
    rig.feed("\x1b[c");
    let plain = rig.replies();
    assert!(plain.starts_with("\x1b[?"), "{plain:?}");
    rig.feed("\x1b[0c");
    assert_eq!(rig.replies(), plain);
}

#[test]
fn csi_s_saves_the_cursor_only_without_parameters_and_csi_u_restores_it_without_one() {
    let mut rig = Rig::new(10, 5);
    // A parameter (even an empty one before another) makes it something else; nothing is saved.
    rig.feed("\x1b[3;4H\x1b[5s\x1b[;5s\x1b[1;1H\x1b[u");
    assert_eq!(rig.cursor(), (0, 0), "nothing was saved");
    rig.feed("\x1b[3;4H\x1b[s\x1b[1;1H\x1b[5u");
    assert_eq!(rig.cursor(), (0, 0), "CSI 5 u is not a restore");
    rig.feed("\x1b[u");
    assert_eq!(rig.cursor(), (2, 3));
}

#[test]
fn only_decst8c_with_parameter_five_resets_the_tab_stops() {
    let mut rig = Rig::new(20, 3);
    rig.feed("\x1b[3g\x1b[?1W\t");
    assert_eq!(rig.cursor(), (0, 19), "no stops are left to go to");
    rig.feed("\x1b[?5W\r\t");
    assert_eq!(rig.cursor(), (0, 8), "the stops every eight are back");
}

#[test]
fn scrolling_columns_is_a_vt400_function() {
    let mut rig = Rig::new(10, 2);
    rig.feed("\x1b[63;1\"pabcdef\x1b[2 @");
    assert_eq!(rig.row(0), "abcdef", "SL at VT300");
    rig.feed("\x1b[2 A");
    assert_eq!(rig.row(0), "abcdef", "SR at VT300");

    let mut rig = Rig::new(10, 2);
    rig.feed("abcdef\x1b[2 @");
    assert_eq!(rig.row(0), "cdef");
    rig.feed("\x1b[2 A");
    assert_eq!(rig.row(0), "  cdef");
}

#[test]
fn a_terminal_state_report_is_read_back_at_vt400_and_not_below() {
    // The report of a terminal with margins at rows 2 and 3 (see `replies`).
    let report = "\x1bP1$p2;3;1;C;@;@;0;2;@;BBBB!5/10!2;3;1;20!4;20!1;5;7\x1b\\";
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[63;1\"p");
    rig.feed(report);
    rig.feed("\x1bP$qr\x1b\\");
    assert_eq!(rig.replies(), "\x1bP1$r1;4r\x1b\\", "DECRSTS is not a VT300 function");
    let mut rig = Rig::new(20, 4);
    rig.feed(report);
    rig.feed("\x1bP$qr\x1b\\");
    assert_eq!(rig.replies(), "\x1bP1$r2;3r\x1b\\");
}

#[test]
fn decsca_is_a_quote_q_and_nothing_else_with_a_quote() {
    let selectively_erased = |select: &str| {
        let mut rig = Rig::new(10, 3);
        rig.feed(select);
        rig.feed("X\x1b[?2J");
        rig.row(0)
    };
    assert_eq!(selectively_erased(""), "", "not protected: DECSED takes it");
    assert_eq!(selectively_erased("\x1b[1\"q"), "X", "protected");
    assert_eq!(selectively_erased("\x1b[1\"q\x1b[0\"q"), "", "protected no more");
    // The same parameter with another final byte selects nothing.
    assert_eq!(selectively_erased("\x1b[1\"z"), "");
}

#[test]
fn only_the_kitty_pop_ends_a_kitty_flags_level() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[>1u");
    assert_eq!(rig.term.modes().kitty_flags(), KittyFlags::DISAMBIGUATE);
    rig.feed("\x1b[<1m\x1b[<1t");
    assert_eq!(rig.term.modes().kitty_flags(), KittyFlags::DISAMBIGUATE, "other finals pop nothing");
    rig.feed("\x1b[<u");
    assert_eq!(rig.term.modes().kitty_flags(), KittyFlags::empty());
}
