//! The text of a file as lines, without a copy per line: one string and where each line starts, so a
//! file of 100,000 lines costs an index, and a row is made only for a line that is on screen.
//!
//! A line is drawn for at most [`MAX_ROW_CHARS`] characters, with an elision mark after them: a minified
//! bundle or a file with no line breaks but `\r` is one line of up to 2 MB, and shaping that every frame
//! would freeze the window. Nothing here runs the syntax highlighter: the spans are made in the
//! background (`colors`) and only looked up when a row is made.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::*;

use super::colors::Colors;
use crate::syntax::visible::visible;

/// The most characters of a line that are drawn (and parsed for colors); the rest is an elision mark.
pub(super) const MAX_ROW_CHARS: usize = 5_000;
/// The elision mark after a line that was cut.
const ELISION: char = '…';

/// The lines of a text: it is cut at `\n`, and a `\r` before it is not part of the line.
pub(super) struct Lines {
    text: Arc<str>,
    /// Where each line starts. Empty for an empty text.
    starts: Vec<usize>,
    /// The characters in the longest line as drawn (so at most one more than the cap, for the elision),
    /// before tabs are expanded: how far the code scrolls sideways.
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
            let shown = line.trim_end_matches(['\n', '\r']).chars().take(MAX_ROW_CHARS + 1).count();
            widest = widest.max(shown);
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

    /// What of line `index` is drawn: its first [`MAX_ROW_CHARS`] characters, and whether there was more.
    pub(super) fn capped(&self, index: usize) -> (&str, bool) {
        let line = self.get(index);
        match line.char_indices().nth(MAX_ROW_CHARS) {
            Some((end, _)) => (&line[..end], true),
            None => (line, false),
        }
    }

    /// The whole text, for the colors to parse.
    pub(super) fn text(&self) -> &Arc<str> {
        &self.text
    }
}

/// A line ready to draw: its number, its text as drawn, and its colors (the syntax's, and the dimming of
/// placeholders and the elision mark).
pub(super) struct Row {
    pub number: usize,
    pub text: SharedString,
    pub colors: Vec<(Range<usize>, HighlightStyle)>,
}

/// How many rows are kept for the next frame: about a hundred screens.
const KEPT: usize = 4_000;

/// The rows made for the frames drawn, so scrolling back over a line (and every frame while it stays in
/// view) does not expand and colour it again. Made for one set of colors, tab width and dim color; asking
/// with others starts it over.
#[derive(Default)]
pub(super) struct RowCache {
    made: RefCell<Made>,
}

#[derive(Default)]
struct Made {
    key: Option<(usize, usize, [u32; 4])>,
    rows: HashMap<usize, Rc<Row>>,
}

impl RowCache {
    /// Row `index` of `lines`; `dim` is the color of what stands in the text's place.
    pub(super) fn row(
        &self,
        index: usize,
        lines: &Lines,
        colors: Option<&Colors>,
        tab_width: usize,
        dim: Hsla,
    ) -> Rc<Row> {
        let key = (
            colors.map_or(0, |colors| std::ptr::from_ref(colors) as usize),
            tab_width,
            [dim.h, dim.s, dim.l, dim.a].map(f32::to_bits),
        );
        let mut made = self.made.borrow_mut();
        if made.key != Some(key) || made.rows.len() > KEPT {
            made.key = Some(key);
            made.rows.clear();
        }
        if let Some(row) = made.rows.get(&index) {
            return row.clone();
        }
        let row = Rc::new(make(index, lines, colors, tab_width, dim));
        made.rows.insert(index, row.clone());
        row
    }
}

fn make(index: usize, lines: &Lines, colors: Option<&Colors>, tab_width: usize, dim: Hsla) -> Row {
    let (line, cut) = lines.capped(index);
    let expanded = visible(line, tab_width);
    let spans: Vec<(Range<usize>, HighlightStyle)> = colors
        .into_iter()
        .flat_map(|colors| colors.line(index))
        .map(|(range, style)| (expanded.offset(range.start)..expanded.offset(range.end), style))
        .filter(|(range, _)| range.start < range.end)
        .collect();
    let mut marks = expanded.marks.clone();
    let mut text = expanded.text.into_owned();
    if cut {
        let start = text.len();
        text.push(ELISION);
        marks.push(start..text.len());
    }
    let dimmed = HighlightStyle { color: Some(dim), ..HighlightStyle::default() };
    Row { number: index + 1, text: text.into(), colors: overlay(spans, &marks, dimmed) }
}

