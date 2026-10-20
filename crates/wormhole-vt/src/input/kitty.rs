//! The kitty keyboard protocol (<https://sw.kovidgoyal.net/kitty/keyboard-protocol/>), used as soon
//! as a program has pushed any of its flags.
//!
//! Every escape code has the form
//! `CSI unicode-key-code:shifted-key:base-layout-key ; modifiers:event-type ; text-as-codepoints u`
//! with the trailing fields left out when empty, and the final byte `~` or a letter for the keys
//! that had such a form before the protocol (their number or `1` takes the place of the key code).

use super::key::{Key, KeyEvent, KeyKind, KeypadKey, MediaKey, ModifierKey, Mods};
use super::legacy;
use super::modes::{InputModes, KittyFlags};
use super::{ESC, push_dec, push_typed};

/// How a key is written in the protocol's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Code {
    /// `CSI code u`
    U(u32),
    /// `CSI number ~`
    Tilde(u32),
    /// `CSI 1 ; mods letter`, or `CSI letter` without modifiers
    Letter(u8),
}

/// The protocol's table of functional keys. `None` for `Key::Char` and for numbers no key has.
fn code_of(key: Key) -> Option<Code> {
    use Code::{Letter, Tilde, U};
    Some(match key {
        Key::Char(_) => return None,
        Key::Escape => U(27),
        Key::Enter => U(13),
        Key::Tab => U(9),
        Key::Backspace => U(127),
        Key::Insert => Tilde(2),
        Key::Delete => Tilde(3),
        Key::PageUp => Tilde(5),
        Key::PageDown => Tilde(6),
        Key::Up => Letter(b'A'),
        Key::Down => Letter(b'B'),
        Key::Right => Letter(b'C'),
        Key::Left => Letter(b'D'),
        Key::Home => Letter(b'H'),
        Key::End => Letter(b'F'),
        Key::F(1) => Letter(b'P'),
        Key::F(2) => Letter(b'Q'),
        // F3 is `CSI 13 ~`: `CSI R` collides with cursor position reports.
        Key::F(3) => Tilde(13),
        Key::F(4) => Letter(b'S'),
        Key::F(5) => Tilde(15),
        Key::F(n @ 6..=10) => Tilde(u32::from(n) + 11),
        Key::F(n @ 11..=12) => Tilde(u32::from(n) + 12),
        // F13 to F35 live in the private use area from 57376.
        Key::F(n @ 13..=35) => U(57376 + u32::from(n) - 13),
        Key::F(_) => return None,
        Key::CapsLock => U(57358),
        Key::ScrollLock => U(57359),
        Key::NumLock => U(57360),
        Key::PrintScreen => U(57361),
        Key::Pause => U(57362),
        Key::Menu => U(57363),
        Key::Keypad(KeypadKey::Digit(d @ 0..=9)) => U(57399 + u32::from(d)),
        Key::Keypad(KeypadKey::Digit(_)) => return None,
        Key::Keypad(KeypadKey::Decimal) => U(57409),
        Key::Keypad(KeypadKey::Divide) => U(57410),
        Key::Keypad(KeypadKey::Multiply) => U(57411),
        Key::Keypad(KeypadKey::Subtract) => U(57412),
        Key::Keypad(KeypadKey::Add) => U(57413),
        Key::Keypad(KeypadKey::Enter) => U(57414),
        Key::Keypad(KeypadKey::Equal) => U(57415),
        Key::Keypad(KeypadKey::Separator) => U(57416),
        Key::Keypad(KeypadKey::Left) => U(57417),
        Key::Keypad(KeypadKey::Right) => U(57418),
        Key::Keypad(KeypadKey::Up) => U(57419),
        Key::Keypad(KeypadKey::Down) => U(57420),
        Key::Keypad(KeypadKey::PageUp) => U(57421),
        Key::Keypad(KeypadKey::PageDown) => U(57422),
        Key::Keypad(KeypadKey::Home) => U(57423),
        Key::Keypad(KeypadKey::End) => U(57424),
        Key::Keypad(KeypadKey::Insert) => U(57425),
        Key::Keypad(KeypadKey::Delete) => U(57426),
        Key::Keypad(KeypadKey::Begin) => Letter(b'E'),
        Key::Media(media) => U(match media {
            MediaKey::Play => 57428,
            MediaKey::Pause => 57429,
            MediaKey::PlayPause => 57430,
            MediaKey::Reverse => 57431,
            MediaKey::Stop => 57432,
            MediaKey::FastForward => 57433,
            MediaKey::Rewind => 57434,
            MediaKey::TrackNext => 57435,
            MediaKey::TrackPrevious => 57436,
            MediaKey::Record => 57437,
            MediaKey::LowerVolume => 57438,
            MediaKey::RaiseVolume => 57439,
            MediaKey::MuteVolume => 57440,
        }),
        Key::Modifier(modifier) => U(match modifier {
            ModifierKey::LeftShift => 57441,
            ModifierKey::LeftControl => 57442,
            ModifierKey::LeftAlt => 57443,
            ModifierKey::LeftSuper => 57444,
            ModifierKey::LeftHyper => 57445,
            ModifierKey::LeftMeta => 57446,
            ModifierKey::RightShift => 57447,
            ModifierKey::RightControl => 57448,
            ModifierKey::RightAlt => 57449,
            ModifierKey::RightSuper => 57450,
            ModifierKey::RightHyper => 57451,
            ModifierKey::RightMeta => 57452,
            ModifierKey::IsoLevel3Shift => 57453,
            ModifierKey::IsoLevel5Shift => 57454,
        }),
    })
}

