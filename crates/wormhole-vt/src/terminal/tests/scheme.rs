//! The host's light or dark scheme: what is asked with `CSI ? 996 n` and told with mode 2031.

use super::Rig;
use crate::terminal::Terminal;
use crate::types::ColorScheme;

#[test]
fn the_builder_can_tell_the_scheme_from_the_start() {
    let mut term = Terminal::builder().color_scheme(ColorScheme::Light).build();
    assert_eq!(term.color_scheme(), Some(ColorScheme::Light));
    term.feed(b"\x1b[?996n");
    assert_eq!(term.take_replies(), b"\x1b[?997;2n");
    assert_eq!(Terminal::new(10, 3).color_scheme(), None);
}

#[test]
fn an_unknown_scheme_is_never_guessed_even_when_asked_to_report() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[?2031h\x1b[?996n");
    assert_eq!(rig.replies(), "");
    // The first thing the host says is a change, and the program that asked to hear gets it.
    rig.term.set_color_scheme(ColorScheme::Dark);
    assert_eq!(rig.replies(), "\x1b[?997;1n");
}

#[test]
fn the_report_of_a_change_is_in_order_with_the_other_replies() {
    let mut rig = Rig::new(20, 5);
    rig.term.set_color_scheme(ColorScheme::Dark);
    rig.feed("\x1b[?2031h\x1b[5n");
    rig.term.set_color_scheme(ColorScheme::Light);
    rig.feed("\x1b[6n");
    assert_eq!(rig.replies(), "\x1b[0n\x1b[?997;2n\x1b[1;1R");
}

#[test]
fn a_reset_keeps_the_scheme_the_host_told_but_not_mode_2031() {
    let mut rig = Rig::new(20, 5);
    rig.term.set_color_scheme(ColorScheme::Light);
    rig.feed("\x1b[?2031h\x1bc");
    rig.term.set_color_scheme(ColorScheme::Dark);
    assert_eq!(rig.replies(), "");
    rig.feed("\x1b[?996n");
    assert_eq!(rig.replies(), "\x1b[?997;1n", "it is the host's, not the program's");
}

#[test]
fn the_answer_is_the_same_whatever_else_the_request_carries() {
    let mut rig = Rig::new(20, 5);
    rig.term.set_color_scheme(ColorScheme::Dark);
    rig.feed("\x1b[?996;1;2n\x1b[?996;5n");
    assert_eq!(rig.replies(), "\x1b[?997;1n\x1b[?997;1n");
}
