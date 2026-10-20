//! xterm's modifyOtherKeys: ordinary keys with a modifier as escape sequences.
//!
//! Spec: xterm's ctlseqs ("Alt and Meta Keys": `CSI > 4 ; Pv m` sets the level, and the sequence
//! sent is `CSI 27 ; modifiers ; code ~`, xterm's `formatOtherKeys` 0), and the table of what xterm
//! sends for each key and modifier at each level
//! (<https://invisible-island.net/xterm/modified-keys-us-pc105.html>), which the rules below follow:
//!
//! - **Level 1**: "the usual shift- and control- modifiers work as expected" — a character with
//!   Shift alone is the character the layout typed, and Control on a key that makes a control code
//!   (the letters, Space, `@ [ \ ] ^ _ /`, the digits 2 to 8, `~`) is that code. Everything else
//!   that has a modifier is the escape sequence: Alt or Meta on any key, Control on the rest of the
//!   characters (`! ? - 0 1 9`, …), and Enter and Tab with any modifier (Shift+Tab stays `CSI Z`);
//!   Escape only with Alt or Meta. Backspace stays as it was.
//! - **Level 2**: "all of the modifiers apply": every key with a modifier is the escape sequence,
//!   Shift+Tab (`CSI 27 ; 2 ; 9 ~`) and Control+Escape included; xterm's table shows Control+Backspace
//!   still sending BS, and the digits keeping the symbol Shift types on them (Shift+1 is `!`).
//! - The modifier parameter is `1 + shift + 2·alt + 4·ctrl + 8·meta`, as for the function keys, and
//!   the code is the character the key types: with Shift, the shifted one (`A` is 65, `{` is 123).
//! - Level 3 (unmodified keys as sequences) and the modifier mask of `CSI > 4 : Pm m` are not
//!   implemented.
//!
//! Cursor, function and keypad keys are not "other" keys: modifyOtherKeys leaves them alone.

use super::key::{Key, KeyEvent, Mods};
use super::legacy::control_byte;
use super::modes::{InputModes, ModifyOtherKeys};
use super::{ESC, push_dec};

/// Appends the `CSI 27` form of `ev` when the level says this key with these modifiers gets one;
/// `false` leaves the key to the legacy encoding.
pub(super) fn encode(ev: &KeyEvent, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    let level = modes.modify_other_keys;
    if level == ModifyOtherKeys::Off {
        return false;
    }
    let m = ev.mods;
    let (shift, ctrl) = (m.contains(Mods::SHIFT), m.contains(Mods::CTRL));
    let alt = m.intersects(Mods::ALT | Mods::META);
    if !(shift || ctrl || alt) {
        return false;
    }
    let all = level == ModifyOtherKeys::Level2;
    let code = match ev.key {
        Key::Char(c) => {
            // The layout composed a character with Alt (German Option+L is `@`): that is what was
            // typed and Alt was used up making it, as in the legacy encoding.
            if alt && !ctrl && ev.text.is_some() {
                return false;
            }
            let typed = typed_char(ev, c);
            let wanted = if alt {
                true
            } else if ctrl {
                all || !makes_control_code(typed)
            } else {
                // Shift alone: level 2, but not on a digit, which keeps its symbol in xterm's table.
                all && !c.is_ascii_digit()
            };
            if !wanted {
                return false;
            }
            u32::from(typed)
        }
        Key::Enter => 13,
        // Level 1 leaves Shift+Tab as `CSI Z`, the legacy form.
        Key::Tab if all || alt || ctrl => 9,
        Key::Escape if all || alt => 27,
        // BS is the key's own byte (DEL unless DECBKM): Control alone keeps sending it.
        Key::Backspace if all && !(ctrl && !shift && !alt) => {
            if modes.backarrow_bs {
                8
            } else {
                127
            }
        }
        _ => return false,
    };
    let modcode = 1
        + u32::from(shift)
        + 2 * u32::from(m.contains(Mods::ALT))
        + 4 * u32::from(ctrl)
        + 8 * u32::from(m.contains(Mods::META));
    out.extend_from_slice(&[ESC, b'[', b'2', b'7', b';']);
    push_dec(out, modcode);
    out.push(b';');
    push_dec(out, code);
    out.push(b'~');
    true
}

/// The character the key types: with Shift the one the layout gave (or the upper case), else the
/// key itself.
fn typed_char(ev: &KeyEvent, c: char) -> char {
    if !ev.mods.contains(Mods::SHIFT) {
        return c;
    }
    ev.shifted
        .or_else(|| {
            let mut chars = ev.text?.chars();
            chars.next().filter(|_| chars.next().is_none())
        })
        .or_else(|| {
            let mut upper = c.to_uppercase();
            upper.next().filter(|_| upper.next().is_none())
        })
        .unwrap_or(c)
}

/// Whether Control on this character is one of the combinations that have always made a control
/// code (xterm's table keeps those as the code at level 1): not `-`, `?` and the digits 0, 1 and
/// 9, where the legacy bytes are a stand-in the table does not keep.
fn makes_control_code(c: char) -> bool {
    control_byte(c).is_some() && !matches!(c, '-' | '?' | '0' | '1' | '9')
}
