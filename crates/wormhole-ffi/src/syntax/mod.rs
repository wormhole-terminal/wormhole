//! Syntax colours for the diff and the Files viewer: a text and its path in, for each line the stretches a theme's
//! `syntax.*` token colours. Tree-sitter parses the whole text (`engine`), with the grammar the file's extension names
//! (`language`); a file without one gets nothing and nothing is parsed.
//!
//! The GPUI app colours through GPUI Kit's highlighter, which is GPUI's own types all the way down, so this is the
//! same rule written for a caller that has no GPUI: the same grammars, the same choice by extension, the same way a
//! capture's name becomes a key and a later capture covers an earlier one. What differs is said in `language`.
//!
//! Content from a repository is hostile input: the text is capped, the parse and the query have a deadline, a line is
//! coloured for as many characters as it is drawn for, and no offset leaves its line.

mod engine;
mod language;

pub(crate) use engine::Colored;

/// One coloured stretch of a line (a "run": Swift's own span counts in Characters). Unicode scalars of the line,
/// never bytes or grapheme clusters, so Swift can follow them whatever the line holds; `key` is an index into
/// [`syntax_keys`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct SyntaxRun {
    pub start: u32,
    pub end: u32,
    pub key: u8,
}

/// The keys a run names, in the order [`SyntaxRun::key`] counts them (`keyword`, `string.escape`): the theme's
/// `syntax.<key>` tokens. Asked once.
#[uniffi::export]
pub fn syntax_keys() -> Vec<String> {
    engine::KEYS.iter().map(|key| (*key).to_string()).collect()
}

/// The colours of `text` read as the language `path` names: one list per line (cut at every line feed), in order.
/// `None` for a file without a grammar, a text over the cap or one that takes too long. Takes long for a large text:
/// not for the main thread.
#[uniffi::export]
pub fn syntax_lines(text: String, path: String) -> Option<Vec<Vec<SyntaxRun>>> {
    let colored = Colored::of(&text, &path)?;
    Some((0..colored.lines()).map(|index| colored.line(index)).collect())
}
