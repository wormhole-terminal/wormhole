//! The keypad's navigation keys (NumLock off): the same bytes as the cursor block's keys.

use super::{ALT, CTRL, NONE, SHIFT, enc, key, s, with};
use crate::input::{InputModes, Key, KeypadKey, Mods};

#[test]
fn the_keypad_navigation_keys_send_what_the_cursor_block_sends() {
    let app = InputModes { app_cursor: true, ..NONE };
    for (pad, twin, plain) in [
        (KeypadKey::Left, Key::Left, "\x1b[D"),
        (KeypadKey::Right, Key::Right, "\x1b[C"),
        (KeypadKey::Up, Key::Up, "\x1b[A"),
        (KeypadKey::Down, Key::Down, "\x1b[B"),
        (KeypadKey::PageUp, Key::PageUp, "\x1b[5~"),
        (KeypadKey::PageDown, Key::PageDown, "\x1b[6~"),
        (KeypadKey::Home, Key::Home, "\x1b[H"),
        (KeypadKey::End, Key::End, "\x1b[F"),
        (KeypadKey::Insert, Key::Insert, "\x1b[2~"),
        (KeypadKey::Delete, Key::Delete, "\x1b[3~"),
    ] {
        assert_eq!(enc(key(Key::Keypad(pad)), NONE), s(plain), "{pad:?}");
        assert_eq!(enc(key(Key::Keypad(pad)), app), enc(key(twin), app), "{pad:?} in application mode");
        let modified = with(key(Key::Keypad(pad)), CTRL);
        assert_eq!(enc(modified, NONE), enc(with(key(twin), CTRL), NONE), "{pad:?} with Control");
    }
}

#[test]
fn the_keypad_comma_is_a_comma_or_ss3_l() {
    let app = InputModes { app_keypad: true, ..NONE };
    let separator = key(Key::Keypad(KeypadKey::Separator));
    assert_eq!(enc(separator, NONE), s(","));
    assert_eq!(enc(separator, app), s("\x1bOl"));
}

#[test]
fn a_held_modifier_makes_the_application_keypad_type_its_character() {
    // DECKPAM's `ESC O x` has no modifier form, so each of the four modifiers on its own gives it up.
    let app = InputModes { app_keypad: true, ..NONE };
    let five = key(Key::Keypad(KeypadKey::Digit(5)));
    assert_eq!(enc(five, app), s("\x1bOu"));
    for (name, mods) in [("Shift", SHIFT), ("Alt", ALT), ("Control", CTRL), ("Meta", Mods::META)] {
        assert_eq!(enc(with(five, mods), app), s("5"), "{name}");
    }
    // Lock keys are not modifiers here.
    assert_eq!(enc(with(five, Mods::NUM_LOCK), app), s("\x1bOu"));
}
