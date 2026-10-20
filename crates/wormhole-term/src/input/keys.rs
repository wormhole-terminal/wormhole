//! Keystroke -> bytes for the PTY.
//!
//! The encoders themselves (the legacy xterm encoding and the kitty keyboard protocol,
//! <https://sw.kovidgoyal.net/kitty/keyboard-protocol/>) are `wormhole_vt::input`. This is the
//! adapter: GPUI's key names and the app's settings become the engine's events, and the host
//! policy stays here (what Option, Shift+Enter and Option+arrows mean in Wormhole).
//!
//! Only key-*press* events are encoded: the host UI only delivers key-down events, so
//! `REPORT_EVENT_TYPES` has no visible effect (press is the default event type and is always
//! omitted from the sequence).

use wormhole_vt::input::{InputModes, Key, KeyEvent, KittyFlags, Mods, encode_key, focus};
use wormhole_vt::TermMode;

/// A GUI-independent key press.
///
/// `key` follows GPUI naming ("enter", "left", "f5", "a", "[" ...), `key_char`
/// is the text the key produces with the active layout and modifiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyInput<'a> {
    pub key: &'a str,
    pub key_char: Option<&'a str>,
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    pub platform: bool,
    /// Option is Meta (the `option_as_meta` setting): Option+key sends ESC and
    /// the key (Alt+key in kitty mode) instead of the character Option composes.
    pub option_as_meta: bool,
}

/// Encode a key press for the PTY. `None` means "nothing to send".
pub fn encode(input: &KeyInput, mode: TermMode) -> Option<Vec<u8>> {
    // Cmd shortcuts belong to the application.
    if input.platform {
        return None;
    }
    let modes = InputModes::from(mode);
    let Some(key) = key_of(input.key) else {
        // A name that is no key the engine knows: the legacy encoding sends what it typed.
        let text = input.key_char.filter(|text| !text.is_empty() && modes.kitty.is_empty());
        return text.map(|text| text.as_bytes().to_vec());
    };
    let event = event_of(input, key, &modes)?;
    let mut out = Vec::new();
    encode_key(&event, &modes, &mut out).then_some(out)
}

/// What to send when the terminal gains or loses focus: `CSI I` / `CSI O`
/// once the program asked for focus events (DECSET 1004; vim, tmux and TUIs
/// that pause when unfocused), otherwise nothing.
pub fn focus_report(focused: bool, mode: TermMode) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    focus(focused, &InputModes::from(mode), &mut out).then_some(out)
}

/// GPUI's name for a key as the engine's [`Key`]; `None` for a name that is no key.
fn key_of(name: &str) -> Option<Key> {
    Some(match name {
        "enter" => Key::Enter,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "escape" => Key::Escape,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "insert" => Key::Insert,
        "delete" => Key::Delete,
        // GPUI names the space bar.
        "space" => Key::Char(' '),
        _ => {
            let mut chars = name.chars();
            match (chars.next()?, chars.next()) {
                // A letter key is its own name; the engine's key is the unshifted one.
                (c, None) => Key::Char(c.to_lowercase().next().unwrap_or(c)),
                ('f', Some(_)) => return Key::function(name[1..].parse().ok()?),
                _ => return None,
            }
        }
    })
}

