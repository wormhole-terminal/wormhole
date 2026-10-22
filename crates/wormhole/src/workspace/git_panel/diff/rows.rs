//! The diff view's display rows, built once per diff (and again when the
//! layout, tab width or syntax colors change), never per frame: tab expansion
//! and syntax colors from the full old / new files happen here, on the
//! background executor (see `load`).

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use gpui_kit::*;

use super::highlight::Highlights;
use wormhole_core::config::Settings;
use wormhole_core::git::{self, DiffLine, FileDiff, LineKind, SplitRow};
use wormhole_core::tokens::{Appearance, ThemeTokens};

pub(crate) struct RenderedLine {
    pub kind: LineKind,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub text: SharedString,
    pub highlights: Vec<(Range<usize>, HighlightStyle)>,
    pub no_newline: bool,
    /// The line of the file as it is now that the icon over this line opens in the editor: its
    /// number on the new side; for a removed line, where its text used to be.
    pub open_line: Option<u32>,
}

impl RenderedLine {
    /// A line without syntax colors (the Studio's sample).
    pub fn plain(kind: LineKind, old: Option<u32>, new: Option<u32>, text: &'static str) -> Self {
        Self { kind, old, new, text: text.into(), highlights: Vec::new(), no_newline: false, open_line: new }
    }
}

/// One side of a split row. Lines are shared: a context line is on both sides
/// of its row, and it is rendered once.
#[derive(Clone)]
pub(crate) enum Cell {
    Line(Arc<RenderedLine>),
    /// No line on this side here; drawn hatched.
    Filler,
}

#[derive(Clone)]
pub(crate) enum Row {
    Hunk(SharedString),
    Unified(Arc<RenderedLine>),
    Split(Cell, Cell),
}

/// What a set of rows was built with; they are built again when any of it changes.
/// Only what changes the rows is in here: wrapping, the font and every other
/// setting change how rows are drawn, not what they hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct BuildKey {
    /// Side by side: the setting, when the diff has room for it (`limits::diff_split_fits`).
    pub split: bool,
    pub tab_width: usize,
    /// The syntax colors the lines were highlighted with, see [`syntax_fingerprint`].
    pub syntax: u64,
}

impl BuildKey {
    /// The rows for `settings` in a diff that has room for a split view or not (`split_fits`).
    pub(super) fn of(settings: &Settings, split_fits: bool, syntax: u64) -> Self {
        Self { split: settings.diff_split && split_fits, tab_width: settings.tab_width, syntax }
    }
}

/// A fingerprint of the colors and styles code is highlighted with. Not the
/// settings revision, which every change to any setting (and every step of a
/// Studio drag) bumps: rows only need to follow the syntax colors.
pub(super) fn syntax_fingerprint(tokens: &ThemeTokens) -> u64 {
    let mut hasher = DefaultHasher::new();
    // The syntax theme falls back to the light or dark default by appearance.
    (tokens.appearance == Appearance::Light).hash(&mut hasher);
    for (name, style) in &tokens.syntax {
        name.hash(&mut hasher);
        let color = style.color;
        [color.r, color.g, color.b, color.a].map(f32::to_bits).hash(&mut hasher);
        (style.bold, style.italic).hash(&mut hasher);
    }
    hasher.finish()
}

/// What the view needs to size its columns, measured while building.
#[derive(Clone, Copy, Default)]
pub(crate) struct RowStats {
    /// Characters in the longest line of code (after tab expansion).
    pub widest_chars: usize,
    /// Digits in the largest line number, for the gutter width.
    pub number_digits: usize,
}

