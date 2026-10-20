//! The keys of a terminal in VT52 mode (DECANM reset).
//!
//! Spec: xterm's ctlseqs ("VT52-Style Function Keys") and the VT100 User Guide (Table 3-7, VT52 mode
//! auxiliary keypad codes). The cursor keys send `ESC A` to `ESC D` (no `[`, and DECCKM makes no
//! difference); the keypad sends its characters, or in application mode (`ESC =`) `ESC ? x`
//! with the same letters the ANSI keypad has after `ESC O`; F1 to F4 are the keypad's PF1 to PF4,
//! `ESC P` to `ESC S`. Everything else a legacy terminal sends is a plain character or control
//! code and is sent as it is. The keys that exist only as `CSI` sequences (Home, End, Insert,
//! Delete, Page Up and Down, F5 and up) send nothing: a VT52 program has no use for them, and what
//! it would read is garbage.
//!
//! The VT52 has no modifier forms: Shift, Control and Alt change the character a key types as
//! they always did and nothing else.

use super::ESC;
use super::key::{Key, KeyEvent, KeyKind, KeypadKey, Mods};
use super::legacy;
use super::modes::{InputModes, ModifyOtherKeys};

/// Appends the bytes for `ev` in VT52 mode and says whether there were any.
pub(super) fn encode(ev: &KeyEvent, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    if ev.kind == KeyKind::Release {
        return false;
    }
    match ev.key {
        Key::Up => escape(b'A', out),
        Key::Down => escape(b'B', out),
        Key::Right => escape(b'C', out),
        Key::Left => escape(b'D', out),
        Key::F(n @ 1..=4) => escape(b'P' + (n - 1), out),
        Key::Home
        | Key::End
        | Key::PageUp
        | Key::PageDown
        | Key::Insert
        | Key::Delete
        | Key::F(_)
        | Key::Media(_)
        | Key::Modifier(_) => false,
        Key::Keypad(key) => keypad(ev, key, modes, out),
        // A character, Enter, Tab, Backspace, Escape: the legacy bytes, without the `CSI` forms
        // (Shift+Tab is a plain tab) and without modifyOtherKeys.
        _ => {
            let plain = InputModes { vt52: false, modify_other_keys: ModifyOtherKeys::Off, ..*modes };
            let shifted_tab = ev.key == Key::Tab && ev.mods.contains(Mods::SHIFT);
            let ev = if shifted_tab { KeyEvent { mods: ev.mods - Mods::SHIFT, ..*ev } } else { *ev };
            legacy::encode(&ev, &plain, out)
        }
    }
}

fn escape(final_byte: u8, out: &mut Vec<u8>) -> bool {
    out.extend_from_slice(&[ESC, final_byte]);
    true
}

/// The keypad: `ESC ? x` in application mode, the characters otherwise; the navigation keys are
/// their cursor-block twins, where the VT52 has them.
fn keypad(ev: &KeyEvent, key: KeypadKey, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    let twin = match key {
        KeypadKey::Left => Key::Left,
        KeypadKey::Right => Key::Right,
        KeypadKey::Up => Key::Up,
        KeypadKey::Down => Key::Down,
        KeypadKey::PageUp
        | KeypadKey::PageDown
        | KeypadKey::Home
        | KeypadKey::End
        | KeypadKey::Insert
        | KeypadKey::Delete
        | KeypadKey::Begin => return false,
        _ => return keypad_character(key, modes, out),
    };
    encode(&KeyEvent { key: twin, ..*ev }, modes, out)
}

fn keypad_character(key: KeypadKey, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    // (application-mode final byte, character)
    let (application, typed) = match key {
        KeypadKey::Digit(d @ 0..=9) => (b'p' + d, b'0' + d),
        KeypadKey::Decimal => (b'n', b'.'),
        KeypadKey::Divide => (b'o', b'/'),
        KeypadKey::Multiply => (b'j', b'*'),
        KeypadKey::Subtract => (b'm', b'-'),
        KeypadKey::Add => (b'k', b'+'),
        KeypadKey::Enter => (b'M', b'\r'),
        KeypadKey::Equal => (b'X', b'='),
        KeypadKey::Separator => (b'l', b','),
        _ => return false,
    };
    if modes.app_keypad {
        out.extend_from_slice(&[ESC, b'?', application]);
    } else {
        out.push(typed);
    }
    true
}
