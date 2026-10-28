//! A line of code as it is drawn in a card: tabs are spaces up to the next tab stop, and what a person
//! can't see or what would change how the line reads is shown as a visible placeholder. The syntax
//! highlighter's byte offsets (which count each of those as it was in the file) are carried over to the
//! text drawn.
//!
//! The placeholders are what editors show: a control character is its picture from the Control Pictures
//! block (`␛`), and a character that reorders text (a bidirectional override or isolate, the marks, the
//! line and paragraph separators) or a C1 control is `<U+202E>`. Without them a file could read as
//! something it is not ("Trojan Source"), and an escape sequence would be invisible.

use std::borrow::Cow;
use std::ops::Range;

/// A line's text as drawn, and the way to carry byte offsets of the original (what the syntax highlighter
/// reports) over to it.
pub(crate) struct Expanded<'a> {
    pub(crate) text: Cow<'a, str>,
    /// The offset in `text` of every byte offset of the original (and of its end). `None` when nothing
    /// moved, which is every line without a tab or a placeholder.
    pub(crate) offsets: Option<Vec<usize>>,
    /// Where the placeholders are in `text`: drawn dimmed, so they read as marks and not as code.
    pub(crate) marks: Vec<Range<usize>>,
}

impl Expanded<'_> {
    pub(crate) fn offset(&self, original: usize) -> usize {
        match &self.offsets {
            Some(offsets) => offsets.get(original).copied().unwrap_or(self.text.len()),
            None => original.min(self.text.len()),
        }
    }
}

/// How a character that is not drawn as itself is shown: `None` for one that is (and for a tab, which is
/// spaces, not a placeholder).
fn placeholder(ch: char) -> Option<String> {
    match ch {
        '\t' => None,
        // C0 controls and DEL: their pictures.
        '\0'..='\u{1F}' => char::from_u32(0x2400 + u32::from(ch)).map(String::from),
        '\u{7F}' => Some("\u{2421}".to_string()),
        // C1 controls, the marks and embeddings that reorder text, the line and paragraph separators.
        '\u{80}'..='\u{9F}'
        | '\u{061C}'
        | '\u{200E}'
        | '\u{200F}'
        | '\u{2028}'
        | '\u{2029}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2066}'..='\u{2069}' => Some(format!("<U+{:04X}>", u32::from(ch))),
        _ => None,
    }
}

/// `text` as drawn. Almost no line needs anything, and those that don't are neither copied nor given an
/// offset table (8 bytes per byte of text).
pub(crate) fn visible(text: &str, tab_width: usize) -> Expanded<'_> {
    if !text.chars().any(|ch| ch == '\t' || placeholder(ch).is_some()) {
        return Expanded { text: Cow::Borrowed(text), offsets: None, marks: Vec::new() };
    }
    let tab_width = tab_width.max(1);
    let mut out = String::with_capacity(text.len());
    let mut offsets = Vec::with_capacity(text.len() + 1);
    let mut marks = Vec::new();
    let mut column = 0;
    for ch in text.chars() {
        offsets.extend(std::iter::repeat_n(out.len(), ch.len_utf8()));
        if ch == '\t' {
            let spaces = tab_width - column % tab_width;
            out.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else if let Some(mark) = placeholder(ch) {
            let start = out.len();
            out.push_str(&mark);
            marks.push(start..out.len());
            column += mark.chars().count();
        } else {
            out.push(ch);
            column += 1;
        }
    }
    offsets.push(out.len());
    Expanded { text: Cow::Owned(out), offsets: Some(offsets), marks }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::visible;

    #[test]
    fn expands_tabs_to_tab_stops() {
        let expanded = visible("\tif x {\t// c", 4);
        assert_eq!(expanded.text, "    if x {  // c");
        // "if" starts at byte 1 in the input and byte 4 in the output.
        assert_eq!(expanded.offset(1), 4);
        // The second tab (byte 7) sits at column 10 → expands to 2 spaces.
        assert_eq!(expanded.offset(7), 10);
        assert_eq!(expanded.offset(8), 12);
        assert_eq!(expanded.offset("\tif x {\t// c".len()), expanded.text.len());
    }

    #[test]
    fn visible_keeps_multibyte_offsets() {
        let expanded = visible("ä\tb", 4);
        assert_eq!(expanded.text, "ä   b");
        assert_eq!(expanded.offsets.as_ref().map(Vec::len), Some("ä\tb".len() + 1));
        assert_eq!(expanded.offset(3), 5); // 'b'
    }

    #[test]
    fn a_line_without_tabs_is_neither_copied_nor_mapped() {
        // This is every line of most files: no output string, no 8-bytes-per-byte offset table.
        let expanded = visible("let x = 1;", 4);
        assert!(matches!(expanded.text, Cow::Borrowed("let x = 1;")));
        assert!(expanded.offsets.is_none());
        assert_eq!(expanded.offset(4), 4);
        // An offset past the end (a stale highlight range) is clamped, not a panic.
        assert_eq!(expanded.offset(99), "let x = 1;".len());
    }

    #[test]
    fn what_reorders_text_or_cannot_be_seen_is_drawn_as_a_placeholder() {
        // A right-to-left override and an isolate, as in a "Trojan Source" line.
        let shown = visible("if x\u{202E} \u{2066}{", 4);
        assert_eq!(shown.text, "if x<U+202E> <U+2066>{");
        assert_eq!(shown.marks, [4..12, 13..21]);
        // The byte after the override (a space, at byte 7 of the original) is where the placeholder ends.
        assert_eq!(shown.offset(7), 12);
        assert_eq!(shown.offset("if x\u{202E} \u{2066}{".len()), shown.text.len());
        // Controls are their pictures, DEL too; C1 controls and the separators are numbered.
        assert_eq!(visible("a\x1b[31mb\x7f\u{85}\u{2028}", 4).text, "a\u{241b}[31mb\u{2421}<U+0085><U+2028>");
        assert_eq!(visible("\0", 4).text, "\u{2400}");
        for ch in ['\u{061C}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202D}', '\u{2067}', '\u{2069}'] {
            let line = format!("a{ch}b");
            assert!(visible(&line, 4).text.contains("<U+"), "{ch:?} is shown");
        }
    }

    #[test]
    fn text_and_emoji_sequences_are_left_alone() {
        // A zero width joiner is part of an emoji; a combining mark and a CJK character are text.
        for line in ["👨\u{200D}👩", "e\u{301}", "日本語", "ok\u{A0}x"] {
            let shown = visible(line, 4);
            assert!(matches!(shown.text, Cow::Borrowed(_)) && shown.marks.is_empty(), "{line:?}");
        }
    }

    #[test]
    fn a_tab_after_a_placeholder_counts_the_placeholders_columns() {
        // `<U+202E>` is 8 columns: the next tab stop (width 4) is 4 further on.
        assert_eq!(visible("\u{202E}\tx", 4).text, "<U+202E>    x");
    }
}
