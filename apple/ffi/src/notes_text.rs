//! The notes' text rules from core (`view::notes::paragraphs`), for the Notes tab: which paragraph ⌘↵ sends and what
//! a note's title says. Pure functions; nothing here touches a file or a terminal.

use wormhole_core::view::notes::paragraphs;

/// The paragraph (the lines around the cursor up to the nearest blank lines) that holds the character at
/// `cursor`, a count of UTF-8 bytes into `text`. Empty on a blank line: sending then sends nothing.
#[uniffi::export]
pub fn note_paragraph_at(text: String, cursor: u64) -> String {
    paragraphs::paragraph_at(&text, usize::try_from(cursor).unwrap_or(usize::MAX))
}

/// A note's title: its first non-empty line without the marks of a list or a heading; none when the note is blank.
#[uniffi::export]
pub fn note_title(text: String) -> Option<String> {
    paragraphs::note_title(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paragraph_is_the_lines_around_the_cursor_and_a_blank_line_has_none() {
        let text = "one\ntwo\n\nthree";
        assert_eq!(note_paragraph_at(text.into(), 5), "one\ntwo");
        assert_eq!(note_paragraph_at(text.into(), 8), "");
        assert_eq!(note_paragraph_at(text.into(), 12), "three");
    }

    #[test]
    fn a_title_loses_the_marks_of_a_heading_or_a_list() {
        assert_eq!(note_title("\n## Plan\nbody".into()).as_deref(), Some("Plan"));
        assert_eq!(note_title("  \n".into()), None);
    }
}
