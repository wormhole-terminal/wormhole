//! One terminal row turned into what the painter needs: its words with style
//! runs, background, underline and strikethrough spans, and the cells painted as shapes.
//!
//! This runs for every changed row, so it does one pass over the cells with no
//! allocation per cell: characters go straight into the segment's text and a
//! new run starts only where the style changes (a small `Copy` key). Blanks are
//! never shaped or painted: they end a segment, so a segment is one word, which
//! `text` shapes once and reuses wherever it shows up again.

use gpui_kit::Hsla;
use wormhole_core::alacritty_terminal::term::cell::{Cell, Flags};
use wormhole_core::alacritty_terminal::term::color::Colors;
use wormhole_core::terminal::images::{Placeholder, placeholder_at};

use crate::palette::Palette;

/// The text attributes runs are split on. Underlines aren't among them: the
/// terminal paints those itself (see [`Underline`]), and strikethroughs in rows too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Style {
    pub fg: Hsla,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
}

impl Style {
    /// A cell's attributes, drawn in `fg`.
    pub fn of(cell: &Cell, fg: Hsla) -> Self {
        let flags = cell.flags;
        Style {
            fg,
            bold: flags.contains(Flags::BOLD),
            italic: flags.contains(Flags::ITALIC),
            strike: flags.contains(Flags::STRIKEOUT),
        }
    }
}

/// The underline styles of SGR 4:1–4:5 (`4`, `21`, `4:3`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnderlineKind {
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

impl UnderlineKind {
    fn of(flags: Flags) -> Option<Self> {
        // alacritty sets exactly one of these; the order only matters for a corrupt cell.
        [
            (Flags::UNDERCURL, Self::Curly),
            (Flags::DOUBLE_UNDERLINE, Self::Double),
            (Flags::DOTTED_UNDERLINE, Self::Dotted),
            (Flags::DASHED_UNDERLINE, Self::Dashed),
            (Flags::UNDERLINE, Self::Single),
        ]
        .into_iter()
        .find_map(|(flag, kind)| flags.contains(flag).then_some(kind))
    }
}

/// An underline across `[first, end)` columns. Painted by the terminal rather than
/// the text shaper: GPUI's text runs have only straight and wavy underlines, in the
/// text's color, while programs also ask for double, dotted and dashed ones in a
/// color of their own (SGR 58), and an underline under blanks must not need text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Underline {
    pub first: usize,
    pub end: usize,
    pub kind: UnderlineKind,
    pub color: Hsla,
}

/// Consecutive non-blank cells shaped as one word. All narrow or all wide, so each
/// glyph can be pinned to a fixed advance of one or two cells.
#[derive(Debug, PartialEq)]
pub(super) struct Segment {
    pub col: usize,
    pub wide: bool,
    pub text: String,
    /// Byte lengths of `text`, one per style.
    pub runs: Vec<(usize, Style)>,
    /// The column after the last cell in the segment.
    end: usize,
}

impl Segment {
    /// How many cells the segment covers.
    pub fn cells(&self) -> usize {
        self.end - self.col
    }

    /// The characters in `bytes` of the text as a segment of their own, `first_cell` cells in.
    /// Only for a segment whose every character has its own cells (no combining marks).
    pub fn part(&self, bytes: std::ops::Range<usize>, first_cell: usize) -> Segment {
        let mut runs = Vec::new();
        let mut at = 0;
        for (len, style) in &self.runs {
            let (start, end) = (at.max(bytes.start), (at + len).min(bytes.end));
            at += len;
            if start < end {
                runs.push((end - start, *style));
            }
        }
        let text = self.text[bytes].to_string();
        let col = self.col + first_cell;
        let end = col + text.chars().count() * if self.wide { 2 } else { 1 };
        Segment { col, wide: self.wide, text, runs, end }
    }
}