/// Precompute display rows, colored from the full old / new files when `highlights` is given.
pub(super) fn build_rows(
    diff: &FileDiff,
    key: BuildKey,
    highlights: Option<&Highlights>,
) -> (Vec<Row>, RowStats) {
    // Removed lines have no number in the new file: the nearest one, found per hunk, keyed by the
    // line itself (the split rows hand out references into the hunks).
    let mut nearest: HashMap<*const DiffLine, u32> = HashMap::new();
    for hunk in &diff.hunks {
        for (line, found) in hunk.lines.iter().zip(git::lines_in_new_file(&hunk.lines)) {
            if let (None, Some(found)) = (line.new, found) {
                nearest.insert(line, found);
            }
        }
    }
    let render = |line: &DiffLine| -> Arc<RenderedLine> {
        let expanded = expand_tabs(&line.text, key.tab_width);
        let colors = highlights.map(|h| h.line(line)).unwrap_or_default();
        let highlights = colors
            .into_iter()
            .map(|(range, style)| (expanded.offset(range.start)..expanded.offset(range.end), style))
            .filter(|(range, _)| range.start < range.end)
            .collect();
        Arc::new(RenderedLine {
            kind: line.kind,
            old: line.old,
            new: line.new,
            text: expanded.text.into(),
            highlights,
            no_newline: line.no_newline,
            open_line: line.new.or_else(|| nearest.get(&std::ptr::from_ref(line)).copied()),
        })
    };

    let rows: Vec<Row> = if key.split {
        git::split_rows(diff)
            .into_iter()
            .map(|row| match row {
                SplitRow::Hunk(hunk) => Row::Hunk(hunk.header.clone().into()),
                // A context line sits on both sides of its row: render it once, show it twice.
                SplitRow::Pair(git::Cell::Line(old), git::Cell::Line(new)) if std::ptr::eq(old, new) => {
                    let line = render(old);
                    Row::Split(Cell::Line(line.clone()), Cell::Line(line))
                }
                SplitRow::Pair(old, new) => {
                    let cell = |c: git::Cell| c.line().map_or(Cell::Filler, |l| Cell::Line(render(l)));
                    Row::Split(cell(old), cell(new))
                }
            })
            .collect()
    } else {
        let mut rows = Vec::new();
        for hunk in &diff.hunks {
            rows.push(Row::Hunk(hunk.header.clone().into()));
            rows.extend(hunk.lines.iter().map(|l| Row::Unified(render(l))));
        }
        rows
    };

    let largest = diff.hunks.iter().flat_map(|h| &h.lines).flat_map(|l| [l.old, l.new]).flatten().max();
    let stats = RowStats {
        widest_chars: rows.iter().map(row_chars).max().unwrap_or(0),
        number_digits: largest.unwrap_or(0).to_string().len(),
    };
    (rows, stats)
}

fn row_chars(row: &Row) -> usize {
    let len = |c: &Cell| match c {
        Cell::Line(l) => l.text.chars().count(),
        Cell::Filler => 0,
    };
    match row {
        // Headers are clipped to their column rather than scrolled.
        Row::Hunk(_) => 0,
        Row::Unified(l) => l.text.chars().count(),
        Row::Split(o, n) => len(o).max(len(n)),
    }
}

/// A line's text with its tabs expanded, and the way to carry byte offsets of
/// the original (what the syntax highlighter reports) over to it.
struct Expanded<'a> {
    text: Cow<'a, str>,
    /// The offset in `text` of every byte offset of the original (and of its
    /// end). `None` when nothing moved, which is every line without a tab.
    offsets: Option<Vec<usize>>,
}

impl Expanded<'_> {
    fn offset(&self, original: usize) -> usize {
        match &self.offsets {
            Some(offsets) => offsets.get(original).copied().unwrap_or(self.text.len()),
            None => original.min(self.text.len()),
        }
    }
}

