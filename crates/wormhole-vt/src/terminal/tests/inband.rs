//! In-band resize notifications (DECSET 2048), against the gist that defines them
//! (<https://gist.github.com/rockorager/e695fb2924d36b2bcf1fff4a3704bd83>): a report when the
//! mode is set and after every resize, `CSI 48 ; rows ; cols ; height_px ; width_px t`.

use super::Rig;
use crate::types::Modes;

fn rig(cols: usize, rows: usize) -> Rig {
    let mut rig = Rig::new(cols, rows);
    rig.term.set_cell_size(8, 16);
    rig
}

#[test]
fn setting_the_mode_reports_the_size_at_once_even_when_it_was_set() {
    let mut rig = rig(80, 24);
    rig.feed("\x1b[?2048h");
    assert_eq!(rig.replies(), "\x1b[48;24;80;384;640t");
    rig.feed("\x1b[?2048h");
    assert_eq!(rig.replies(), "\x1b[48;24;80;384;640t", "enabling again reports again");
    rig.feed("\x1b[?2048l");
    assert_eq!(rig.replies(), "");
    assert!(!rig.term.modes().contains(Modes::INBAND_RESIZE));
}

#[test]
fn decrqm_answers_the_mode() {
    let mut rig = rig(80, 24);
    rig.feed("\x1b[?2048$p");
    assert_eq!(rig.replies(), "\x1b[?2048;2$y");
    rig.feed("\x1b[?2048h");
    let _ = rig.replies();
    rig.feed("\x1b[?2048$p");
    assert_eq!(rig.replies(), "\x1b[?2048;1$y");
}

#[test]
fn every_resize_is_reported_once_the_screen_has_its_new_size() {
    let mut rig = rig(80, 24);
    rig.feed("\x1b[?2048h");
    let _ = rig.replies();
    rig.term.resize(100, 30);
    assert_eq!(rig.replies(), "\x1b[48;30;100;480;800t");
    // The same size again is no resize.
    rig.term.resize(100, 30);
    assert_eq!(rig.replies(), "");
    rig.term.resize(100, 31);
    assert_eq!(rig.replies(), "\x1b[48;31;100;496;800t");
}

#[test]
fn a_cell_that_changed_is_reported_by_the_resize_that_follows_it_once() {
    let mut rig = rig(80, 24);
    rig.feed("\x1b[?2048h");
    let _ = rig.replies();
    // The host's order: the cell, then the size.
    rig.term.set_cell_size(10, 20);
    rig.term.resize(60, 20);
    assert_eq!(rig.replies(), "\x1b[48;20;60;400;600t");
    // The cell alone, with the size the screen has.
    rig.term.set_cell_size(9, 18);
    rig.term.resize(60, 20);
    assert_eq!(rig.replies(), "\x1b[48;20;60;360;540t");
}

#[test]
fn a_resize_without_the_mode_reports_nothing() {
    let mut rig = rig(80, 24);
    rig.term.resize(100, 30);
    assert_eq!(rig.replies(), "");
    rig.feed("\x1b[?2048h\x1b[?2048l");
    let _ = rig.replies();
    rig.term.resize(90, 30);
    assert_eq!(rig.replies(), "");
}

#[test]
fn a_full_reset_turns_it_off() {
    let mut rig = rig(80, 24);
    rig.feed("\x1b[?2048h\x1bc");
    let _ = rig.replies();
    rig.term.resize(90, 30);
    assert_eq!(rig.replies(), "");
}

#[test]
fn deccolm_reports_its_new_width() {
    let mut rig = rig(80, 24);
    rig.feed("\x1b[?40h\x1b[?2048h");
    let _ = rig.replies();
    rig.feed("\x1b[?3h");
    assert_eq!(rig.replies(), "\x1b[48;24;132;384;1056t");
}

#[test]
fn a_host_that_gave_no_cell_size_gets_the_engines_default_cell() {
    // The host never said: the engine's default cell, 8 by 16 (as XTWINOPS 16 answers).
    let mut rig = Rig::new(10, 4);
    rig.feed("\x1b[?2048h");
    assert_eq!(rig.replies(), "\x1b[48;4;10;64;80t");
}