#[derive(Debug, Default, PartialEq)]
pub(super) struct BuiltRow {
    pub segments: Vec<Segment>,
    /// `[first, end)` columns with a background other than the default, merged.
    pub backgrounds: Vec<(usize, usize, Hsla)>,
    /// Underlined spans, merged where kind and color continue.
    pub underlines: Vec<Underline>,
    /// `[first, end)` columns struck through, in a color, merged.
    pub strikes: Vec<(usize, usize, Hsla)>,
    /// Box-drawing and block-element cells, painted as shapes (see `glyphs`).
    pub shapes: Vec<(usize, char, Hsla)>,
    /// The tiles of inline images on the row, consecutive ones as one run (see `images`).
    pub images: Vec<ImageRun>,
}

/// Consecutive cells of one row showing consecutive tiles of one placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ImageRun {
    pub col: usize,
    pub cells: usize,
    pub image: u32,
    pub placement: u32,
    /// The first cell's tile.
    pub tile_row: u16,
    pub tile_col: u16,
}

impl ImageRun {
    /// Whether the placeholder at `col` continues this run.
    fn continues(&self, col: usize, p: &Placeholder) -> bool {
        self.col + self.cells == col
            && self.image == p.image
            && self.placement == p.placement
            && self.tile_row == p.row
            && u32::from(self.tile_col) + self.cells as u32 == u32::from(p.col)
    }
}

/// What colors a row's cells beyond their own: the selection and find's matches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Marks {
    /// The selected columns `[first, last]`.
    pub selected: Option<(usize, usize)>,
    /// Matches' columns `[first, last]`, in order; `true` on the current match.
    pub found: Vec<(usize, usize, bool)>,
}

impl Marks {
    /// Whether `col` is in a match, and in the current one.
    fn found_at(&self, col: usize) -> Option<bool> {
        self.found.iter().find(|(first, last, _)| (*first..=*last).contains(&col)).map(|m| m.2)
    }
}

/// Characters painted as shapes so they join across cells and rows.
pub(super) fn is_shape(c: char) -> bool {
    ('\u{2500}'..='\u{259F}').contains(&c)
}

/// Build one row with what `marks` colors on it.
pub(super) fn build_row(cells: &[Cell], palette: &Palette, overrides: &Colors, marks: &Marks) -> BuiltRow {
    let mut row = BuiltRow::default();
    for (col, cell) in cells.iter().enumerate() {
        if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
            continue;
        }
        let in_selection = marks.selected.is_some_and(|(first, last)| (first..=last).contains(&col));
        let (mut fg, mut bg) = palette.cell_colors(cell, in_selection, overrides);
        // The current match stands out over everything; the others tint, except in the selection.
        let found = marks.found_at(col);
        match found {
            Some(true) => (fg, bg) = (palette.find_current_text, palette.find_current),
            Some(false) if !in_selection => bg = palette.found_background(bg),
            _ => {}
        }
        let wide = cell.flags.contains(Flags::WIDE_CHAR);
        let end = col + if wide { 2 } else { 1 };
        if bg != palette.background {
            match row.backgrounds.last_mut() {
                Some((_, last_end, color)) if *last_end == col && *color == bg => *last_end = end,
                _ => row.backgrounds.push((col, end, bg)),
            }
        }

        // An image's placeholder draws its tile, not a character (its background still paints).
        if let Some(placeholder) = placeholder_at(cell) {
            match row.images.last_mut() {
                Some(run) if run.continues(col, &placeholder) => run.cells += 1,
                _ => row.images.push(ImageRun {
                    col,
                    cells: 1,
                    image: placeholder.image,
                    placement: placeholder.placement,
                    tile_row: placeholder.row,
                    tile_col: placeholder.col,
                }),
            }
            continue;
        }
        let shown = if !wide && is_shape(cell.c) {
            row.shapes.push((col, cell.c, fg));
            ' '
        } else if cell.c == '\0' || cell.c == '\t' {
            ' '
        } else {
            cell.c
        };
        // A hidden cell (SGR 8) hides its underline too.
        if let Some(kind) = UnderlineKind::of(cell.flags).filter(|_| !cell.flags.contains(Flags::HIDDEN)) {
            let color = cell
                .underline_color()
                .filter(|_| !in_selection || palette.selection_fg.is_none())
                .map_or(fg, |color| palette.resolve(color, overrides));
            match row.underlines.last_mut() {
                Some(last) if last.end == col && last.kind == kind && last.color == color => last.end = end,
                _ => row.underlines.push(Underline { first: col, end, kind, color }),
            }
        }
        let mut style = Style::of(cell, fg);
        style.bold |= found == Some(true);
        if style.strike {
            match row.strikes.last_mut() {
                Some((_, last_end, color)) if *last_end == col && *color == fg => *last_end = end,
                _ => row.strikes.push((col, end, fg)),
            }
        }
        let zerowidth = cell.zerowidth();
        // A blank draws nothing: it ends the word before it, and the next
        // character starts one at its own column.
        if shown == ' ' && zerowidth.is_none() {
            continue;
        }

        let continues = row.segments.last().is_some_and(|seg| seg.wide == wide && seg.end == col);
        if !continues {
            row.segments.push(Segment { col, wide, text: String::new(), runs: Vec::new(), end: col });
        }
        let Some(segment) = row.segments.last_mut() else { continue };
        let start = segment.text.len();
        segment.text.push(shown);
        if let Some(zerowidth) = zerowidth {
            segment.text.extend(zerowidth);
        }
        segment.end = end;
        let len = segment.text.len() - start;
        match segment.runs.last_mut() {
            Some((run_len, run_style)) if *run_style == style => *run_len += len,
            _ => segment.runs.push((len, style)),
        }
    }
    row
}

