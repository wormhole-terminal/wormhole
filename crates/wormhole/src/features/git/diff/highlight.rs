//! Syntax colors for the diff's lines, taken from the full old / new file (a hunk alone is too little
//! context for tree-sitter). The parsing and its cache are shared with the Files viewer (`syntax`); what is
//! the diff's own is which side a line is colored from.

use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::*;
use wormhole_core::git::{DiffLine, FileDiff, LineKind};

use crate::syntax::highlight::TextHighlights;

type Styles = Vec<(std::ops::Range<usize>, HighlightStyle)>;

/// The syntax colors of a diff's two files.
pub(super) struct Highlights {
    old: Option<TextHighlights>,
    new: Option<TextHighlights>,
    theme: Arc<HighlightTheme>,
}

impl Highlights {
    /// `None` for a file without a grammar (plain text, unknown extensions): its lines get no colors and
    /// nothing is parsed.
    pub(super) fn for_diff(diff: &FileDiff, path: &str, theme: &Arc<HighlightTheme>) -> Option<Self> {
        // The first parse says whether there is a grammar at all: no side is parsed for a file without one.
        let side =
            |text: &Option<String>| text.as_deref().and_then(|text| TextHighlights::for_text(text, path));
        let (old, new) = (side(&diff.old_text), side(&diff.new_text));
        if old.is_none() && new.is_none() {
            return None;
        }
        Some(Self { old, new, theme: theme.clone() })
    }

    /// Colors for one diff line, relative to its text: a removed line from the old file, any other from the
    /// new one (the old one when the new is missing).
    pub(super) fn line(&self, line: &DiffLine) -> Styles {
        let source = match line.kind {
            LineKind::Removed => self.old.as_ref().zip(line.old),
            _ => self.new.as_ref().zip(line.new).or(self.old.as_ref().zip(line.old)),
        };
        let Some((parsed, number)) = source else { return Vec::new() };
        let Some(index) = (number as usize).checked_sub(1) else { return Vec::new() };
        parsed.line(index, &line.text, &self.theme)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::highlighter::HighlightTheme;
    use wormhole_core::git::{DiffLine, FileDiff, LineKind};

    use super::Highlights;

    fn removed(old: u32, text: &str) -> DiffLine {
        DiffLine { kind: LineKind::Removed, old: Some(old), new: None, text: text.into(), no_newline: false }
    }

    #[test]
    fn plain_text_is_not_parsed_at_all() {
        let theme = HighlightTheme::default_dark();
        let diff =
            FileDiff { old_text: Some("a\n".into()), new_text: Some("b\n".into()), ..FileDiff::default() };
        assert!(Highlights::for_diff(&diff, "notes.unknownext", &theme).is_none());
        assert!(Highlights::for_diff(&diff, "Makefile", &theme).is_none());
    }

    #[test]
    fn lines_get_colors_from_their_own_side_and_only_if_the_text_matches() {
        let theme = HighlightTheme::default_dark();
        let diff = FileDiff {
            old_text: Some("fn diff_old() {}\n".into()),
            new_text: Some("fn diff_new() {}\n".into()),
            ..FileDiff::default()
        };
        let highlights = Highlights::for_diff(&diff, "a.rs", &theme).expect("rust has a grammar");
        // A removed line is read from the old file, at its old line number.
        assert!(!highlights.line(&removed(1, "fn diff_old() {}")).is_empty());
        // The line the diff holds is not what the file has there: no colors, no panic.
        assert!(highlights.line(&removed(1, "fn other() {}")).is_empty());
        assert!(highlights.line(&removed(9, "fn diff_old() {}")).is_empty());
        // Line numbers count from 1; 0 must not underflow.
        assert!(highlights.line(&removed(0, "fn diff_old() {}")).is_empty());
    }
}
