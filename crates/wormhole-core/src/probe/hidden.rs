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
}

/// Whether `text` holds a character [`is_hidden`] names.
pub(crate) fn has_hidden(text: &str) -> bool {
    text.chars().any(is_hidden)
}

/// `text` with every [`is_hidden`] character written out (`\n`, `\t`, `\u{202e}`), so that text a Probe
/// or a folder chose can be printed without driving the terminal it is printed on.
pub(crate) fn reveal(text: &str) -> Cow<'_, str> {
    if !has_hidden(text) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
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
}
