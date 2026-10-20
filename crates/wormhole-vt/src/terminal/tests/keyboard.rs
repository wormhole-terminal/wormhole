//! The kitty keyboard protocol's flags, per screen.

use super::Rig;
use crate::input::{InputModes, Key, KeyEvent, MouseEncoding, MouseTracking, encode_key};
use crate::types::{KittyFlags, Modes};

#[test]
fn flags_are_pushed_popped_set_and_asked_for() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?0u");
    rig.feed("\x1b[>1u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?1u");
    assert!(rig.term.modes().contains(Modes::KEYS_DISAMBIGUATE));
    rig.feed("\x1b[>9u\x1b[?u");
    assert_eq!(rig.replies(), "\x1b[?9u");
    assert!(rig.term.modes().contains(Modes::KEYS_ALL_AS_ESCAPES));
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
    assert!(!rig.term.modes().intersects(Modes::KEYS_DISAMBIGUATE | Modes::KEYS_ALL_AS_ESCAPES));
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
    assert!(rig.term.modes().contains(Modes::KEYS_DISAMBIGUATE));
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
    let mode = |rig: &Rig| rig.term.modes();
    assert!(mode(&rig).contains(Modes::SHOW_CURSOR | Modes::AUTOWRAP | Modes::ALTERNATE_SCROLL));
    rig.feed("\x1b[?1h\x1b[?1000h\x1b[?1006h\x1b[?2004h\x1b[?1004h\x1b[?25l");
    assert!(mode(&rig).contains(
        Modes::APP_CURSOR
            | Modes::MOUSE_CLICK
            | Modes::MOUSE_SGR
            | Modes::BRACKETED_PASTE
            | Modes::FOCUS_REPORT
    ));
    assert!(!mode(&rig).contains(Modes::SHOW_CURSOR));
    rig.feed("\x1b[?1002h\x1b[?1003h\x1b[?1005h\x1b[?7l\x1b[?1049h");
    assert!(
        mode(&rig).contains(Modes::MOUSE_DRAG | Modes::MOUSE_MOTION | Modes::MOUSE_UTF8 | Modes::ALT_SCREEN)
    );
    assert!(!mode(&rig).contains(Modes::AUTOWRAP));
}

#[test]
fn the_modes_a_host_could_not_reach_before_reach_the_encoders() {
    let mut rig = Rig::new(20, 5);
    let input = |rig: &Rig| InputModes::from(rig.term.modes());
    let backspace = |rig: &Rig| {
        let mut out = Vec::new();
        encode_key(&KeyEvent::new(Key::Backspace), &input(rig), &mut out);
        out
    };
    assert_eq!(input(&rig), InputModes::default());
    // ESC = and ESC >: the keypad; 9, 1015 and 1016: the older and the newer mouse forms.
    rig.feed("=");
    assert!(input(&rig).app_keypad);
    rig.feed(">");
    assert!(!input(&rig).app_keypad);
    rig.feed("[?9h");
    assert_eq!(rig.term.modes().mouse_tracking(), MouseTracking::X10);
    rig.feed("[?9l[?1000h[?1015h");
    assert_eq!(input(&rig).mouse_encoding, MouseEncoding::Urxvt);
    rig.feed("[?1016h");
    assert_eq!(input(&rig).mouse_encoding, MouseEncoding::SgrPixels);
    // The kitty flags in force.
    rig.feed("[>5u");
    assert_eq!(input(&rig).kitty, KittyFlags::DISAMBIGUATE | KittyFlags::REPORT_ALTERNATE_KEYS);
    // DECBKM (VT520): reset, the backarrow key sends DEL; set, BS. It starts reset, which is what
    // every terminal a shell runs in does (xterm-256color's terminfo has kbs=^?).
    rig.feed("c");
    assert_eq!(backspace(&rig), b"");
    rig.feed("[?67$p");
    assert_eq!(rig.replies(), "[?67;2$y");
    rig.feed("[?67h");
    assert_eq!(backspace(&rig), b"");
    rig.feed("[?67$p");
    assert_eq!(rig.replies(), "[?67;1$y");
    rig.feed("c");
    assert_eq!(backspace(&rig), b"", "RIS puts it back");
}

#[test]
fn the_locator_is_a_mode_while_a_program_has_it_on() {
    let mut rig = Rig::new(20, 5);
    assert!(!rig.term.modes().contains(Modes::LOCATOR));
    rig.feed("[1'z");
    assert!(rig.term.modes().contains(Modes::LOCATOR));
    rig.feed("[0'z");
    assert!(!rig.term.modes().contains(Modes::LOCATOR));
}
