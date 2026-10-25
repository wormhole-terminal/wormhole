//! Characters that would hide part of a text where a person is asked to approve it.
//!
//! Everything a Probe says about itself ends up on a sheet the user reads before enabling it, so a
//! name or an argument must not be able to reorder itself (bidi overrides, "Trojan Source"), vanish
//! (zero-width and other invisible characters), or carry text only a language model reads (Unicode
//! tags). The set is the one project trust writes out in its approval dialog
//! (`project::trust::hidden`); a Probe's text is refused when it has one, rather than shown escaped.
//! Where text of a Probe must be printed all the same (the command line names what it refused), it
//! goes through [`reveal`], which writes the characters out.

use std::borrow::Cow;

/// Whether `c` is a control character or one that draws as nothing or reorders what is around it.
pub(crate) fn is_hidden(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2069}')
        || matches!(c, '\u{061c}' | '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{00ad}' | '\u{180e}')
        || matches!(c, '\u{115f}' | '\u{1160}' | '\u{3164}' | '\u{ffa0}')
        || matches!(c, '\u{fe00}'..='\u{fe0f}' | '\u{e0000}'..='\u{e007f}' | '\u{e0100}'..='\u{e01ef}')
        // Joiners and fillers that draw as nothing, the interlinear annotation marks that hide text
        // between them, and the musical formatting controls.
        || matches!(c, '\u{034f}' | '\u{17b4}' | '\u{17b5}' | '\u{fff9}'..='\u{fffb}' | '\u{1d173}'..='\u{1d17a}')
}

/// Whether `text` holds a character [`is_hidden`] names.
pub(crate) fn has_hidden(text: &str) -> bool {
    text.chars().any(is_hidden)
}

/// `text` with every [`is_hidden`] character written out (`\n`, `\t`, `\u{202e}`), so that text a Probe
/// or a folder chose can be printed without driving the terminal it is printed on.
///
/// What is written out must not be forgeable by text that merely looks like it, so a literal backslash
/// that could begin one of our escapes is doubled: `\u{1b}` typed in a name comes out as `\\u{1b}`, and
/// only a real ESC comes out as `\u{1b}`. A backslash that begins no escape (the `\b` of a pattern) is
/// left alone while nothing in the text is hidden, because a hint such as `write (?-u:\b)` must stay
/// something to copy; once the text has a hidden character every backslash is doubled, so a
/// backslash can not be taken for the start of the escape that follows it.
pub fn reveal(text: &str) -> Cow<'_, str> {
    let hidden = has_hidden(text);
    let looks_like_an_escape = |rest: &str| rest.starts_with(['n', 'r', 't', 'u', '\\']);
    if !hidden && !text.match_indices('\\').any(|(at, _)| looks_like_an_escape(&text[at + 1..])) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    for (at, c) in text.char_indices() {
        match c {
            '\\' if hidden || looks_like_an_escape(&text[at + 1..]) => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if is_hidden(c) => out.push_str(&format!("\\u{{{:x}}}", u32::from(c))),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_bidi_and_invisible_characters_are_hidden() {
        for c in [
            '\n',
            '\u{1b}',
            '\u{85}',
            '\u{202e}',
            '\u{2066}',
            '\u{200b}',
            '\u{200d}',
            '\u{feff}',
            '\u{2028}',
            '\u{e0041}',
            '\u{34f}',
            '\u{17b4}',
            '\u{17b5}',
            '\u{fff9}',
            '\u{fffa}',
            '\u{fffb}',
            '\u{1d173}',
            '\u{1d17a}',
        ] {
            assert!(is_hidden(c), "{:x}", u32::from(c));
        }
        assert!(has_hidden("a\u{202e}b"));
        for c in ['a', ' ', 'é', '→', '日', '-'] {
            assert!(!is_hidden(c), "{c}");
        }
        assert!(!has_hidden("Acme tools — deploy"));
    }

    #[test]
    fn revealing_writes_hidden_characters_out_and_leaves_the_rest() {
        assert_eq!(reveal("Acme tools — deploy"), "Acme tools — deploy");
        assert!(matches!(reveal("plain"), Cow::Borrowed(_)));
        assert_eq!(reveal("a\u{1b}[2Jb\u{202e}c\nd\te\r"), "a\\u{1b}[2Jb\\u{202e}c\\nd\\te\\r");
        assert_eq!(reveal("\u{e0041}"), "\\u{e0041}");
    }

    #[test]
    fn a_backslash_that_could_begin_an_escape_is_doubled_so_text_cannot_forge_one() {
        // Typed by a Probe: looks like a revealed ESC, is not one.
        assert_eq!(reveal("\\u{1b}"), "\\\\u{1b}");
        assert_eq!(reveal("a\\nb"), "a\\\\nb");
        // Two backslashes: the first begins what looks like an escape and is doubled, the last ends the text.
        assert_eq!(reveal("\\\\"), "\\\\\\");
        // A real ESC after a backslash stays apart from it.
        assert_eq!(reveal("\\\u{1b}"), "\\\\\\u{1b}");
        assert_eq!(reveal("x\\\u{202e}y"), "x\\\\\\u{202e}y");
    }

    #[test]
    fn a_backslash_that_begins_no_escape_is_left_for_copying_when_nothing_is_hidden() {
        assert_eq!(reveal(r"write `(?-u:\b)` and \d"), r"write `(?-u:\b)` and \d");
        assert!(matches!(reveal(r"\b"), Cow::Borrowed(_)));
        // With a hidden character about, every backslash is doubled.
        assert_eq!(reveal("\\b\u{1b}"), "\\\\b\\u{1b}");
    }
}
