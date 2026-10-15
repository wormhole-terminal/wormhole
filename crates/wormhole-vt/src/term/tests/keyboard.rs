//! The kitty keyboard protocol's flags, per screen.

use super::Rig;
use crate::types::TermMode;

#[test]
fn flags_are_pushed_popped_set_and_asked_for() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?0u");
    rig.feed("\x1b[>1u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?1u");
    assert!(rig.term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));
    rig.feed("\x1b[>9u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?9u");
    assert!(rig.term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC));
    // Set: replace (1), add (2), remove (3).
    rig.feed("\x1b[=4;1u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?4u");
    rig.feed("\x1b[=1;2u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?5u");
    rig.feed("\x1b[=4;3u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?1u");
    // Pop two: back to the base entry, which is 0.
    rig.feed("\x1b[<2u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?0u");
    assert!(!rig.term.mode().intersects(TermMode::DISAMBIGUATE_ESC_CODES | TermMode::REPORT_ALL_KEYS_AS_ESC));
}

#[test]
fn each_screen_has_its_own_stack() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[>1u\x1b[?1049h\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?0u", "the alternate screen starts with nothing");
    rig.feed("\x1b[>8u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?8u");
    rig.feed("\x1b[?1049l\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?1u", "and the main screen has what it had");
    assert!(rig.term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));
}

#[test]
fn unknown_bits_and_modes_are_ignored_and_a_reset_clears_the_stack() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[>255u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?31u");
    rig.feed("\x1b[=1;9u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?31u", "mode 9 is not one");
    rig.feed("\x1bc\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?0u");
}

#[test]
fn mode_bits_for_the_app_follow_the_dec_modes() {
    let mut rig = Rig::new(20, 5);
    let mode = |rig: &Rig| rig.term.mode();
    assert!(mode(&rig).contains(TermMode::SHOW_CURSOR | TermMode::LINE_WRAP | TermMode::ALTERNATE_SCROLL));
    rig.feed("\x1b[?1h\x1b[?1000h\x1b[?1006h\x1b[?2004h\x1b[?1004h\x1b[?25l");
    assert!(mode(&rig).contains(
        TermMode::APP_CURSOR
            | TermMode::MOUSE_REPORT_CLICK
            | TermMode::SGR_MOUSE
            | TermMode::BRACKETED_PASTE
            | TermMode::FOCUS_IN_OUT
    ));
    assert!(!mode(&rig).contains(TermMode::SHOW_CURSOR));
    rig.feed("\x1b[?1002h\x1b[?1003h\x1b[?1005h\x1b[?7l\x1b[?1049h");
    assert!(mode(&rig).contains(
        TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION | TermMode::UTF8_MOUSE | TermMode::ALT_SCREEN
    ));
    assert!(!mode(&rig).contains(TermMode::LINE_WRAP));
}