#[cfg(test)]
mod tests {
    use gpui_kit::Hsla;
    use wormhole_core::alacritty_terminal::term::cell::{Cell, Flags};
    use wormhole_core::alacritty_terminal::term::color::Colors;
    use wormhole_core::alacritty_terminal::vte::ansi::{Color, NamedColor};
    use wormhole_core::theme;

    use super::{BuiltRow, ImageRun, Marks, UnderlineKind, build_row};
    use crate::palette::Palette;

    fn palette() -> Palette {
        Palette::from_theme(&theme::builtin()[0])
    }

    fn cells(text: &str) -> Vec<Cell> {
        let mut cells = Vec::new();
        for c in text.chars() {
            if matches!(c, '日' | '本') {
                cells.push(Cell { c, flags: Flags::WIDE_CHAR, ..Cell::default() });
                cells.push(Cell { c: ' ', flags: Flags::WIDE_CHAR_SPACER, ..Cell::default() });
            } else {
                cells.push(Cell { c, ..Cell::default() });
            }
        }
        cells
    }

    fn colored(mut cells: Vec<Cell>, range: std::ops::Range<usize>, color: NamedColor) -> Vec<Cell> {
        for cell in &mut cells[range] {
            cell.fg = Color::Named(color);
        }
        cells
    }

    fn build(cells: &[Cell]) -> BuiltRow {
        build_row(cells, &palette(), &Colors::default(), &Marks::default())
    }

    fn texts(row: &BuiltRow) -> Vec<(usize, bool, &str)> {
        row.segments.iter().map(|s| (s.col, s.wide, s.text.as_str())).collect()
    }

    #[test]
    fn three_colors_are_three_runs_of_one_segment() {
        let row = cells("abcdefghi");
        let row = colored(colored(row, 0..3, NamedColor::Red), 6..9, NamedColor::Blue);
        let built = build(&row);
        assert_eq!(texts(&built), [(0, false, "abcdefghi")]);
        let lens: Vec<usize> = built.segments[0].runs.iter().map(|(len, _)| *len).collect();
        assert_eq!(lens, [3, 3, 3]);
        let palette = palette();
        assert_eq!(built.segments[0].runs[0].1.fg, palette.ansi[1]);
        assert_eq!(built.segments[0].runs[1].1.fg, palette.foreground);
    }

