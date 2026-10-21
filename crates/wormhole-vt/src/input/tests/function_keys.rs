//! Function keys: which numbers exist, and the kitty protocol's tilde codes for F6 to F12.

use super::{DIS, SHIFT, enc, key, modes, s, with};
use crate::input::Key;

#[test]
fn function_keys_exist_from_f1_to_f35() {
    assert_eq!(Key::function(0), None);
    assert_eq!(Key::function(1), Some(Key::F(1)));
    assert_eq!(Key::function(12), Some(Key::F(12)));
    assert_eq!(Key::function(35), Some(Key::F(35)));
    assert_eq!(Key::function(36), None);
}

#[test]
fn kitty_writes_f6_to_f12_as_the_vt220_tildes() {
    let m = modes(DIS);
    // Unmodified they keep their legacy bytes, so a modifier is what shows the protocol's own form.
    for (n, code) in [(6, 17), (7, 18), (8, 19), (9, 20), (10, 21), (11, 23), (12, 24)] {
        assert_eq!(enc(with(key(Key::F(n)), SHIFT), m), s(&format!("\x1b[{code};2~")), "F{n}");
    }
}
