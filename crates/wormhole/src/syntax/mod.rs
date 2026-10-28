//! Syntax colours for code shown in a card: tree-sitter parses and a shared cache keeps the parsed texts
//! (`highlight`), and a line is made drawable (`visible`: tabs, and placeholders for what can't be seen). The
//! diff view and the Files viewer both draw from it; it knows no feature.
//!
//! It uses nothing of the app beyond GPUI Kit's highlighter, so it sits with `fonts`, below `style`.

pub(crate) mod highlight;
pub(crate) mod visible;
