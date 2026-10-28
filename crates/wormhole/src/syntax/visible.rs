//! Tab expansion for code drawn in a card: a tab is spaces up to the next tab stop, and the syntax
//! highlighter's byte offsets (which count the tab as one byte) are carried over to the expanded text.

use std::borrow::Cow;

/// A line's text with its tabs expanded, and the way to carry byte offsets of
/// the original (what the syntax highlighter reports) over to it.
pub(crate) struct Expanded<'a> {
    pub(crate) text: Cow<'a, str>,
    /// The offset in `text` of every byte offset of the original (and of its
    /// end). `None` when nothing moved, which is every line without a tab.
    pub(crate) offsets: Option<Vec<usize>>,
}

impl Expanded<'_> {
    pub(crate) fn offset(&self, original: usize) -> usize {
        match &self.offsets {
            Some(offsets) => offsets.get(original).copied().unwrap_or(self.text.len()),
            None => original.min(self.text.len()),
        }
    }
}

/// Expand tabs to the next tab stop. Almost no line has one, and those that
/// don't are neither copied nor given an offset table (8 bytes per byte of text).
pub(crate) fn expand_tabs(text: &str, tab_width: usize) -> Expanded<'_> {
    if !text.contains('\t') {
        return Expanded { text: Cow::Borrowed(text), offsets: None };
    }
    let tab_width = tab_width.max(1);
    let mut out = String::with_capacity(text.len());
    let mut offsets = Vec::with_capacity(text.len() + 1);
    let mut column = 0;
    for ch in text.chars() {
        offsets.extend(std::iter::repeat_n(out.len(), ch.len_utf8()));
        if ch == '\t' {
            let spaces = tab_width - column % tab_width;
            out.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else {
            out.push(ch);
            column += 1;
        }
    }
    offsets.push(out.len());
    Expanded { text: Cow::Owned(out), offsets: Some(offsets) }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::expand_tabs;

    #[test]
    fn expands_tabs_to_tab_stops() {
        let expanded = expand_tabs("\tif x {\t// c", 4);
        assert_eq!(expanded.text, "    if x {  // c");
        // "if" starts at byte 1 in the input and byte 4 in the output.
        assert_eq!(expanded.offset(1), 4);
        // The second tab (byte 7) sits at column 10 → expands to 2 spaces.
        assert_eq!(expanded.offset(7), 10);
        assert_eq!(expanded.offset(8), 12);
        assert_eq!(expanded.offset("\tif x {\t// c".len()), expanded.text.len());
    }

    #[test]
    fn expand_tabs_keeps_multibyte_offsets() {
        let expanded = expand_tabs("ä\tb", 4);
        assert_eq!(expanded.text, "ä   b");
        assert_eq!(expanded.offsets.as_ref().map(Vec::len), Some("ä\tb".len() + 1));
        assert_eq!(expanded.offset(3), 5); // 'b'
    }

    #[test]
    fn a_line_without_tabs_is_neither_copied_nor_mapped() {
        // This is every line of most files: no output string, no 8-bytes-per-byte offset table.
        let expanded = expand_tabs("let x = 1;", 4);
        assert!(matches!(expanded.text, Cow::Borrowed("let x = 1;")));
        assert!(expanded.offsets.is_none());
        assert_eq!(expanded.offset(4), 4);
        // An offset past the end (a stale highlight range) is clamped, not a panic.
        assert_eq!(expanded.offset(99), "let x = 1;".len());
    }
}
