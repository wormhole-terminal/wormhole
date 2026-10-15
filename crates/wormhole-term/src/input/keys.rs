//! Keystroke -> bytes for the PTY.
//!
//! Two encoders live here:
//!
//! * the legacy xterm encoding (used when no kitty keyboard flag is set), and
//! * the kitty keyboard protocol ("progressive enhancement",
//!   <https://sw.kovidgoyal.net/kitty/keyboard-protocol/>), used as soon as any
//!   of the kitty flags is pushed by the application.
//!
//! Only key-*press* events are encoded: the host UI only delivers key-down
//! events, so `REPORT_EVENT_TYPES` has no visible effect (press is the default
//! event type and is always omitted from the sequence).

use wormhole_vt::types::TermMode;

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
    let kitty = mode.intersects(
        TermMode::DISAMBIGUATE_ESC_CODES
            | TermMode::REPORT_EVENT_TYPES
            | TermMode::REPORT_ALTERNATE_KEYS
            | TermMode::REPORT_ALL_KEYS_AS_ESC
            | TermMode::REPORT_ASSOCIATED_TEXT,
    );
    if kitty { encode_kitty(input, mode) } else { encode_legacy(input, mode.contains(TermMode::APP_CURSOR)) }
}

/// What to send when the terminal gains or loses focus: `CSI I` / `CSI O`
/// once the program asked for focus events (DECSET 1004; vim, tmux and TUIs
/// that pause when unfocused), otherwise nothing.
pub fn focus_report(focused: bool, mode: TermMode) -> Option<&'static [u8]> {
    mode.contains(TermMode::FOCUS_IN_OUT).then_some(if focused { b"\x1b[I" } else { b"\x1b[O" })
}

// ---------------------------------------------------------------------------
// Legacy xterm encoding
// ---------------------------------------------------------------------------

fn encode_legacy(input: &KeyInput, app_cursor: bool) -> Option<Vec<u8>> {
    let key = input.key;
    let (shift, alt, control) = (input.shift, input.alt, input.control);
    // GPUI names the space bar "space"; as Meta it is ESC SP (Option+Space would type a NBSP).
    let key_text = if key == "space" { " " } else { key };
    if alt && input.option_as_meta && !control && key_text.chars().count() == 1 {
        let key = if shift { key_text.to_uppercase() } else { key_text.to_string() };
        return Some([b"\x1b".as_slice(), key.as_bytes()].concat());
    }
    // xterm modifier parameter: 1 + shift + 2*alt + 4*ctrl.
    let modcode = 1 + shift as u8 + 2 * alt as u8 + 4 * control as u8;

    let cursor = |final_byte: char| {
        if modcode > 1 {
            format!("\x1b[1;{modcode}{final_byte}")
        } else if app_cursor {
            format!("\x1bO{final_byte}")
        } else {
            format!("\x1b[{final_byte}")
        }
    };
    let tilde = |n: u8| {
        if modcode > 1 { format!("\x1b[{n};{modcode}~") } else { format!("\x1b[{n}~") }
    };
    let ss3 = |final_byte: char| {
        if modcode > 1 { format!("\x1b[1;{modcode}{final_byte}") } else { format!("\x1bO{final_byte}") }
    };

    let seq: String = match key {
        // Shift+Enter inserts a newline in AI CLIs (same as Option+Enter).
        "enter" if shift || alt => "\x1b\r".into(),
        "enter" => "\r".into(),
        "backspace" if alt => "\x1b\x7f".into(),
        "backspace" if control => "\x08".into(),
        "backspace" => "\x7f".into(),
        "tab" if shift => "\x1b[Z".into(),
        "tab" => "\t".into(),
        "escape" => "\x1b".into(),
        // Option+arrows jump words, like Terminal.app / iTerm2.
        "left" if alt && !shift && !control => "\x1bb".into(),
        "right" if alt && !shift && !control => "\x1bf".into(),
        "up" => cursor('A'),
        "down" => cursor('B'),
        "right" => cursor('C'),
        "left" => cursor('D'),
        "home" => cursor('H'),
        "end" => cursor('F'),
        "insert" => tilde(2),
        "delete" => tilde(3),
        "pageup" => tilde(5),
        "pagedown" => tilde(6),
        "f1" => ss3('P'),
        "f2" => ss3('Q'),
        "f3" => ss3('R'),
        "f4" => ss3('S'),
        "f5" => tilde(15),
        "f6" => tilde(17),
        "f7" => tilde(18),
        "f8" => tilde(19),
        "f9" => tilde(20),
        "f10" => tilde(21),
        "f11" => tilde(23),
        "f12" => tilde(24),
        "space" if control => "\0".into(),
        _ if control && key.chars().count() == 1 => {
            let c = key.chars().next()?;
            let byte = match c.to_ascii_lowercase() {
                c @ 'a'..='z' => c as u8 & 0x1f,
                '@' | '2' => 0,
                '[' | '3' => 27,
                '\\' | '4' => 28,
                ']' | '5' => 29,
                '^' | '6' | '~' => 30,
                '_' | '-' | '7' | '/' => 31,
                '?' | '8' => 127,
                // As xterm, kitty and iTerm2 send them.
                c @ ('0' | '1' | '9') => c as u8,
                _ => return input.key_char.map(|s| s.as_bytes().to_vec()),
            };
            return Some(if alt { vec![0x1b, byte] } else { vec![byte] });
        }
        // Everything else is text. Option is *not* Meta by default so that
        // international layouts (e.g. German Option+L = @) keep working.
        _ => input.key_char?.to_string(),
    };
    Some(seq.into_bytes())
}

