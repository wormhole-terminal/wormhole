//! The keys of VT52 mode: xterm's ctlseqs ("VT52-Style Function Keys") and the VT100 User Guide
//! (Table 3-7, VT52 mode auxiliary keypad codes).

use super::{ALT, CTRL, DIS, NONE, SHIFT, enc, key, s, typed, with};
use crate::input::{
    InputModes, Key, KeypadKey, ModifyOtherKeys, MouseButton, MouseEvent, MouseKind, MouseTracking,
    encode_mouse, focus, paste,
};

fn vt52() -> InputModes {
    InputModes { vt52: true, ..NONE }
}

#[test]
fn cursor_keys_have_no_bracket_and_ignore_decckm() {
    for m in [vt52(), InputModes { app_cursor: true, ..vt52() }] {
        assert_eq!(enc(key(Key::Up), m), s("\x1bA"));
        assert_eq!(enc(key(Key::Down), m), s("\x1bB"));
        assert_eq!(enc(key(Key::Right), m), s("\x1bC"));
        assert_eq!(enc(key(Key::Left), m), s("\x1bD"));
        // No modifier forms.
        assert_eq!(enc(with(key(Key::Up), CTRL), m), s("\x1bA"));
    }
}

#[test]
fn function_keys_one_to_four_are_the_pf_keys() {
    let m = vt52();
    assert_eq!(enc(key(Key::F(1)), m), s("\x1bP"));
    assert_eq!(enc(key(Key::F(2)), m), s("\x1bQ"));
    assert_eq!(enc(key(Key::F(3)), m), s("\x1bR"));
    assert_eq!(enc(key(Key::F(4)), m), s("\x1bS"));
    assert_eq!(enc(key(Key::F(5)), m), None);
}

#[test]
fn the_keypad_sends_its_characters_or_esc_question_mark_in_application_mode() {
    let numeric = vt52();
    let application = InputModes { app_keypad: true, ..vt52() };
    for (pad, typed, app) in [
        (KeypadKey::Digit(0), "0", "\x1b?p"),
        (KeypadKey::Digit(5), "5", "\x1b?u"),
        (KeypadKey::Digit(9), "9", "\x1b?y"),
        (KeypadKey::Decimal, ".", "\x1b?n"),
        (KeypadKey::Enter, "\r", "\x1b?M"),
        (KeypadKey::Add, "+", "\x1b?k"),
        (KeypadKey::Subtract, "-", "\x1b?m"),
        (KeypadKey::Multiply, "*", "\x1b?j"),
        (KeypadKey::Divide, "/", "\x1b?o"),
        (KeypadKey::Separator, ",", "\x1b?l"),
        (KeypadKey::Equal, "=", "\x1b?X"),
    ] {
        assert_eq!(enc(key(Key::Keypad(pad)), numeric), s(typed), "{pad:?}");
        assert_eq!(enc(key(Key::Keypad(pad)), application), s(app), "{pad:?}");
    }
    assert_eq!(enc(key(Key::Keypad(KeypadKey::Left)), application), s("\x1bD"));
    assert_eq!(enc(key(Key::Keypad(KeypadKey::Right)), application), s("\x1bC"));
    assert_eq!(enc(key(Key::Keypad(KeypadKey::Up)), application), s("\x1bA"));
    assert_eq!(enc(key(Key::Keypad(KeypadKey::Down)), application), s("\x1bB"));
}

#[test]
fn characters_are_as_ever_and_nothing_needs_a_csi() {
    let m = InputModes { modify_other_keys: ModifyOtherKeys::Level2, ..vt52() };
    assert_eq!(enc(typed('a', "a"), m), s("a"));
    assert_eq!(enc(with(key(Key::Char('a')), CTRL), m), s("\u{1}"));
    assert_eq!(enc(with(key(Key::Char('a')), ALT), m), s("\x1ba"));
    assert_eq!(enc(key(Key::Enter), m), s("\r"));
    assert_eq!(enc(with(key(Key::Tab), SHIFT), m), s("\t"));
    // Only Tab loses its Shift: on a character it is what makes the capital.
    assert_eq!(enc(with(key(Key::Char('a')), SHIFT), m), s("A"));
    assert_eq!(enc(key(Key::Escape), m), s("\x1b"));
    for gone in [Key::Home, Key::End, Key::Insert, Key::Delete, Key::PageUp, Key::PageDown] {
        assert_eq!(enc(key(gone), m), None, "{gone:?}");
    }
    // The kitty flags the program pushed before entering VT52 mode do not apply in it.
    let m = InputModes { kitty: DIS, ..vt52() };
    assert_eq!(enc(key(Key::Up), m), s("\x1bA"));
}

#[test]
fn there_is_no_mouse_focus_or_paste_markers() {
    let m = InputModes { mouse: MouseTracking::Click, focus_events: true, bracketed_paste: true, ..vt52() };
    let mut out = Vec::new();
    let press = MouseEvent::new(MouseButton::Left, MouseKind::Press, 1, 1);
    assert!(!encode_mouse(&press, &m, &mut out));
    assert!(!focus(true, &m, &mut out));
    paste("ls", &m, &mut out);
    assert_eq!(out, b"ls");
}
