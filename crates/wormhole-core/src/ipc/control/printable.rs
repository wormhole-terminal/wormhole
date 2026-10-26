//! Terminal text is hostile: whatever a program printed may be shown on another terminal, or put
//! in a log, by a script that reads it through the command line. Text read from a terminal must
//! not be able to drive the terminal it is printed in, so the characters that act (controls) and
//! the ones that deceive (bidi overrides, zero-width and other invisible formatting) are written
//! as escapes, for people ([`for_terminal`]) and in the JSON the command line prints
//! ([`json_line`]).

use std::borrow::Cow;
use std::fmt::Write as _;

use serde::Serialize;

/// Whether `c` is written as an escape: a control character (C0, DEL, C1), the line and paragraph
/// separators, or an invisible formatting one.
fn is_hidden(c: char) -> bool {
    c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') || wormhole_term::is_invisible(c)
}

/// `text` as it can be printed on a terminal: every hidden character as `\u{1b}` (its code point in
/// hex). A run of backslashes that comes right before `u{` or before a hidden character is doubled,
/// so an escape in the output always stands for a character and never for typed text: a run of an
/// odd length before `u{` ends in an escape, an even one is typed text.
pub fn for_terminal(text: &str) -> Cow<'_, str> {
    if !text.contains(is_hidden) && !text.contains("\\u{") {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '\\' {
            let run = rest.len() - rest.trim_start_matches('\\').len();
            let after = &rest[run..];
            let escapes = after.starts_with("u{") || after.chars().next().is_some_and(is_hidden);
            out.extend(std::iter::repeat_n('\\', if escapes { 2 * run } else { run }));
            rest = after;
        } else if is_hidden(c) {
            let _ = write!(out, "\\u{{{:x}}}", c as u32);
            rest = &rest[c.len_utf8()..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    Cow::Owned(out)
}

/// `value` as one line of JSON, with the characters `for_terminal` hides written as `\uXXXX` (a
/// surrogate pair past the basic plane). serde_json escapes the C0 controls already; this adds DEL,
/// C1, the separators and the invisible ones, which can only occur inside strings, so a pass over
/// the characters is exact and a JSON parser reads the original text back.
pub fn json_line(value: &impl Serialize) -> String {
    let json = serde_json::to_string(value).unwrap_or_default();
    if !json.contains(is_hidden) {
        return json;
    }
    let mut out = String::with_capacity(json.len() + 16);
    for c in json.chars() {
        if is_hidden(c) {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                let _ = write!(out, "\\u{unit:04x}");
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{for_terminal, json_line};

    #[test]
    fn plain_text_is_left_alone_and_not_copied() {
        for text in ["", "hello", "naïve café 日本語 🎉", "tab\u{fe0f}"] {
            let shown = for_terminal(text);
            if text != "tab\u{fe0f}" {
                assert!(matches!(shown, std::borrow::Cow::Borrowed(_)), "{text:?}");
            }
        }
        assert_eq!(for_terminal("a b"), "a b");
    }

    #[test]
    fn controls_and_invisible_characters_are_written_as_escapes() {
        assert_eq!(for_terminal("\u{1b}[31mred\u{7}"), "\\u{1b}[31mred\\u{7}");
        assert_eq!(for_terminal("a\nb\tc\u{7f}"), "a\\u{a}b\\u{9}c\\u{7f}");
        assert_eq!(for_terminal("\u{9b}31m"), "\\u{9b}31m", "C1 acts as an escape too");
        // Bidi overrides, zero-width, soft hyphen, tag characters, the separators.
        assert_eq!(for_terminal("a\u{202e}b"), "a\\u{202e}b");
        assert_eq!(for_terminal("a\u{200b}b\u{ad}c"), "a\\u{200b}b\\u{ad}c");
        assert_eq!(for_terminal("\u{e0041}"), "\\u{e0041}");
        assert_eq!(for_terminal("a\u{2028}b"), "a\\u{2028}b");
    }

    /// An escape in the output stands for a character; text that merely looks like one is marked.
    #[test]
    fn typed_text_that_looks_like_an_escape_is_not_read_as_one() {
        let typed = "\\u{1b}[31m";
        let shown = for_terminal(typed);
        assert_eq!(shown, "\\\\u{1b}[31m", "a typed backslash before u{{ is doubled");
        let real = for_terminal("\u{1b}[31m");
        assert_eq!(real, "\\u{1b}[31m");
        assert_ne!(shown, real);
        // Odd run: an escape ends it; even: it is all typed. Never the same output for both.
        assert_eq!(for_terminal("\\\u{1b}"), "\\\\\\u{1b}", "a backslash, then an ESC");
        assert_eq!(for_terminal("\\\\u{1b}"), "\\\\\\\\u{1b}", "two typed backslashes, then u{{1b}}");
        // A backslash that begins nothing is left alone.
        assert_eq!(for_terminal("C:\\users\\me"), "C:\\users\\me");
        assert_eq!(for_terminal("trailing \\"), "trailing \\");
    }

    #[test]
    fn json_lines_escape_what_serde_does_not_and_read_back_whole() {
        let text = "a\u{1b}[31m\u{7f}\u{9b}\u{202e}\u{200b}\u{e0041}\u{2028}z \"q\" \\ é 🎉\n";
        let line = json_line(&json!({ "text": text }));
        assert!(line.is_ascii() || !line.contains(['\u{7f}', '\u{9b}', '\u{202e}', '\u{200b}', '\u{2028}']));
        for hidden in ['\u{1b}', '\u{7f}', '\u{9b}', '\u{202e}', '\u{200b}', '\u{e0041}', '\u{2028}', '\n'] {
            assert!(!line.contains(hidden), "{hidden:?} in {line}");
        }
        assert!(line.contains("\\udb40\\udc41"), "a surrogate pair past the basic plane: {line}");
        let back: Value = serde_json::from_str(&line).expect("parses");
        assert_eq!(back["text"], text, "a JSON parser reads the original back");
    }

    #[test]
    fn json_without_hidden_characters_is_what_serde_writes() {
        let value = json!({ "a": [1, 2], "b": "naïve 日本" });
        assert_eq!(json_line(&value), serde_json::to_string(&value).expect("serializes"));
    }
}
