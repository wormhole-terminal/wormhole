//! `send-keys`: key names in, bytes for the program out.
//!
//! A request is a list of items, each a key (`ctrl-c`, `up`, `shift-tab`, `f5`) or a segment of
//! text. The grammar is parsed here, once, for the command line (so a mistake is exit code 2
//! before anything is sent) and for the app (which trusts nothing it is sent). The bytes are made
//! with the terminal's modes as they are when each item goes out (`wormhole_term::input`):
//! application cursor keys, the kitty keyboard flags, modifyOtherKeys, and bracketed paste for a
//! segment sent as a paste.

use serde::{Deserialize, Serialize};
use wormhole_term::input::keys::{self, KeyInput};
use wormhole_term::types::Modes;

use super::Target;

/// The most items one `send-keys` may have.
pub const KEYS_MAX_ITEMS: usize = 64;
/// The longest key name, in bytes.
pub const KEY_SPEC_MAX_BYTES: usize = 32;
/// The most text one `send-keys` may type, all its segments together.
pub const KEYS_TEXT_MAX_BYTES: usize = 4096;

/// `send-keys`' request: the items to send, in order, to a terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendKeysSpec {
    pub target: Target,
    pub keys: Vec<KeyItem>,
}

/// One thing to send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyItem {
    /// A key press: `ctrl-c`, `enter`, `alt-x`.
    Key { key: String },
    /// Text: a key press for each character, as typing it would be; or, with `paste`, as a paste
    /// (wrapped in the program's bracketed-paste markers when it asked for them).
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        paste: bool,
    },
}

/// A key press, parsed: the key by the name GPUI gives it (what `wormhole_term::input::keys`
/// reads) and the modifiers held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChord {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// An item ready to be encoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stroke {
    Key(KeyChord),
    Typed(String),
    Pasted(String),
}

/// What is wrong with a key name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    Empty,
    TooLong,
    /// A modifier of the app itself, which is not the program's to be sent.
    AppModifier(String),
    Unknown(String),
}

/// The names a key may have, for the error that lists them.
const NAMES: &str = "enter, tab, backspace, escape, space, up, down, left, right, home, end, pageup, \
                     pagedown, insert, delete, f1 to f24, or one character";

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::Empty => write!(f, "an empty key name (keys are {NAMES}, with ctrl-, alt- and shift-)"),
            KeyError::TooLong => write!(f, "a key name over {KEY_SPEC_MAX_BYTES} bytes"),
            KeyError::AppModifier(name) => {
                write!(f, "`{name}` is Wormhole's own key, not the program's: use ctrl-, alt- or shift-")
            }
            KeyError::Unknown(name) => {
                write!(f, "unknown key `{name}` (keys are {NAMES}, with ctrl-, alt- and shift-)")
            }
        }
    }
}

impl std::error::Error for KeyError {}

/// A caller's word, repeated back only when it is a short plain one.
fn shown(word: &str) -> String {
    let plain = word.len() <= KEY_SPEC_MAX_BYTES && word.chars().all(|c| c.is_ascii_graphic());
    if plain { word.to_string() } else { "?".to_string() }
}

/// The modifier a word names, case aside.
fn modifier(word: &str) -> Option<fn(&mut KeyChord)> {
    Some(match word.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => |chord| chord.ctrl = true,
        "alt" | "option" | "opt" | "meta" => |chord| chord.alt = true,
        "shift" => |chord| chord.shift = true,
        _ => return None,
    })
}

fn is_app_modifier(word: &str) -> bool {
    matches!(word.to_ascii_lowercase().as_str(), "cmd" | "command" | "super" | "win" | "windows")
}

/// The name GPUI gives the key `word` names, when it names one by a word.
fn named(word: &str) -> Option<String> {
    let word = word.to_ascii_lowercase();
    let name = match word.as_str() {
        "enter" | "return" => "enter",
        "escape" | "esc" => "escape",
        "delete" | "del" => "delete",
        "tab" | "backspace" | "space" | "up" | "down" | "left" | "right" | "home" | "end" | "pageup"
        | "pagedown" | "insert" => word.as_str(),
        _ => {
            let number = word.strip_prefix('f')?;
            let number: u8 = number.parse().ok().filter(|n| (1..=24).contains(n))?;
            return Some(format!("f{number}"));
        }
    };
    Some(name.to_string())
}