/// The engine's event for a press, with this app's rules about the modifiers applied. `None`: the
/// press types nothing.
fn event_of<'a>(input: &KeyInput<'a>, key: Key, modes: &InputModes) -> Option<KeyEvent<'a>> {
    let kitty = !modes.kitty.is_empty();
    let (shift, alt, control) = (input.shift, input.alt, input.control);
    // Option as Meta is the host sending Alt with no text. Otherwise the layout composes a
    // character with Option (German Option+L = @, US Option+A = å) and Alt is used up making it.
    let meta = alt && input.option_as_meta;
    let typed_text = input.key_char.is_some_and(|s| !s.is_empty() && !s.chars().any(char::is_control));
    let is_char = matches!(key, Key::Char(_));

    let mut mods = Mods::empty();
    mods.set(Mods::SHIFT, shift);
    mods.set(Mods::ALT, alt);
    mods.set(Mods::CTRL, control);
    if is_char && alt && !meta && !control && typed_text {
        mods.remove(Mods::ALT);
    }
    let text = if meta && !control { None } else { input.key_char };

    // GPUI gives a key no text while Control is held, or when Option started a dead key. What is
    // left to send is a control code or Meta+key (legacy), or an escape code (kitty, which reports
    // a modified key, and any key under "report all"); a bare key that typed nothing is nothing.
    if is_char && input.key_char.is_none() {
        let sends = if kitty {
            modes.kitty.contains(KittyFlags::REPORT_ALL_KEYS) || !mods.is_empty()
        } else {
            control || meta
        };
        if !sends {
            return None;
        }
    }

    let (mut key, mut text) = (key, text);
    if !kitty {
        // Shift+Enter inserts a newline in AI CLIs (same as Option+Enter).
        if key == Key::Enter && shift {
            mods.remove(Mods::SHIFT);
            mods.insert(Mods::ALT);
        }
        // Option+arrows jump words, like Terminal.app / iTerm2: Meta+b and Meta+f.
        if matches!(key, Key::Left | Key::Right) && alt && !shift && !control {
            key = Key::Char(if key == Key::Left { 'b' } else { 'f' });
            text = None;
        }
    }

    let mut event = KeyEvent::new(key).with_mods(mods);
    event.text = text;
    // Kitty's alternate key: what Shift typed, when the layout gave something other than the key
    // itself (GPUI hands Shift+1 over as `!`). The engine falls back to the upper case.
    if let (Key::Char(base), true) = (key, shift) {
        let composed = !alt || (!control && !meta && typed_text);
        let mut chars = input.key_char.unwrap_or_default().chars();
        if let (true, Some(c), None) = (composed, chars.next(), chars.next())
            && !c.is_control()
            && c != base
        {
            event = event.with_shifted(c);
        }
    }
    Some(event)
}
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: TermMode = TermMode::empty();
    const DIS: TermMode = TermMode::DISAMBIGUATE_ESC_CODES;

    fn k<'a>(key: &'a str, key_char: Option<&'a str>) -> KeyInput<'a> {
        KeyInput { key, key_char, ..Default::default() }
    }
    fn enc(i: KeyInput, mode: TermMode) -> Option<String> {
        encode(&i, mode).map(|b| String::from_utf8(b).unwrap())
    }
    fn s(x: &str) -> Option<String> {
        Some(x.to_string())
    }
    fn shift(mut i: KeyInput) -> KeyInput {
        i.shift = true;
        i
    }
    fn alt(mut i: KeyInput) -> KeyInput {
        i.alt = true;
        i
    }
    fn ctrl(mut i: KeyInput) -> KeyInput {
        i.control = true;
        i
    }

    // ---------------- legacy parity ----------------

    #[test]
    fn legacy_enter_variants() {
        assert_eq!(enc(k("enter", Some("\r")), NONE), s("\r"));
        assert_eq!(enc(shift(k("enter", None)), NONE), s("\x1b\r"));
        assert_eq!(enc(alt(k("enter", None)), NONE), s("\x1b\r"));
    }

    #[test]
    fn legacy_backspace_tab_escape() {
        assert_eq!(enc(k("backspace", None), NONE), s("\x7f"));
        assert_eq!(enc(alt(k("backspace", None)), NONE), s("\x1b\x7f"));
        assert_eq!(enc(ctrl(k("backspace", None)), NONE), s("\x08"));
        assert_eq!(enc(k("tab", None), NONE), s("\t"));
        assert_eq!(enc(shift(k("tab", None)), NONE), s("\x1b[Z"));
        assert_eq!(enc(k("escape", None), NONE), s("\x1b"));
    }

    #[test]
    fn legacy_arrows_and_app_cursor() {
        assert_eq!(enc(k("up", None), NONE), s("\x1b[A"));
        assert_eq!(enc(k("down", None), TermMode::APP_CURSOR), s("\x1bOB"));
        assert_eq!(enc(k("home", None), TermMode::APP_CURSOR), s("\x1bOH"));
        assert_eq!(enc(k("end", None), NONE), s("\x1b[F"));
        assert_eq!(enc(shift(k("up", None)), NONE), s("\x1b[1;2A"));
        assert_eq!(enc(ctrl(k("right", None)), TermMode::APP_CURSOR), s("\x1b[1;5C"));
        assert_eq!(enc(shift(alt(k("left", None))), NONE), s("\x1b[1;4D"));
    }

    #[test]
    fn legacy_option_word_jump() {
        assert_eq!(enc(alt(k("left", None)), NONE), s("\x1bb"));
        assert_eq!(enc(alt(k("right", None)), NONE), s("\x1bf"));
        assert_eq!(enc(alt(k("up", None)), NONE), s("\x1b[1;3A"));
        assert_eq!(enc(ctrl(alt(k("left", None))), NONE), s("\x1b[1;7D"));
    }

    #[test]
    fn legacy_tilde_and_function_keys() {
        assert_eq!(enc(k("insert", None), NONE), s("\x1b[2~"));
        assert_eq!(enc(k("delete", None), NONE), s("\x1b[3~"));
        assert_eq!(enc(shift(k("delete", None)), NONE), s("\x1b[3;2~"));
        assert_eq!(enc(k("pageup", None), NONE), s("\x1b[5~"));
        assert_eq!(enc(k("pagedown", None), NONE), s("\x1b[6~"));
        assert_eq!(enc(k("f1", None), NONE), s("\x1bOP"));
        assert_eq!(enc(k("f3", None), NONE), s("\x1bOR"));
        assert_eq!(enc(ctrl(k("f4", None)), NONE), s("\x1b[1;5S"));
        assert_eq!(enc(k("f5", None), NONE), s("\x1b[15~"));
        assert_eq!(enc(alt(k("f12", None)), NONE), s("\x1b[24;3~"));
        assert_eq!(enc(k("f13", None), NONE), None);
    }

    #[test]
    fn legacy_ctrl_mappings() {
        assert_eq!(encode(&ctrl(k("a", None)), NONE), Some(vec![1]));
        assert_eq!(encode(&ctrl(k("C", None)), NONE), Some(vec![3]));
        assert_eq!(encode(&ctrl(k("space", None)), NONE), Some(vec![0]));
        assert_eq!(encode(&ctrl(k("2", None)), NONE), Some(vec![0]));
        assert_eq!(encode(&ctrl(k("[", None)), NONE), Some(vec![27]));
        assert_eq!(encode(&ctrl(k("\\", None)), NONE), Some(vec![28]));
        assert_eq!(encode(&ctrl(k("]", None)), NONE), Some(vec![29]));
        assert_eq!(encode(&ctrl(k("6", None)), NONE), Some(vec![30]));
        assert_eq!(encode(&ctrl(k("-", None)), NONE), Some(vec![31]));
        assert_eq!(encode(&ctrl(k("?", None)), NONE), Some(vec![127]));
        assert_eq!(encode(&ctrl(alt(k("x", None))), NONE), Some(vec![0x1b, 24]));
        // GPUI sets no key_char while Control is held: these come from the table.
        assert_eq!(encode(&ctrl(k("/", None)), NONE), Some(vec![0x1f]), "undo in readline, zsh, emacs");
        assert_eq!(encode(&ctrl(k("~", None)), NONE), Some(vec![30]));
        assert_eq!(encode(&ctrl(k("1", None)), NONE), Some(b"1".to_vec()));
        assert_eq!(encode(&ctrl(k("0", None)), NONE), Some(b"0".to_vec()));
        assert_eq!(encode(&ctrl(k("9", None)), NONE), Some(b"9".to_vec()));
        // Unmapped ctrl char falls back to key_char.
        assert_eq!(enc(ctrl(k(".", Some("."))), NONE), s("."));
        assert_eq!(enc(ctrl(k(".", None)), NONE), None);
    }

    #[test]
    fn legacy_text() {
        assert_eq!(enc(k("a", Some("a")), NONE), s("a"));
        assert_eq!(enc(shift(k("a", Some("A"))), NONE), s("A"));
        assert_eq!(enc(k("space", Some(" ")), NONE), s(" "));
        // Option is not Meta: German Option+L sends "@".
        assert_eq!(enc(alt(k("l", Some("@"))), NONE), s("@"));
        assert_eq!(enc(k("a", None), NONE), None);
    }

    #[test]
    fn focus_is_reported_only_when_asked_for() {
        assert_eq!(focus_report(true, NONE), None);
        assert_eq!(focus_report(true, TermMode::FOCUS_IN_OUT), Some(b"\x1b[I".to_vec()));
        assert_eq!(focus_report(false, TermMode::FOCUS_IN_OUT | DIS), Some(b"\x1b[O".to_vec()));
    }

    fn meta(mut i: KeyInput) -> KeyInput {
        i.alt = true;
        i.option_as_meta = true;
        i
    }

    #[test]
    fn option_as_meta_in_both_encoders() {
        // US Option+B composes "∫"; as Meta it is ESC b, or Alt+b in kitty mode.
        assert_eq!(enc(meta(k("b", Some("∫"))), NONE), s("\x1bb"));
        assert_eq!(enc(shift(meta(k("b", Some("ı")))), NONE), s("\x1bB"));
        assert_eq!(enc(meta(k("b", Some("∫"))), DIS), s("\x1b[98;3u"));
        assert_eq!(enc(meta(k("b", Some("∫"))), DIS | TermMode::REPORT_ALL_KEYS_AS_ESC), s("\x1b[98;3u"));
        // Without it, Option composes text (German Option+L = @).
        assert_eq!(enc(alt(k("l", Some("@"))), NONE), s("@"));
        // Keys without text keep their own Option meaning.
        assert_eq!(enc(meta(k("left", None)), NONE), s("\x1bb"));
        assert_eq!(enc(ctrl(meta(k("x", None))), NONE), s("\x1b\x18"));
        assert_eq!(enc(meta(k("space", Some("\u{a0}"))), NONE), s("\x1b "));
    }

    #[test]
    fn platform_is_ignored() {
        let mut i = k("c", Some("c"));
        i.platform = true;
        assert_eq!(encode(&i, NONE), None);
        assert_eq!(encode(&i, DIS), None);
    }

    // ---------------- kitty ----------------

    #[test]
    fn kitty_disambiguate_basics() {
        assert_eq!(enc(k("escape", None), DIS), s("\x1b[27u"));
        assert_eq!(enc(alt(k("escape", None)), DIS), s("\x1b[27;3u"));
        // Enter/Tab/Backspace stay legacy when unmodified.
        assert_eq!(enc(k("enter", Some("\r")), DIS), s("\r"));
        assert_eq!(enc(k("tab", None), DIS), s("\t"));
        assert_eq!(enc(k("backspace", None), DIS), s("\x7f"));
        assert_eq!(enc(shift(k("enter", None)), DIS), s("\x1b[13;2u"));
        assert_eq!(enc(alt(k("enter", None)), DIS), s("\x1b[13;3u"));
        assert_eq!(enc(ctrl(k("enter", None)), DIS), s("\x1b[13;5u"));
        assert_eq!(enc(shift(k("tab", None)), DIS), s("\x1b[9;2u"));
        assert_eq!(enc(ctrl(k("backspace", None)), DIS), s("\x1b[127;5u"));
    }

    #[test]
    fn kitty_disambiguate_text() {
        assert_eq!(enc(k("a", Some("a")), DIS), s("a"));
        assert_eq!(enc(shift(k("a", Some("A"))), DIS), s("A"));
        assert_eq!(enc(k("space", Some(" ")), DIS), s(" "));
        assert_eq!(enc(ctrl(k("a", None)), DIS), s("\x1b[97;5u"));
        assert_eq!(enc(ctrl(k("A", None)), DIS), s("\x1b[97;5u"));
        assert_eq!(enc(ctrl(shift(k("a", None))), DIS), s("\x1b[97;6u"));
        assert_eq!(enc(ctrl(k("space", None)), DIS), s("\x1b[32;5u"));
        assert_eq!(enc(ctrl(k("[", None)), DIS), s("\x1b[91;5u"));
        assert_eq!(enc(ctrl(alt(k("x", None))), DIS), s("\x1b[120;7u"));
        // Option composes text -> alt consumed, plain text sent.
        assert_eq!(enc(alt(k("l", Some("@"))), DIS), s("@"));
        // Alt without composed text is reported.
        assert_eq!(enc(alt(k("b", None)), DIS), s("\x1b[98;3u"));
    }

    #[test]
    fn kitty_functional_keys() {
        // Unmodified functional keys stay legacy (and honour DECCKM).
        assert_eq!(enc(k("up", None), DIS), s("\x1b[A"));
        assert_eq!(enc(k("up", None), DIS | TermMode::APP_CURSOR), s("\x1bOA"));
        assert_eq!(enc(k("f1", None), DIS), s("\x1bOP"));
        assert_eq!(enc(k("delete", None), DIS), s("\x1b[3~"));
        // Modified: no Option word jump, proper CSI encoding.
        assert_eq!(enc(alt(k("left", None)), DIS), s("\x1b[1;3D"));
        assert_eq!(enc(shift(k("home", None)), DIS), s("\x1b[1;2H"));
        assert_eq!(enc(ctrl(k("pageup", None)), DIS), s("\x1b[5;5~"));
        assert_eq!(enc(ctrl(k("f3", None)), DIS), s("\x1b[13;5~"));
        assert_eq!(enc(shift(k("f1", None)), DIS), s("\x1b[1;2P"));
        assert_eq!(enc(k("f13", None), DIS), s("\x1b[57376u"));
        assert_eq!(enc(shift(k("f20", None)), DIS), s("\x1b[57383;2u"));
    }

    #[test]
    fn kitty_report_all() {
        let m = DIS | TermMode::REPORT_ALL_KEYS_AS_ESC;
        assert_eq!(enc(k("a", Some("a")), m), s("\x1b[97u"));
        assert_eq!(enc(shift(k("a", Some("A"))), m), s("\x1b[97;2u"));
        assert_eq!(enc(k("enter", Some("\r")), m), s("\x1b[13u"));
        assert_eq!(enc(k("tab", None), m), s("\x1b[9u"));
        assert_eq!(enc(k("backspace", None), m), s("\x1b[127u"));
        assert_eq!(enc(k("escape", None), m), s("\x1b[27u"));
        assert_eq!(enc(k("up", None), m | TermMode::APP_CURSOR), s("\x1b[A"));
        assert_eq!(enc(k("f1", None), m), s("\x1b[P"));
        assert_eq!(enc(k("insert", None), m), s("\x1b[2~"));
        assert_eq!(enc(k("space", Some(" ")), m), s("\x1b[32u"));
        // Report-all without disambiguate also works.
        assert_eq!(enc(k("z", Some("z")), TermMode::REPORT_ALL_KEYS_AS_ESC), s("\x1b[122u"));
    }

    #[test]
    fn kitty_alternate_keys() {
        let m = DIS | TermMode::REPORT_ALTERNATE_KEYS;
        // Shift+text without report-all is still plain text.
        assert_eq!(enc(shift(k("a", Some("A"))), m), s("A"));
        assert_eq!(enc(ctrl(shift(k("a", None))), m), s("\x1b[97:65;6u"));
        let m = m | TermMode::REPORT_ALL_KEYS_AS_ESC;
        assert_eq!(enc(shift(k("a", Some("A"))), m), s("\x1b[97:65;2u"));
        // GPUI hands Shift+1 over already shifted (`!`, Shift off) and the
        // unshifted key can't be recovered, so this differs from the spec's
        // `CSI 49:33;2u`.
        assert_eq!(enc(k("!", Some("!")), m), s("\x1b[33u"));
        assert_eq!(enc(k("a", Some("a")), m), s("\x1b[97u"));
        // Shift on a key with no shifted form: no alternate.
        assert_eq!(enc(shift(k("space", Some(" "))), m), s("\x1b[32;2u"));
    }

    #[test]
    fn kitty_associated_text() {
        let m = DIS | TermMode::REPORT_ALL_KEYS_AS_ESC | TermMode::REPORT_ASSOCIATED_TEXT;
        assert_eq!(enc(k("a", Some("a")), m), s("\x1b[97;1;97u"));
        assert_eq!(enc(shift(k("a", Some("A"))), m), s("\x1b[97;2;65u"));
        assert_eq!(enc(alt(k("l", Some("@"))), m), s("\x1b[108;1;64u"));
        // Ctrl combos and functional keys carry no text.
        assert_eq!(enc(ctrl(k("a", Some("\x01"))), m), s("\x1b[97;5u"));
        assert_eq!(enc(k("enter", Some("\r")), m), s("\x1b[13u"));
        let m = m | TermMode::REPORT_ALTERNATE_KEYS;
        assert_eq!(enc(shift(k("a", Some("A"))), m), s("\x1b[97:65;2;65u"));
        // Associated text without report-all is ignored.
        let m = DIS | TermMode::REPORT_ASSOCIATED_TEXT;
        assert_eq!(enc(k("a", Some("a")), m), s("a"));
    }

    #[test]
    fn kitty_event_types_flag_only() {
        // Press events are the default type and never carry `:1`.
        let m = TermMode::REPORT_EVENT_TYPES;
        assert_eq!(enc(k("escape", None), m), s("\x1b[27u"));
        assert_eq!(enc(shift(k("enter", None)), m), s("\x1b[13;2u"));
        assert_eq!(enc(k("a", Some("a")), m), s("a"));
    }
}
