//! The text of a file as lines, without a copy per line: one string and where each line starts, so a
//! file of 100,000 lines costs an index, and a row is made only for a line that is on screen.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::*;

use crate::syntax::highlight::TextHighlights;
use crate::syntax::tabs::expand_tabs;

/// The lines of a text: it is cut at `\n`, and a `\r` before it is not part of the line.
pub(super) struct Lines {
    text: Arc<str>,
    /// Where each line starts. Empty for an empty text.
    starts: Vec<usize>,
    /// The characters in the longest line, before tabs are expanded: how far the code scrolls sideways.
    widest: usize,
}

impl Lines {
    /// Indexes `text`. Walks all of it once: not for a frame.
    pub(super) fn of(text: &str) -> Self {
        let mut starts = Vec::new();
        let mut widest = 0;
        let mut at = 0;
        for line in text.split_inclusive('\n') {
            starts.push(at);
            at += line.len();
            widest = widest.max(line.trim_end_matches(['\n', '\r']).chars().count());
        }
        Self { text: text.into(), starts, widest }
    }

    pub(super) fn len(&self) -> usize {
        self.starts.len()
    }

    pub(super) fn widest(&self) -> usize {
        self.widest
    }

    /// The text of line `index` (0-based) without its break.
    pub(super) fn get(&self, index: usize) -> &str {
        let Some(&start) = self.starts.get(index) else { return "" };
        let end = self.starts.get(index + 1).copied().unwrap_or(self.text.len());
        self.text[start..end].trim_end_matches(['\n', '\r'])
    }

    /// The whole text, for the colors to parse.
    pub(super) fn text(&self) -> &Arc<str> {
        &self.text
    }
}

/// A line ready to draw: its number, its text with tabs expanded, and its syntax colors.
pub(super) struct Row {
    pub number: usize,
    pub text: SharedString,
    pub colors: Vec<(Range<usize>, HighlightStyle)>,
}

/// How many rows are kept for the next frame: about a hundred screens.
const KEPT: usize = 4_000;

/// The rows made for the frames drawn, so scrolling back over a line (and every frame while it stays in
/// view) does not tab-expand and colour it again. Made for one theme and tab width; asking with others
/// starts it over.
#[derive(Default)]
pub(super) struct RowCache {
    made: RefCell<Made>,
}

#[derive(Default)]
struct Made {
    /// The syntax theme (by identity), the tab width, and whether colours were there.
    key: Option<(usize, usize, bool)>,
    rows: HashMap<usize, Rc<Row>>,
}

impl RowCache {
    /// Row `index` of `lines`.
    pub(super) fn row(
        &self,
        index: usize,
        lines: &Lines,
        colors: Option<&TextHighlights>,
        theme: &Arc<HighlightTheme>,
        tab_width: usize,
    ) -> Rc<Row> {
        let key = (Arc::as_ptr(theme) as usize, tab_width, colors.is_some());
        let mut made = self.made.borrow_mut();
        if made.key != Some(key) || made.rows.len() > KEPT {
            made.key = Some(key);
            made.rows.clear();
        }
        if let Some(row) = made.rows.get(&index) {
            return row.clone();
        }
        let line = lines.get(index);
        let expanded = expand_tabs(line, tab_width);
        let colors = colors
            .map(|colors| colors.line(index, line, theme))
            .unwrap_or_default()
            .into_iter()
            .map(|(range, style)| (expanded.offset(range.start)..expanded.offset(range.end), style))
            .filter(|(range, _)| range.start < range.end)
            .collect();
        let row = Rc::new(Row { number: index + 1, text: expanded.text.into_owned().into(), colors });
        made.rows.insert(index, row.clone());
        row
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::highlighter::HighlightTheme;

    use super::{Lines, RowCache};
    use crate::syntax::highlight::TextHighlights;

    #[test]
    fn lines_are_cut_at_breaks_and_a_last_break_adds_none() {
        let lines = Lines::of("one\r\ntwo\n\nlast");
        assert_eq!(lines.len(), 4);
        assert_eq!((lines.get(0), lines.get(1), lines.get(2), lines.get(3)), ("one", "two", "", "last"));
        assert_eq!(lines.get(9), "");
        assert_eq!(Lines::of("a\n").len(), 1);
        assert_eq!(Lines::of("").len(), 0);
        assert_eq!(Lines::of("\n").len(), 1);
        assert_eq!(lines.widest(), 4);
    }

    #[test]
    fn a_row_has_its_tabs_expanded_and_its_colors_moved_with_them() {
        let source = "fn f() {\n\tlet x = 1;\n}\n";
        let lines = Lines::of(source);
        let colors = TextHighlights::for_text(source, "a.rs");
        let theme = HighlightTheme::default_dark();
        let cache = RowCache::default();
        let row = cache.row(1, &lines, colors.as_ref(), &theme, 4);
        assert_eq!((row.number, &*row.text), (2, "    let x = 1;"));
        assert!(row.colors.iter().any(|(range, style)| range.start == 4 && style.color.is_some()));
        assert!(row.colors.iter().all(|(range, _)| range.end <= row.text.len()));
        // The same row again is the one made, until the tab width or the theme is another.
        assert!(std::rc::Rc::ptr_eq(&cache.row(1, &lines, colors.as_ref(), &theme, 4), &row));
        let wider = cache.row(1, &lines, colors.as_ref(), &theme, 8);
        assert_eq!(&*wider.text, "        let x = 1;");
    }
}
