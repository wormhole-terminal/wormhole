//! The legacy xterm key encoding, used while no kitty keyboard flag is set.
//!
//! Sources: xterm's "PC-Style Function Keys" and "Alt and Meta Keys" in ctlseqs
//! (<https://invisible-island.net/xterm/ctlseqs/ctlseqs.html>). Modifiers go in the second
//! parameter as `1 + shift + 2·alt + 4·ctrl + 8·meta`.

use super::key::{Key, KeyEvent, KeyKind, KeypadKey, Mods};
use super::modes::InputModes;
use super::{ESC, other_keys, push_dec, push_typed};

/// Appends the bytes for `ev` and says whether there were any.
pub(super) fn encode(ev: &KeyEvent, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    // The legacy encoding has no key-up; a repeat is a press.
    if ev.kind == KeyKind::Release {
        return false;
    }
    if other_keys::encode(ev, modes, out) {
        return true;
    }
    let m = ev.mods;
    let (shift, alt, ctrl) = (m.contains(Mods::SHIFT), m.contains(Mods::ALT), m.contains(Mods::CTRL));
    let modcode =
        1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl) + 8 * u8::from(m.contains(Mods::META));

    match ev.key {
        Key::Char(c) => char_key(ev, c, out),
        // Alt+Enter is ESC CR, which readline binds and the AI CLIs take as "newline".
        Key::Enter => {
            if alt {
                out.push(ESC);
            }
            out.push(b'\r');
            true
        }
        // DEL, and BS with Ctrl; DECBKM swaps them (xterm's `backarrowKey`). Alt sends ESC and
        // DEL (BS with DECBKM) whatever else is held.
        Key::Backspace => {
            if alt {
                out.extend_from_slice(&[ESC, if modes.backarrow_bs { 0x08 } else { 0x7f }]);
            } else {
                out.push(if ctrl != modes.backarrow_bs { 0x08 } else { 0x7f });
            }
            true
        }
        // Shift+Tab is CSI Z (back-tab, "kcbt").
        Key::Tab => {
            if shift {
                out.extend_from_slice(b"\x1b[Z");
            } else {
                out.push(b'\t');
            }
            true
        }
        Key::Escape => {
            out.push(ESC);
            true
        }
        Key::Up => cursor(b'A', modcode, modes.app_cursor, out),
        Key::Down => cursor(b'B', modcode, modes.app_cursor, out),
        Key::Right => cursor(b'C', modcode, modes.app_cursor, out),
        Key::Left => cursor(b'D', modcode, modes.app_cursor, out),
        Key::Home => cursor(b'H', modcode, modes.app_cursor, out),
        Key::End => cursor(b'F', modcode, modes.app_cursor, out),
        Key::Insert => tilde(2, modcode, out),
        Key::Delete => tilde(3, modcode, out),
        Key::PageUp => tilde(5, modcode, out),
        Key::PageDown => tilde(6, modcode, out),
        Key::F(n) => function(n, modcode, out),
        Key::Keypad(key) => keypad(ev, key, modes, out),
        // Keys that never produced a byte on a PC keyboard in xterm, and keys xterm leaves to the
        // window system (media, lock keys, a modifier alone).
        _ => false,
    }
}

/// `CSI final`, `ESC O final` (application cursor keys) or `CSI 1 ; mod final`.
fn cursor(final_byte: u8, modcode: u8, app_cursor: bool, out: &mut Vec<u8>) -> bool {
    if modcode > 1 {
        out.extend_from_slice(b"\x1b[1;");
        push_dec(out, modcode.into());
        out.push(final_byte);
    } else {
        out.extend_from_slice(if app_cursor { b"\x1bO" } else { b"\x1b[" });
        out.push(final_byte);
    }
    true
}

/// `CSI n ~` or `CSI n ; mod ~`.
fn tilde(n: u32, modcode: u8, out: &mut Vec<u8>) -> bool {
    out.extend_from_slice(b"\x1b[");
    push_dec(out, n);
    if modcode > 1 {
        out.push(b';');
        push_dec(out, modcode.into());
    }
    out.push(b'~');
    true
}