/// Read a key name: modifiers (`ctrl-`, `alt-`, `shift-`; `option-`, `opt-`, `meta-` for alt) and
/// then the key. Modifiers come off the left while a known one is followed by `-`, so the key may
/// be `-` itself (`ctrl--`). A single character is its own key; a capital letter is shift and the
/// letter. `cmd` is the app's own and refused.
pub fn parse_key(spec: &str) -> Result<KeyChord, KeyError> {
    if spec.is_empty() {
        return Err(KeyError::Empty);
    }
    if spec.len() > KEY_SPEC_MAX_BYTES {
        return Err(KeyError::TooLong);
    }
    let mut chord = KeyChord { key: String::new(), ctrl: false, alt: false, shift: false };
    let mut rest = spec;
    while let Some(dash) = rest.find('-').filter(|dash| *dash > 0 && dash + 1 < rest.len()) {
        let word = &rest[..dash];
        if is_app_modifier(word) {
            return Err(KeyError::AppModifier(shown(word)));
        }
        let Some(apply) = modifier(word) else { break };
        apply(&mut chord);
        rest = &rest[dash + 1..];
    }
    if is_app_modifier(rest) {
        return Err(KeyError::AppModifier(shown(rest)));
    }
    if let Some(key) = named(rest) {
        chord.key = key;
        return Ok(chord);
    }
    let mut chars = rest.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if !c.is_control() => {
            if c.is_uppercase() {
                chord.shift = true;
            }
            chord.key = c.to_lowercase().collect();
            Ok(chord)
        }
        _ => Err(KeyError::Unknown(shown(rest))),
    }
}

/// Why an item cannot be sent.
pub fn parse_items(items: &[KeyItem]) -> Result<Vec<Stroke>, KeyError> {
    items
        .iter()
        .map(|item| match item {
            KeyItem::Key { key } => parse_key(key).map(Stroke::Key),
            KeyItem::Text { text, paste: false } => Ok(Stroke::Typed(text.clone())),
            KeyItem::Text { text, paste: true } => Ok(Stroke::Pasted(text.clone())),
        })
        .collect()
}

impl Stroke {
    /// The bytes for the program, with its modes as they are now; empty when the modes make the
    /// item mean nothing.
    pub fn encode(&self, modes: Modes) -> Vec<u8> {
        match self {
            Stroke::Key(chord) => {
                // A plain character key types its character, as the layout would give it.
                let mut buffer = [0u8; 4];
                let typed = match chord.key.as_str() {
                    "space" if !chord.ctrl && !chord.alt => Some(" "),
                    key if !chord.ctrl && !chord.alt && key.chars().count() == 1 => {
                        Some(typed_char(key.chars().next().unwrap_or(' '), chord.shift, &mut buffer))
                    }
                    _ => None,
                };
                let input = KeyInput {
                    key: &chord.key,
                    key_char: typed,
                    shift: chord.shift,
                    alt: chord.alt,
                    control: chord.ctrl,
                    platform: false,
                    option_as_meta: true,
                };
                keys::encode(&input, modes).unwrap_or_default()
            }
            Stroke::Typed(text) => {
                let mut out = Vec::new();
                for c in text.chars() {
                    let mut buffer = [0u8; 4];
                    let one = c.encode_utf8(&mut buffer);
                    let lower: String = c.to_lowercase().collect();
                    let input = KeyInput {
                        key: if c == ' ' { "space" } else { &lower },
                        key_char: Some(one),
                        shift: c.is_uppercase(),
                        ..KeyInput::default()
                    };
                    out.extend(keys::encode(&input, modes).unwrap_or_default());
                }
                out
            }
            Stroke::Pasted(text) => wormhole_term::input::paste(text, modes),
        }
    }
}

/// The character a plain key press types: the key, upper case with shift.
fn typed_char(c: char, shift: bool, buffer: &mut [u8; 4]) -> &str {
    let c = if shift { c.to_uppercase().next().unwrap_or(c) } else { c };
    c.encode_utf8(buffer)
}

