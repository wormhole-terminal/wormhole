//! The syntax colours of a diff's lines, taken from the full old and new file (a hunk alone is too little context
//! for tree-sitter), as the GPUI app takes them (`features/git/diff/highlight.rs`): which side a line is coloured
//! from is the diff's own, the parse is `syntax`'s.

use wormhole_core::git::{DiffLine, FileDiff, LineKind};

use crate::syntax::{Colored, SyntaxRun};

/// The colours of a diff's two files. A side core did not read (missing, binary, over its limit, a diff too long to
/// colour) and a file without a grammar have none, and nothing is parsed for them.
pub(super) struct DiffColors<'a> {
    old: Option<Colored<'a>>,
    new: Option<Colored<'a>>,
}

impl<'a> DiffColors<'a> {
    pub(super) fn of(diff: &'a FileDiff, path: &str) -> Self {
        let side = |text: &'a Option<String>| text.as_deref().and_then(|text| Colored::of(text, path));
        Self { old: side(&diff.old_text), new: side(&diff.new_text) }
    }

    /// The spans of one diff line: a removed line from the old file, any other from the new one (the old one when
    /// the new is missing). None when the file does not have the line's text at its number.
    pub(super) fn line(&self, line: &DiffLine) -> Vec<SyntaxRun> {
        let source = match line.kind {
            LineKind::Removed => self.old.as_ref().zip(line.old),
            _ => self.new.as_ref().zip(line.new).or(self.old.as_ref().zip(line.old)),
        };
        let Some((colored, number)) = source else { return Vec::new() };
        // Line numbers count from 1; a 0 must not underflow.
        let Some(index) = (number as usize).checked_sub(1) else { return Vec::new() };
        colored.line_of(index, &line.text)
    }
}
