//! The keypad's navigation keys (NumLock off): the same bytes as the cursor block's keys.

use super::{CTRL, NONE, enc, key, s, with};
use crate::input::{InputModes, Key, KeypadKey};

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