/// Keys whose plain press keeps its legacy bytes under the protocol: Enter, Tab and Backspace
/// ("a shell stays usable after a crashed app") and the cursor block and F1 to F12, which were
/// unambiguous already. Escape is not one: it is the key the protocol exists to disambiguate.
fn keeps_legacy_form(key: Key) -> bool {
    matches!(
        key,
        Key::Enter
            | Key::Tab
            | Key::Backspace
            | Key::Insert
            | Key::Delete
            | Key::PageUp
            | Key::PageDown
            | Key::Up
            | Key::Down
            | Key::Left
            | Key::Right
            | Key::Home
            | Key::End
            | Key::F(1..=12)
    )
}

/// Text the protocol may carry: not empty, no C0, DEL or C1 control.
fn printable(text: &str) -> bool {
    !text.is_empty() && !text.chars().any(char::is_control)
}

/// Appends the bytes for `ev` and says whether there were any.
pub(super) fn encode(ev: &KeyEvent, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    let flags = modes.kitty;
    let report_all = flags.contains(KittyFlags::REPORT_ALL_KEYS);
    // Without flag 2 only presses exist, and a repeat is one.
    let kind = match ev.kind {
        KeyKind::Release if !flags.contains(KittyFlags::REPORT_EVENT_TYPES) => return false,
        KeyKind::Repeat if !flags.contains(KittyFlags::REPORT_EVENT_TYPES) => KeyKind::Press,
        kind => kind,
    };
    let mods = ev.mods;
    let real = mods.without_locks();
    // A key that types text is sent as that text, so it has no events of its own, unless every key
    // is reported (flag 8).
    let plain_text = !report_all && real.difference(Mods::SHIFT).is_empty();
    // Associated text never accompanies a control combination or a release.
    let carried = ev.text.filter(|t| printable(t) && !mods.contains(Mods::CTRL) && kind != KeyKind::Release);
    let text_field = carried.filter(|_| report_all && flags.contains(KittyFlags::REPORT_ASSOCIATED_TEXT));

    match ev.key {
        Key::Char(c) => {
            let base = c.to_lowercase().next().unwrap_or(c);
            if plain_text {
                if real.is_empty() {
                    if kind == KeyKind::Release {
                        return false;
                    }
                    match ev.text {
                        Some(text) => out.extend_from_slice(text.as_bytes()),
                        // A host with no layout text: the key types itself.
                        None => push_typed(out, c, false),
                    }
                    return true;
                }
                // Shift alone with typed text is that text; without it Shift is reported.
                if let Some(text) = ev.text.filter(|t| printable(t)) {
                    if kind == KeyKind::Release {
                        return false;
                    }
                    out.extend_from_slice(text.as_bytes());
                    return true;
                }
            }
            let (shifted, base_layout) = alternates(ev, base, flags);
            write_csi(out, Code::U(base as u32), (shifted, base_layout), mods, kind, text_field);
            true
        }
        Key::Keypad(_) if plain_text && real.is_empty() && ev.text.is_some_and(printable) => {
            if kind == KeyKind::Release {
                return false;
            }
            out.extend_from_slice(ev.text.map_or(&[][..], str::as_bytes));
            true
        }
        // A modifier key on its own is only reported when every key is.
        Key::Modifier(_) if !report_all => false,
        key => {
            let Some(code) = code_of(key) else { return false };
            if !report_all {
                let etb = matches!(key, Key::Enter | Key::Tab | Key::Backspace);
                if etb && kind == KeyKind::Release {
                    return false;
                }
                let plain_event = kind == KeyKind::Press || (etb && kind == KeyKind::Repeat);
                if keeps_legacy_form(key) && real.is_empty() && plain_event && legacy::encode(ev, modes, out)
                {
                    return true;
                }
            }
            let text = if matches!(key, Key::Keypad(_)) { text_field } else { None };
            write_csi(out, code, (None, None), mods, kind, text);
            true
        }
    }
}