/// Whether a text segment may be sent: printable characters only. Control codes are keys
/// (`ctrl-c`, `enter`, `escape`), named as such, and never smuggled in text.
pub fn text_problem(text: &str) -> Option<&'static str> {
    if text.is_empty() {
        return Some("a text segment is empty");
    }
    text.chars()
        .any(|c| c.is_control() || wormhole_term::is_invisible(c) || matches!(c, '\u{2028}' | '\u{2029}'))
        .then_some("text takes printable characters only: use key names (enter, tab, escape, ctrl-c) for control keys")
}

#[cfg(test)]
mod tests {
    use wormhole_term::types::Modes;

    use super::{KeyChord, KeyError, KeyItem, Stroke, parse_items, parse_key, text_problem};

    fn chord(spec: &str) -> (String, bool, bool, bool) {
        let chord = parse_key(spec).unwrap_or_else(|e| panic!("{spec}: {e}"));
        (chord.key, chord.ctrl, chord.alt, chord.shift)
    }

    fn key(name: &str, ctrl: bool, alt: bool, shift: bool) -> (String, bool, bool, bool) {
        (name.to_string(), ctrl, alt, shift)
    }

    #[test]
    fn every_name_the_grammar_has_parses() {
        for (spec, name) in [
            ("enter", "enter"),
            ("Return", "enter"),
            ("tab", "tab"),
            ("backspace", "backspace"),
            ("escape", "escape"),
            ("esc", "escape"),
            ("space", "space"),
            ("up", "up"),
            ("down", "down"),
            ("left", "left"),
            ("right", "right"),
            ("home", "home"),
            ("end", "end"),
            ("pageup", "pageup"),
            ("pagedown", "pagedown"),
            ("insert", "insert"),
            ("delete", "delete"),
            ("del", "delete"),
            ("f1", "f1"),
            ("F24", "f24"),
            ("x", "x"),
            ("-", "-"),
            ("é", "é"),
        ] {
            assert_eq!(chord(spec), key(name, false, false, false), "{spec}");
        }
        for spec in ["f0", "f25", "ff", "foo", "ctrl-", "ctrl-ctrl-", ""] {
            assert!(parse_key(spec).is_err(), "{spec:?} is not a key");
        }
    }

    #[test]
    fn modifiers_come_off_the_left_and_the_key_may_be_a_dash() {
        assert_eq!(chord("ctrl-c"), key("c", true, false, false));
        assert_eq!(chord("Control-C"), key("c", true, false, true), "a capital is a shift");
        assert_eq!(chord("alt-x"), key("x", false, true, false));
        assert_eq!(chord("option-x"), key("x", false, true, false));
        assert_eq!(chord("meta-x"), key("x", false, true, false));
        assert_eq!(chord("shift-tab"), key("tab", false, false, true));
        assert_eq!(chord("ctrl-alt-shift-f5"), key("f5", true, true, true));
        assert_eq!(chord("ctrl--"), key("-", true, false, false));
        assert_eq!(chord("alt--"), key("-", false, true, false));
        assert_eq!(chord("ctrl-ctrl-a"), key("a", true, false, false), "twice is once");
        assert_eq!(chord("alt-enter"), key("enter", false, true, false));
    }

    #[test]
    fn the_apps_own_key_is_refused_and_a_hostile_name_is_not_echoed() {
        for spec in ["cmd-q", "super-q", "ctrl-cmd-c", "command", "win-x"] {
            assert!(matches!(parse_key(spec), Err(KeyError::AppModifier(_))), "{spec}");
        }
        assert_eq!(parse_key(&"x".repeat(33)), Err(KeyError::TooLong));
        let message = parse_key("\u{1b}[2J").expect_err("not a key").to_string();
        assert!(message.contains("`?`") && !message.contains('\u{1b}'), "{message}");
        let named = parse_key("nope").expect_err("not a key").to_string();
        assert!(named.contains("`nope`") && named.contains("pagedown"), "{named}");
    }