// ---------------------------------------------------------------------------
// Kitty keyboard protocol
// ---------------------------------------------------------------------------

/// How a key is terminated in the kitty protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Term {
    /// `CSI code ; mods u`
    U(u32),
    /// `CSI number ; mods ~`
    Tilde(u32),
    /// `CSI 1 ; mods <letter>` (or `CSI <letter>` without modifiers)
    Letter(char),
}

fn functional_key(key: &str) -> Option<Term> {
    Some(match key {
        "escape" => Term::U(27),
        "enter" => Term::U(13),
        "tab" => Term::U(9),
        "backspace" => Term::U(127),
        "insert" => Term::Tilde(2),
        "delete" => Term::Tilde(3),
        "pageup" => Term::Tilde(5),
        "pagedown" => Term::Tilde(6),
        "up" => Term::Letter('A'),
        "down" => Term::Letter('B'),
        "right" => Term::Letter('C'),
        "left" => Term::Letter('D'),
        "home" => Term::Letter('H'),
        "end" => Term::Letter('F'),
        "f1" => Term::Letter('P'),
        "f2" => Term::Letter('Q'),
        // F3 uses `CSI 13 ~` in kitty: `CSI R` collides with cursor position reports.
        "f3" => Term::Tilde(13),
        "f4" => Term::Letter('S'),
        "f5" => Term::Tilde(15),
        "f6" => Term::Tilde(17),
        "f7" => Term::Tilde(18),
        "f8" => Term::Tilde(19),
        "f9" => Term::Tilde(20),
        "f10" => Term::Tilde(21),
        "f11" => Term::Tilde(23),
        "f12" => Term::Tilde(24),
        _ => {
            // F13..F35 live in the private use area starting at 57376.
            let n: u32 = key.strip_prefix('f')?.parse().ok()?;
            if (13..=35).contains(&n) {
                Term::U(57376 + n - 13)
            } else {
                return None;
            }
        }
    })
}

/// Printable text (no C0/C1 controls, not empty).
fn printable(s: &str) -> Option<&str> {
    (!s.is_empty() && !s.chars().any(char::is_control)).then_some(s)
}

fn single_char(s: &str) -> Option<char> {
    let mut it = s.chars();
    let c = it.next()?;
    it.next().is_none().then_some(c)
}