/// `spans` with the `marks` drawn in `dimmed`: a span does not color a mark, which stands where the
/// character it replaces was. Both are sorted and apart.
fn overlay(
    spans: Vec<(Range<usize>, HighlightStyle)>,
    marks: &[Range<usize>],
    dimmed: HighlightStyle,
) -> Vec<(Range<usize>, HighlightStyle)> {
    if marks.is_empty() {
        return spans;
    }
    let mut out = Vec::with_capacity(spans.len() + marks.len());
    for (range, style) in spans {
        let mut start = range.start;
        for mark in marks.iter().filter(|mark| mark.start < range.end && mark.end > range.start) {
            if mark.start > start {
                out.push((start..mark.start, style));
            }
            start = start.max(mark.end);
        }
        if start < range.end {
            out.push((start..range.end, style));
        }
    }
    out.extend(marks.iter().map(|mark| (mark.clone(), dimmed)));
    out.sort_by_key(|(range, _)| range.start);
    out
}

#[cfg(test)]
mod tests {
    use gpui_kit::Hsla;

    use super::{Lines, MAX_ROW_CHARS, RowCache};

    const DIM: Hsla = Hsla { h: 0.0, s: 0.0, l: 0.5, a: 1.0 };

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
    fn a_row_has_its_tabs_expanded() {
        let lines = Lines::of("fn f() {\n\tlet x = 1;\n}\n");
        let cache = RowCache::default();
        let row = cache.row(1, &lines, None, 4, DIM);
        assert_eq!((row.number, &*row.text), (2, "    let x = 1;"));
        // The same row again is the one made, until the tab width is another.
        assert!(std::rc::Rc::ptr_eq(&cache.row(1, &lines, None, 4, DIM), &row));
        assert_eq!(&*cache.row(1, &lines, None, 8, DIM).text, "        let x = 1;");
    }

    #[test]
    fn one_very_long_line_is_drawn_for_a_few_thousand_characters_only() {
        // A minified bundle, or a file with no breaks but `\r`: one line of two million characters.
        let long = "x".repeat(2_000_000);
        let lines = Lines::of(&long);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines.widest(), MAX_ROW_CHARS + 1, "the scroll range is of what is drawn");
        let row = RowCache::default().row(0, &lines, None, 4, DIM);
        assert_eq!(row.text.chars().count(), MAX_ROW_CHARS + 1, "the cap and the elision mark");
        assert!(row.text.ends_with('…'));
        // The elision mark is dimmed; nothing is colored past the cap.
        let (range, _) = row.colors.last().expect("the mark's color");
        assert_eq!(range.end, row.text.len());
        // A line of exactly the cap is not cut.
        let exact = Lines::of(&"y".repeat(MAX_ROW_CHARS));
        assert!(!exact.capped(0).1);
        assert!(Lines::of(&"y".repeat(MAX_ROW_CHARS + 1)).capped(0).1);
        // Characters, not bytes: a cut never lands inside one.
        let wide = Lines::of(&"é".repeat(MAX_ROW_CHARS + 10));
        assert_eq!(wide.capped(0).0.chars().count(), MAX_ROW_CHARS);
    }

    #[test]
    fn a_bidirectional_override_is_drawn_as_a_visible_dimmed_placeholder() {
        let source = "if x\u{202E} \u{2066}{";
        let lines = Lines::of(source);
        let row = RowCache::default().row(0, &lines, None, 4, DIM);
        assert_eq!(&*row.text, "if x<U+202E> <U+2066>{");
        assert!(
            !row.text.contains('\u{202E}') && !row.text.contains('\u{2066}'),
            "nothing reorders the line"
        );
        let dimmed: Vec<_> = row.colors.iter().map(|(range, style)| (range.clone(), style.color)).collect();
        assert_eq!(dimmed, [(4..12, Some(DIM)), (13..21, Some(DIM))]);
    }
}