/// Expand tabs to the next tab stop. Almost no line has one, and those that
/// don't are neither copied nor given an offset table (8 bytes per byte of text).
fn expand_tabs(text: &str, tab_width: usize) -> Expanded<'_> {
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
    use std::sync::Arc;

    use super::{BuildKey, Cell, Row, build_rows, expand_tabs, syntax_fingerprint};
    use crate::workspace::git_panel::diff::highlight::Highlights;
    use gpui_kit::component::highlighter::HighlightTheme;
    use wormhole_core::config::Settings;
    use wormhole_core::git::{DiffLine, FileDiff, Hunk, LineKind};
    use wormhole_core::tokens::{self, Color};

    fn line(kind: LineKind, old: Option<u32>, new: Option<u32>, text: &str) -> DiffLine {
        DiffLine { kind, old, new, text: text.into(), no_newline: false }
    }

    /// Two context lines around one replaced line, then a pure addition.
    fn sample_diff() -> FileDiff {
        use LineKind::{Added, Context, Removed};
        let hunk = |header: &str, lines| Hunk { header: header.into(), lines };
        FileDiff {
            hunks: vec![
                hunk(
                    "@@ -1,3 +1,3 @@",
                    vec![
                        line(Context, Some(1), Some(1), "one"),
                        line(Removed, Some(2), None, "two"),
                        line(Added, None, Some(2), "TWO"),
                        line(Context, Some(3), Some(3), "three"),
                    ],
                ),
                hunk("@@ -10,0 +10,1 @@", vec![line(Added, None, Some(10), "ten")]),
            ],
            ..FileDiff::default()
        }
    }

    fn key(split: bool) -> BuildKey {
        BuildKey { split, tab_width: 4, syntax: 0 }
    }

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

    #[test]
    fn context_lines_are_rendered_once_in_split() {
        let (rows, _) = build_rows(&sample_diff(), key(true), None);
        let shared = rows
            .iter()
            .filter(|row| matches!(row, Row::Split(Cell::Line(a), Cell::Line(b)) if Arc::ptr_eq(a, b)))
            .count();
        // "one" and "three"; the replaced pair is two different lines.
        assert_eq!(shared, 2);
        let Some(Row::Split(Cell::Line(old), Cell::Line(new))) = rows.get(2) else {
            panic!("row 2 should be the replaced pair");
        };
        assert!(!Arc::ptr_eq(old, new));
        assert_eq!((&*old.text, &*new.text), ("two", "TWO"));
    }

    #[test]
    fn split_rows_pad_a_pure_addition_with_a_filler() {
        let (rows, _) = build_rows(&sample_diff(), key(true), None);
        assert_eq!(rows.len(), 1 + 3 + 1 + 1);
        assert!(matches!(rows.last(), Some(Row::Split(Cell::Filler, Cell::Line(_)))));
    }

    #[test]
    fn every_line_knows_which_line_of_the_file_to_open() {
        let open_lines = |split| {
            let (rows, _) = build_rows(&sample_diff(), key(split), None);
            let lines: Vec<Option<u32>> = rows
                .iter()
                .flat_map(|row| match row {
                    Row::Unified(line) => vec![line.open_line],
                    // The old side of a replaced pair is the removed line.
                    Row::Split(Cell::Line(old), Cell::Line(new)) => vec![old.open_line, new.open_line],
                    Row::Split(..) | Row::Hunk(_) => Vec::new(),
                })
                .collect();
            lines
        };
        // "two" was removed at old line 2: it opens where "TWO" now is.
        assert_eq!(open_lines(false), [Some(1), Some(2), Some(2), Some(3), Some(10)]);
        assert_eq!(open_lines(true), [Some(1), Some(1), Some(2), Some(2), Some(3), Some(3)]);
    }

    #[test]
    fn unified_rows_are_a_header_then_one_row_per_line() {
        let (rows, stats) = build_rows(&sample_diff(), key(false), None);
        assert_eq!(rows.len(), (1 + 4) + (1 + 1));
        assert!(matches!(rows[0], Row::Hunk(_)));
        assert!(matches!(&rows[2], Row::Unified(l) if &*l.text == "two" && l.kind == LineKind::Removed));
        assert_eq!(stats.widest_chars, "three".len());
        // Line 10 is the largest number.
        assert_eq!(stats.number_digits, 2);
    }

    #[test]
    fn colors_follow_a_tab_through_expansion() {
        let theme = HighlightTheme::default_dark();
        let old = "fn f() {\n\tlet x = 1;\n}\n";
        let diff = FileDiff {
            hunks: vec![Hunk {
                header: "@@".into(),
                lines: vec![line(LineKind::Context, Some(2), Some(2), "\tlet x = 1;")],
            }],
            old_text: Some(old.into()),
            new_text: Some(old.into()),
            ..FileDiff::default()
        };
        let highlights = Highlights::for_diff(&diff, "a.rs", &theme).expect("rust has a grammar");
        let (rows, _) = build_rows(&diff, key(false), Some(&highlights));
        let Row::Unified(rendered) = &rows[1] else { panic!("expected a unified line") };
        assert_eq!(&*rendered.text, "    let x = 1;");
        // `let` is a keyword: its range starts after the four spaces the tab became.
        let keyword =
            rendered.highlights.iter().find(|(range, style)| range.start == 4 && style.color.is_some());
        assert!(keyword.is_some(), "no colored range starts at column 4: {:?}", rendered.highlights);
        // Every range lies inside the expanded text.
        assert!(rendered.highlights.iter().all(|(range, _)| range.end <= rendered.text.len()));
    }

    #[test]
    fn rows_ignore_wrapping_and_unrelated_settings() {
        let tokens = tokens::builtin_tokens().remove(0);
        let syntax = syntax_fingerprint(&tokens);
        let base = Settings::default();
        let key = BuildKey::of(&base, true, syntax);
        // Wrapping and the font size are drawing, not rows: no rebuild.
        let drawn_differently =
            Settings { diff_wrap: !base.diff_wrap, font_size: base.font_size + 2.0, ..base.clone() };
        assert_eq!(BuildKey::of(&drawn_differently, true, syntax), key);
        // Layout and tab width are rows.
        assert_ne!(
            BuildKey::of(&Settings { diff_split: !base.diff_split, ..base.clone() }, true, syntax),
            key
        );
        assert_ne!(
            BuildKey::of(&Settings { tab_width: base.tab_width + 1, ..base.clone() }, true, syntax),
            key
        );
    }

    #[test]
    fn a_diff_without_room_for_two_columns_is_unified_whatever_the_setting() {
        let syntax = 0;
        let split = Settings { diff_split: true, ..Settings::default() };
        assert!(BuildKey::of(&split, true, syntax).split);
        // Narrow: unified, and the choice is kept for when there is room again.
        assert!(!BuildKey::of(&split, false, syntax).split);
        assert!(split.diff_split);
        let unified = Settings { diff_split: false, ..Settings::default() };
        assert!(!BuildKey::of(&unified, true, syntax).split);
    }

    #[test]
    fn only_syntax_colors_change_the_syntax_fingerprint() {
        let tokens = tokens::builtin_tokens().remove(0);
        let before = syntax_fingerprint(&tokens);

        // Anything else about the theme (name, terminal colors, metrics) is not in it.
        let mut elsewhere = tokens.clone();
        elsewhere.name.push_str(" (edited)");
        elsewhere.terminal.background = Color::rgba(0.2, 0.1, 0.3, 1.0);
        elsewhere.metrics.radius += 1.0;
        assert_eq!(syntax_fingerprint(&elsewhere), before);

        let (name, style) = tokens.syntax.iter().next().map(|(n, s)| (n.clone(), *s)).expect("syntax styles");
        let mut recolored = tokens.clone();
        if let Some(entry) = recolored.syntax.get_mut(&name) {
            entry.color = Color::rgba(1.0 - style.color.r, style.color.g, style.color.b, 1.0);
        }
        assert_ne!(syntax_fingerprint(&recolored), before);

        let mut bolder = tokens.clone();
        if let Some(entry) = bolder.syntax.get_mut(&name) {
            entry.bold = !style.bold;
        }
        assert_ne!(syntax_fingerprint(&bolder), before);
    }
}