fn encode_kitty(input: &KeyInput, mode: TermMode) -> Option<Vec<u8>> {
    let report_all = mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC);
    let report_alt = mode.contains(TermMode::REPORT_ALTERNATE_KEYS);
    let report_text = report_all && mode.contains(TermMode::REPORT_ASSOCIATED_TEXT);

    let functional = functional_key(input.key);

    // The text this key produces. Ctrl combos never carry text, nor does
    // Option when it is Meta.
    let meta = input.alt && input.option_as_meta;
    let text =
        if input.control || meta || functional.is_some() { None } else { input.key_char.and_then(printable) };

    // Option is not Meta: when Alt composed a character (German Option+L = @,
    // US Option+A = å) the modifier was consumed by the layout, like kitty with
    // `macos_option_as_alt no`. Only report Alt when it produced no text.
    let alt = input.alt && text.is_none();
    let (shift, control) = (input.shift, input.control);
    let mods = shift as u32 + 2 * alt as u32 + 4 * control as u32;

    if let Some(term) = functional {
        // Escape is always disambiguated; Enter/Tab/Backspace stay legacy when
        // unmodified so a shell stays usable after a crashed app; other
        // unmodified functional keys are unambiguous in legacy form (and keep
        // honouring DECCKM).
        if !report_all && mods == 0 && term != Term::U(27) {
            // Keys with no legacy form (F13+) fall through to CSI u.
            if let Some(bytes) = encode_legacy(input, mode.contains(TermMode::APP_CURSOR)) {
                return Some(bytes);
            }
        }
        return Some(csi(term, &[], mods, None));
    }

    // Text-producing / character keys.
    let base_char = match input.key {
        "space" => ' ',
        k => single_char(k)?.to_lowercase().next()?,
    };
    // Unmodified (or Shift-only) text goes out as plain text unless all keys
    // must be reported as escape codes.
    if !report_all && (mods & !1) == 0 {
        if let Some(t) = text {
            return Some(t.as_bytes().to_vec());
        }
        if mods == 0 {
            return input.key_char.map(|s| s.as_bytes().to_vec());
        }
    }

    let shifted = if report_alt && shift {
        input
            .key_char
            .filter(|_| !input.alt || text.is_some())
            .and_then(single_char)
            .filter(|c| !c.is_control() && *c != base_char)
            .or_else(|| {
                let up = base_char.to_uppercase().next()?;
                (up != base_char).then_some(up)
            })
    } else {
        None
    };

    let mut code = vec![base_char as u32];
    if let Some(s) = shifted {
        code.push(s as u32);
    }
    let text = if report_text { text } else { None };
    Some(csi(Term::U(0), &code, mods, text))
}

fn csi(term: Term, code_parts: &[u32], mods: u32, text: Option<&str>) -> Vec<u8> {
    let join = |v: &mut dyn Iterator<Item = u32>| v.map(|n| n.to_string()).collect::<Vec<_>>().join(":");
    let mut s = String::from("\x1b[");
    let (first, fin) = match term {
        Term::U(n) => {
            let first =
                if code_parts.is_empty() { n.to_string() } else { join(&mut code_parts.iter().copied()) };
            (first, 'u')
        }
        Term::Tilde(n) => (n.to_string(), '~'),
        Term::Letter(c) => (if mods == 0 { String::new() } else { "1".into() }, c),
    };
    s.push_str(&first);
    if mods != 0 || text.is_some() {
        s.push(';');
        s.push_str(&(mods + 1).to_string());
    }
    if let Some(t) = text {
        s.push(';');
        s.push_str(&join(&mut t.chars().map(|c| c as u32)));
    }
    s.push(fin);
    s.into_bytes()
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
        assert_eq!(focus_report(true, TermMode::FOCUS_IN_OUT), Some(&b"\x1b[I"[..]));
        assert_eq!(focus_report(false, TermMode::FOCUS_IN_OUT | DIS), Some(&b"\x1b[O"[..]));
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
