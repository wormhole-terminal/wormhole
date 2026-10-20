//! Input: what a host sends to the program for a key, a pointer event, a paste or a focus change.
//!
//! Every function is pure: bytes out are a function of the event and the [`InputModes`] the
//! program switched on, and they are appended to a `Vec<u8>` the caller owns and reuses, so
//! encoding allocates nothing once that buffer has grown. Nothing here knows a GUI toolkit, a PTY
//! or the screen; the host turns its own events into [`KeyEvent`] and [`MouseEvent`] once.
//!
//! - [`encode_key`]: legacy xterm encoding (cursor keys with DECCKM, the keypad with DECKPAM, F1 to
//!   F12, Control and Alt combinations), or the full
//!   [kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) (all five
//!   flags) as soon as the program has pushed one;
//! - [`encode_mouse`]: the X10, UTF-8, URXVT, SGR and SGR-pixels reports for the tracking level
//!   the program asked for;
//! - [`paste`]: bracketed paste, with what could escape the bracket removed;
//! - [`focus`]: focus in and out reports.
//!
//! ```
//! use wormhole_vt::input::{InputModes, Key, KeyEvent, Mods, encode_key};
//!
//! let mut out = Vec::new();
//! // Ctrl+C is the byte 0x03 ...
//! let ctrl_c = KeyEvent::new(Key::Char('c')).with_mods(Mods::CTRL);
//! assert!(encode_key(&ctrl_c, &InputModes::default(), &mut out));
//! assert_eq!(out, [0x03]);
//! // ... and a cursor key follows DECCKM.
//! out.clear();
//! let mut modes = InputModes::default();
//! modes.app_cursor = true;
//! encode_key(&KeyEvent::new(Key::Up), &modes, &mut out);
//! assert_eq!(out, b"\x1bOA");
//! ```

mod key;
mod kitty;
mod legacy;
mod modes;
mod mouse;
mod other_keys;
mod paste;
#[cfg(test)]
mod tests;

pub use key::{Key, KeyEvent, KeyKind, KeypadKey, MediaKey, ModifierKey, Mods};
pub use modes::{InputModes, KittyFlags, ModifyOtherKeys, MouseEncoding, MouseTracking};
pub use mouse::{MouseButton, MouseEvent, MouseKind};
pub use paste::{focus, paste};

const ESC: u8 = 0x1b;

/// Appends the bytes a program expects for a key event and says whether there were any.
///
/// With no kitty flag set in `modes` this is the legacy xterm encoding, and only presses and
/// repeats produce bytes. With any flag set it is the kitty keyboard protocol: keys that were
/// unambiguous before (an unmodified Enter, Tab, Backspace, cursor key or F1 to F12) keep their
/// legacy bytes unless the program asked for all keys as escape codes.
///
/// The host decides what the layout typed (`text`) and which modifiers count: see [`Mods`] for
/// "Option as Meta". Shortcuts that belong to the host (Cmd+C) are the host's to filter out before
/// it calls this.
pub fn encode_key(event: &KeyEvent, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    if modes.kitty.is_empty() { legacy::encode(event, modes, out) } else { kitty::encode(event, modes, out) }
}

/// Appends the report for a pointer event and says whether there was one: nothing is sent when the
/// program did not ask to be told about the event, or the encoding it chose cannot write it (an
/// X10 report past column 223, an extra button outside SGR).
pub fn encode_mouse(event: &MouseEvent, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    mouse::encode(event, modes, out)
}

/// Appends `n` in decimal; `Vec<u8>` formatting without going through `fmt`.
fn push_dec(out: &mut Vec<u8>, mut n: u32) {
    let mut digits = [0u8; 10];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[at..]);
}

/// Appends `c` as UTF-8.
fn push_char(out: &mut Vec<u8>, c: char) {
    out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
}

/// Appends `c`, upper-cased when `shift` is held: what a key types when the host gave no text.
fn push_typed(out: &mut Vec<u8>, c: char, shift: bool) {
    if shift {
        c.to_uppercase().for_each(|upper| push_char(out, upper));
    } else {
        push_char(out, c);
    }
}