/// F1 to F4 are `SS3 P` to `SS3 S` (`CSI 1 ; mod P` with modifiers), F5 to F12 the VT220 tildes.
/// F13 and up have no xterm form that every program agrees on; the kitty protocol has them.
fn function(n: u8, modcode: u8, out: &mut Vec<u8>) -> bool {
    match n {
        1..=4 => {
            let final_byte = b'P' + (n - 1);
            if modcode > 1 {
                out.extend_from_slice(b"\x1b[1;");
                push_dec(out, modcode.into());
            } else {
                out.extend_from_slice(b"\x1bO");
            }
            out.push(final_byte);
            true
        }
        5 => tilde(15, modcode, out),
        6..=10 => tilde(u32::from(n) + 11, modcode, out),
        11 | 12 => tilde(u32::from(n) + 12, modcode, out),
        _ => false,
    }
}

/// A key that types a character. Control makes a C0 code, Alt (or Meta) prefixes ESC.
fn char_key(ev: &KeyEvent, c: char, out: &mut Vec<u8>) -> bool {
    let m = ev.mods;
    let esc = m.intersects(Mods::ALT | Mods::META);
    let shift = m.contains(Mods::SHIFT);
    if m.contains(Mods::CTRL) {
        if let Some(byte) = control_byte(c) {
            if esc {
                out.push(ESC);
            }
            out.push(byte);
            return true;
        }
        // A control combination with no C0 code types whatever the host says it typed.
        return ev.text.is_some_and(|text| push_str(out, text));
    }
    match ev.text {
        // The layout composed a character (German Option+L is `@`): that is what was typed, and
        // Alt was used up making it.
        Some(text) => push_str(out, text),
        // Alt (or Option as Meta) with no composed text: ESC and the key.
        None if esc => {
            out.push(ESC);
            push_typed(out, c, shift);
            true
        }
        None => {
            push_typed(out, c, shift);
            true
        }
    }
}

fn push_str(out: &mut Vec<u8>, text: &str) -> bool {
    out.extend_from_slice(text.as_bytes());
    !text.is_empty()
}

/// The C0 code Control+`c` makes, as xterm, kitty and iTerm2 send it.
pub(super) fn control_byte(c: char) -> Option<u8> {
    Some(match c.to_ascii_lowercase() {
        c @ 'a'..='z' => c as u8 & 0x1f,
        '@' | '2' | ' ' => 0,
        '[' | '3' => 27,
        '\\' | '4' => 28,
        ']' | '5' => 29,
        '^' | '6' | '~' => 30,
        '_' | '-' | '7' | '/' => 31,
        '?' | '8' => 127,
        // Control does not change these digits.
        c @ ('0' | '1' | '9') => c as u8,
        _ => return None,
    })
}

/// The keypad: `SS3` codes in application mode (DECKPAM), the characters otherwise, and the
/// navigation keys as their cursor-block twins.
fn keypad(ev: &KeyEvent, key: KeypadKey, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    let twin = match key {
        KeypadKey::Left => Some(Key::Left),
        KeypadKey::Right => Some(Key::Right),
        KeypadKey::Up => Some(Key::Up),
        KeypadKey::Down => Some(Key::Down),
        KeypadKey::PageUp => Some(Key::PageUp),
        KeypadKey::PageDown => Some(Key::PageDown),
        KeypadKey::Home => Some(Key::Home),
        KeypadKey::End => Some(Key::End),
        KeypadKey::Insert => Some(Key::Insert),
        KeypadKey::Delete => Some(Key::Delete),
        _ => None,
    };
    if let Some(twin) = twin {
        return encode(&KeyEvent { key: twin, ..*ev }, modes, out);
    }
    if key == KeypadKey::Begin {
        out.extend_from_slice(b"\x1b[E");
        return true;
    }
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
    if modes.app_keypad && !ev.mods.intersects(Mods::SHIFT | Mods::ALT | Mods::CTRL | Mods::META) {
        out.extend_from_slice(&[ESC, b'O', application]);
    } else {
        out.push(typed);
    }
    true
}
