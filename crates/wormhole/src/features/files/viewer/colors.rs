//! The syntax colors of a text, made off the main thread: for each line, the spans the highlighter says,
//! for the characters that are drawn. The draw only looks them up, so no highlight query runs where a frame
//! is built. Spans are compact (a style's number and two offsets in the line, 6 bytes) because a file of
//! 100,000 lines has a million of them.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::*;

use super::lines::Lines;
use crate::syntax::highlight::TextHighlights;

/// One colored stretch of a line: byte offsets in the drawn part of the line, and which style.
#[derive(Clone, Copy)]
struct Span {
    start: u16,
    end: u16,
    style: u16,
}

/// The colors of every line of a text, for one syntax theme.
pub(super) struct Colors {
    /// The distinct styles; a theme has a few dozen.
    styles: Vec<HighlightStyle>,
    lines: Vec<Vec<Span>>,
}

/// The identity of a syntax theme, to tell whether colors were made with it.
pub(super) fn theme_key(theme: &Arc<HighlightTheme>) -> usize {
    Arc::as_ptr(theme) as usize
}

impl Colors {
    /// Parses `text` as the language `path` names and colors each line's drawn part with `theme`; `None`
    /// for a file without a grammar. Takes long for a large text: not for the main thread.
    pub(super) fn make(lines: &Lines, path: &str, theme: &Arc<HighlightTheme>) -> Option<Self> {
        let parsed = TextHighlights::for_text(lines.text(), path)?;
        let mut styles: Vec<HighlightStyle> = Vec::new();
        let mut per_line = Vec::with_capacity(lines.len());
        for index in 0..lines.len() {
            // Only what is drawn is asked of the highlighter: a line of two million characters costs the cap.
            let (line, _) = lines.capped(index);
            let spans = parsed
                .line(index, line, theme)
                .into_iter()
                .filter_map(|(range, style)| {
                    // A token that runs past the cut is cut with the line.
                    let range = range.start..range.end.min(line.len());
                    if range.start >= range.end {
                        return None;
                    }
                    let at = styles.iter().position(|known| *known == style).unwrap_or_else(|| {
                        styles.push(style);
                        styles.len() - 1
                    });
                    Some(Span {
                        start: u16::try_from(range.start).ok()?,
                        end: u16::try_from(range.end).ok()?,
                        style: u16::try_from(at).ok()?,
                    })
                })
                .collect();
            per_line.push(spans);
        }
        Some(Self { styles, lines: per_line })
    }

    /// The spans of line `index`, as byte ranges of the line's drawn part (before tabs are expanded).
    pub(super) fn line(&self, index: usize) -> impl Iterator<Item = (Range<usize>, HighlightStyle)> + '_ {
        self.lines.get(index).into_iter().flatten().map(|span| {
            (usize::from(span.start)..usize::from(span.end), self.styles[usize::from(span.style)])
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::highlighter::HighlightTheme;

    use super::Colors;
    use crate::features::files::viewer::lines::{Lines, MAX_ROW_CHARS};

    #[test]
    fn each_line_has_its_spans_and_a_file_without_a_grammar_has_none() {
        let source = "fn colors_test() {\n    let x = 1;\n}\n";
        let lines = Lines::of(source);
        let theme = HighlightTheme::default_dark();
        let colors = Colors::make(&lines, "a.rs", &theme).expect("rust has a grammar");
        assert!(colors.line(1).any(|(range, style)| range.start == 4 && style.color.is_some()), "`let`");
        assert_eq!(colors.line(99).count(), 0);
        assert!(Colors::make(&lines, "notes.txt", &theme).is_none());
    }

    #[test]
    fn nothing_is_colored_past_what_is_drawn() {
        let long = format!("let x = \"{}\";\n", "a".repeat(MAX_ROW_CHARS * 3));
        let lines = Lines::of(&long);
        let colors = Colors::make(&lines, "a.rs", &HighlightTheme::default_dark()).expect("grammar");
        let (drawn, cut) = lines.capped(0);
        assert!(cut);
        assert!(colors.line(0).all(|(range, _)| range.end <= drawn.len()));
    }
}