    #[test]
    fn blanks_are_never_shaped() {
        let built = build(&cells("    indented code      "));
        assert_eq!(texts(&built), [(4, false, "indented"), (13, false, "code")]);
        assert!(build(&cells("        ")).segments.is_empty(), "a blank row shapes nothing");
    }

    #[test]
    fn strikethroughs_are_spans_of_their_own() {
        // `ESC[9m` + spaces: a strikethrough with no text still draws, without shaping blanks.
        let mut row = cells("ab    c");
        for cell in &mut row[1..] {
            cell.flags = Flags::STRIKEOUT;
        }
        let built = build(&row);
        assert_eq!(texts(&built), [(0, false, "ab"), (6, false, "c")]);
        assert_eq!(built.strikes, [(1, 7, palette().foreground)]);
    }

    #[test]
    fn underlines_are_spans_of_their_own() {
        // `ESC[4m` + spaces: an underline with no text still draws, without shaping blanks.
        let mut row = cells("ab    cd");
        for cell in &mut row[2..6] {
            cell.flags = Flags::UNDERLINE;
        }
        row[6].flags = Flags::UNDERCURL;
        row[7].flags = Flags::UNDERCURL;
        row[7].set_underline_color(Some(Color::Named(NamedColor::Red)));
        let palette = palette();
        let built = build(&row);
        assert_eq!(texts(&built), [(0, false, "ab"), (6, false, "cd")]);
        let mut trailing = cells("ab    ");
        for cell in &mut trailing[2..] {
            cell.flags = Flags::UNDERLINE;
        }
        let trailing = build(&trailing);
        assert_eq!(texts(&trailing), [(0, false, "ab")], "underlined blanks at the end aren't shaped");
        assert_eq!((trailing.underlines[0].first, trailing.underlines[0].end), (2, 6));
        let spans: Vec<(usize, usize, UnderlineKind, Hsla)> =
            built.underlines.iter().map(|u| (u.first, u.end, u.kind, u.color)).collect();
        assert_eq!(
            spans,
            [
                (2, 6, UnderlineKind::Single, palette.foreground),
                (6, 7, UnderlineKind::Curly, palette.foreground),
                (7, 8, UnderlineKind::Curly, palette.ansi[1]),
            ]
        );
    }

    #[test]
    fn each_underline_style_is_told_apart() {
        let kinds = [
            (Flags::UNDERLINE, UnderlineKind::Single),
            (Flags::DOUBLE_UNDERLINE, UnderlineKind::Double),
            (Flags::UNDERCURL, UnderlineKind::Curly),
            (Flags::DOTTED_UNDERLINE, UnderlineKind::Dotted),
            (Flags::DASHED_UNDERLINE, UnderlineKind::Dashed),
        ];
        for (flag, kind) in kinds {
            let mut row = cells("x");
            row[0].flags = flag;
            assert_eq!(build(&row).underlines[0].kind, kind);
        }
        let mut hidden = cells("x");
        hidden[0].flags = Flags::UNDERLINE | Flags::HIDDEN;
        assert!(build(&hidden).underlines.is_empty());
    }

    #[test]
    fn box_characters_are_shapes_and_blank_in_the_text() {
        let built = build(&cells("│ ok │"));
        assert_eq!(texts(&built), [(2, false, "ok")]);
        let shapes: Vec<(usize, char)> = built.shapes.iter().map(|(col, c, _)| (*col, *c)).collect();
        assert_eq!(shapes, [(0, '│'), (5, '│')]);
    }

    #[test]
    fn wide_characters_keep_their_columns() {
        let built = build(&cells("a日本b c"));
        assert_eq!(texts(&built), [(0, false, "a"), (1, true, "日本"), (5, false, "b"), (7, false, "c")]);
    }