    #[test]
    fn text_takes_printable_characters_only() {
        assert_eq!(text_problem("ls -la | grep 日本"), None);
        for bad in
            ["a\nb", "x\ry", "\u{1b}[31m", "tab\there", "bell\x07", "del\x7f", "c1\u{9b}", "bidi\u{202e}", ""]
        {
            assert!(text_problem(bad).is_some(), "{bad:?}");
        }
    }

    fn bytes(spec: &str, modes: Modes) -> Vec<u8> {
        Stroke::Key(parse_key(spec).expect("parses")).encode(modes)
    }

    #[test]
    fn keys_are_encoded_with_the_modes_the_program_has_set() {
        let none = Modes::empty();
        assert_eq!(bytes("up", none), b"\x1b[A");
        assert_eq!(bytes("up", Modes::APP_CURSOR), b"\x1bOA", "application cursor keys");
        assert_eq!(bytes("ctrl-c", none), [3]);
        assert_eq!(bytes("enter", none), b"\r");
        assert_eq!(bytes("escape", none), [0x1b]);
        assert_eq!(bytes("tab", none), b"\t");
        assert_eq!(bytes("shift-tab", none), b"\x1b[Z");
        assert_eq!(bytes("alt-x", none), b"\x1bx", "meta sends escape first");
        assert_eq!(bytes("backspace", none), [0x7f]);
        assert_eq!(bytes("f5", none), b"\x1b[15~");
        assert_eq!(bytes("pageup", none), b"\x1b[5~");
        assert_eq!(bytes("delete", none), b"\x1b[3~");
        assert_eq!(bytes("a", none), b"a");
        assert_eq!(bytes("A", none), b"A");
        assert_eq!(bytes("space", none), b" ");
        // The kitty keyboard protocol: disambiguated escape codes.
        let kitty = Modes::KEYS_DISAMBIGUATE;
        assert_eq!(bytes("escape", kitty), b"\x1b[27u");
        assert_eq!(bytes("ctrl-c", kitty), b"\x1b[99;5u");
        assert_eq!(bytes("up", kitty), b"\x1b[A");
    }

    #[test]
    fn text_is_typed_a_key_at_a_time_or_pasted_in_the_programs_brackets() {
        let none = Modes::empty();
        assert_eq!(Stroke::Typed("ls -la".into()).encode(none), b"ls -la");
        assert_eq!(Stroke::Typed("Ab é日".into()).encode(none), "Ab é日".as_bytes());
        let kitty_all = Modes::KEYS_DISAMBIGUATE | Modes::KEYS_ALL_AS_ESCAPES;
        let typed = Stroke::Typed("a".into()).encode(kitty_all);
        assert_eq!(typed, b"\x1b[97u", "a program that wants every key as an escape gets it");
        assert_eq!(Stroke::Pasted("x y".into()).encode(none), b"x y");
        assert_eq!(Stroke::Pasted("x y".into()).encode(Modes::BRACKETED_PASTE), b"\x1b[200~x y\x1b[201~");
    }

    #[test]
    fn items_parse_in_order_and_the_wire_shapes_read_back() {
        let items: Vec<KeyItem> = serde_json::from_str(
            r#"[{"key":"ctrl-c"},{"text":"ls -la"},{"text":"x","paste":true},{"key":"enter"}]"#,
        )
        .expect("parses");
        assert_eq!(
            serde_json::to_string(&items).expect("serializes"),
            r#"[{"key":"ctrl-c"},{"text":"ls -la"},{"text":"x","paste":true},{"key":"enter"}]"#
        );
        let strokes = parse_items(&items).expect("parses");
        assert_eq!(strokes.len(), 4);
        assert!(matches!(&strokes[0], Stroke::Key(KeyChord { key, ctrl: true, .. }) if key == "c"));
        assert_eq!(strokes[1], Stroke::Typed("ls -la".into()));
        assert_eq!(strokes[2], Stroke::Pasted("x".into()));
        assert!(parse_items(&[KeyItem::Key { key: "nope".into() }]).is_err());
        assert!(serde_json::from_str::<Vec<KeyItem>>(r#"[{"neither":1}]"#).is_err());
    }
}