/// The shifted and base-layout key codes of flag 4, when it is on and they differ from the key.
fn alternates(ev: &KeyEvent, base: char, flags: KittyFlags) -> (Option<char>, Option<char>) {
    if !flags.contains(KittyFlags::REPORT_ALTERNATE_KEYS) {
        return (None, None);
    }
    let shifted = if ev.mods.contains(Mods::SHIFT) {
        ev.shifted.or_else(|| base.to_uppercase().next()).filter(|s| *s != base)
    } else {
        None
    };
    (shifted, ev.base_layout.filter(|b| *b != base))
}

/// Writes one escape code.
fn write_csi(
    out: &mut Vec<u8>,
    code: Code,
    (shifted, base_layout): (Option<char>, Option<char>),
    mods: Mods,
    kind: KeyKind,
    text: Option<&str>,
) {
    let event_type = match kind {
        KeyKind::Press => 0,
        KeyKind::Repeat => 2,
        _ => 3,
    };
    // The modifier field is there when it says something, and when a later field needs its place.
    let modifier_field = !mods.is_empty() || event_type != 0 || text.is_some();
    out.extend_from_slice(&[ESC, b'[']);
    match code {
        Code::U(n) => {
            push_dec(out, n);
            if shifted.is_some() || base_layout.is_some() {
                out.push(b':');
                if let Some(shifted) = shifted {
                    push_dec(out, shifted as u32);
                }
                if let Some(base_layout) = base_layout {
                    out.push(b':');
                    push_dec(out, base_layout as u32);
                }
            }
        }
        Code::Tilde(n) => push_dec(out, n),
        Code::Letter(_) if modifier_field => out.push(b'1'),
        Code::Letter(_) => {}
    }
    if modifier_field {
        out.push(b';');
        push_dec(out, u32::from(mods.bits()) + 1);
        if event_type != 0 {
            out.push(b':');
            push_dec(out, event_type);
        }
    }
    if let Some(text) = text {
        out.push(b';');
        for (i, c) in text.chars().enumerate() {
            if i > 0 {
                out.push(b':');
            }
            push_dec(out, c as u32);
        }
    }
    out.push(match code {
        Code::U(_) => b'u',
        Code::Tilde(_) => b'~',
        Code::Letter(letter) => letter,
    });
}