    #[test]
    fn backgrounds_merge_and_the_selection_recolors() {
        let mut row = cells("abcdef");
        for cell in &mut row[1..3] {
            cell.bg = Color::Named(NamedColor::Red);
        }
        let palette = palette();
        let marks = Marks { selected: Some((4, 5)), ..Marks::default() };
        let built = build_row(&row, &palette, &Colors::default(), &marks);
        assert_eq!(built.backgrounds, [(1, 3, palette.ansi[1]), (4, 6, palette.selection)]);
        let fg: Vec<Hsla> = built.segments[0].runs.iter().map(|(_, style)| style.fg).collect();
        let selected = palette.selection_fg.unwrap_or(palette.foreground);
        assert_eq!(fg.last(), Some(&selected));
    }

    #[test]
    fn matches_tint_and_the_current_one_stands_out() {
        let palette = palette();
        let marks = Marks { selected: None, found: vec![(0, 1, false), (4, 5, true)] };
        let built = build_row(&cells("ab  cd"), &palette, &Colors::default(), &marks);
        assert_eq!(
            built.backgrounds,
            [(0, 2, palette.find_match), (4, 6, palette.find_current)],
            "an ordinary match tints the terminal's background"
        );
        let current = built.segments[1].runs[0].1;
        assert_eq!((current.fg, current.bold), (palette.find_current_text, true));
        assert_eq!(
            built.segments[0].runs[0].1.fg, palette.foreground,
            "an ordinary match keeps its text color"
        );
        // A selected match shows the selection; the current match shows over it.
        let marks = Marks { selected: Some((0, 5)), found: vec![(0, 1, false), (4, 5, true)] };
        let built = build_row(&cells("ab  cd"), &palette, &Colors::default(), &marks);
        assert_eq!(built.backgrounds, [(0, 4, palette.selection), (4, 6, palette.find_current)]);
    }

    #[test]
    fn image_placeholders_become_runs_of_tiles_and_no_text() {
        use wormhole_core::alacritty_terminal::term::cell::Flags;
        use wormhole_core::alacritty_terminal::vte::ansi::Rgb;
        use wormhole_core::terminal::images::PLACEHOLDER;
        // Row 1 of a placement of image 0x000005, tiles 0..3 at columns 1..4, then an unrelated
        // tile (another image) and text.
        let mut row = cells("a      b");
        let marks = ['\u{030D}', '\u{0305}', '\u{030D}', '\u{030E}', '\u{0310}'];
        for (k, col) in (1..=4).enumerate() {
            row[col].c = PLACEHOLDER;
            row[col].fg = Color::Spec(Rgb { r: 0, g: 0, b: 5 });
            row[col].push_zerowidth(marks[0]);
            row[col].push_zerowidth(marks[k + 1]);
        }
        row[5].c = PLACEHOLDER;
        row[5].fg = Color::Spec(Rgb { r: 0, g: 0, b: 6 });
        row[5].push_zerowidth(marks[1]);
        row[5].push_zerowidth(marks[1]);
        row[5].flags = Flags::empty();
        let built = build(&row);
        assert_eq!(texts(&built), [(0, false, "a"), (7, false, "b")]);
        assert_eq!(
            built.images,
            [
                ImageRun { col: 1, cells: 4, image: 5, placement: 0, tile_row: 1, tile_col: 0 },
                ImageRun { col: 5, cells: 1, image: 6, placement: 0, tile_row: 0, tile_col: 0 },
            ]
        );
    }

    #[test]
    fn zero_width_marks_stay_with_their_character() {
        let mut row = cells("e x");
        row[0].push_zerowidth('\u{301}');
        let built = build(&row);
        assert_eq!(texts(&built), [(0, false, "e\u{301}"), (2, false, "x")]);
    }
}
