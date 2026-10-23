//! One terminal row turned into what the painter needs: its words with style
//! runs, background, underline and strikethrough spans, and the cells painted as shapes.
//!
//! This runs for every changed row, so it does one pass over the cells with no
//! allocation per cell: characters go straight into the segment's text and a
//! new run starts only where the style changes (a small `Copy` key). Blanks are
//! never shaped or painted: they end a segment, so a segment is one word, which
//! `text` shapes once and reuses wherever it shows up again.

use gpui_kit::Hsla;
use wormhole_term::images::{Placeholder, placeholder_at};
use wormhole_term::types::{Cell, Colors, Flags, LineSize, RowExtras};

use crate::palette::Palette;

/// The text attributes runs are split on. Underlines aren't among them: the
/// terminal paints those itself (see [`Underline`]), and strikethroughs in rows too. `blink` only
/// splits runs: shaping ignores it (words are cached by text and fonts), the painter fades the
/// glyphs of a blinking run in the clock's off phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Style {
    pub fg: Hsla,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub blink: bool,
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
            blink: flags.contains(Flags::BLINK),
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
        // Exactly one of these is set; the order only matters for a corrupt cell.
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
            if at >= bytes.end {
                break;
            }
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
    /// `[first, end)` columns with a line along the top of the cell (SGR 53), in a color, merged.
    pub overlines: Vec<(usize, usize, Hsla)>,
    /// Some visible text on the row blinks (SGR 5 and 6): the screen needs the blink clock.
    pub blinks: bool,
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
            && self.image == p.image_id
            && self.placement == p.placement_id
            && self.tile_row == p.row
            && u32::from(self.tile_col) + self.cells as u32 == u32::from(p.col)
    }
}

/// What a row is built with beyond its cells: the selection and find's matches, which color them,
/// and the row's size, which says how many of them show. A row is kept or found again by its
/// cells and these.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Marks {
    /// How big the row is drawn: a double-width or double-height row shows the first half of its
    /// cells (each is drawn twice as wide by the painter, which scales by columns).
    pub size: LineSize,
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

/// Build one row with what `marks` colors on it. `extras` is what the cells carry beside them
/// (almost always nothing, which costs a check per cell).
#[cfg(test)]
pub(super) fn build_row(
    cells: &[Cell],
    extras: &RowExtras,
    palette: &Palette,
    overrides: &Colors,
    marks: &Marks,
) -> BuiltRow {
    let mut builder = RowBuilder::default();
    builder.build(cells, extras, palette, overrides, marks);
    builder.row
}

/// Builds rows one after another, keeping the last row's buffers (its segments' text and runs
/// too) for the next, so a screen rebuilt every frame stops allocating once they are big enough.
#[derive(Default)]
pub(super) struct RowBuilder {
    row: BuiltRow,
    /// The segments of earlier rows, emptied, waiting to be filled again.
    idle: Vec<Segment>,
}

impl BuiltRow {
    /// Empty the row for the next build. The segments go to `idle` with their buffers.
    fn recycle(&mut self, idle: &mut Vec<Segment>) {
        idle.append(&mut self.segments);
        self.backgrounds.clear();
        self.underlines.clear();
        self.strikes.clear();
        self.overlines.clear();
        self.blinks = false;
        self.shapes.clear();
        self.images.clear();
    }
}

impl RowBuilder {
    /// The row built from `cells`; it is the builder's own and is emptied by the next build. Its
    /// vectors may be swapped for spare ones (`std::mem::swap`), which then come back cleared.
    pub fn build(
        &mut self,
        cells: &[Cell],
        extras: &RowExtras,
        palette: &Palette,
        overrides: &Colors,
        marks: &Marks,
    ) -> &mut BuiltRow {
        let Self { row, idle } = self;
        row.recycle(idle);
        fill(row, idle, cells, extras, palette, overrides, marks);
        row
    }
}

fn fill(
    row: &mut BuiltRow,
    idle: &mut Vec<Segment>,
    cells: &[Cell],
    extras: &RowExtras,
    palette: &Palette,
    overrides: &Colors,
    marks: &Marks,
) {
    let shown = marks.size.visible_columns(cells.len());
    for (col, cell) in cells.iter().enumerate().take(shown) {
        if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
            continue;
        }
        let extra = extras.cell(col);
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
        if let Some(placeholder) = placeholder_at(cell, extra) {
            match row.images.last_mut() {
                Some(run) if run.continues(col, &placeholder) => run.cells += 1,
                _ => row.images.push(ImageRun {
                    col,
                    cells: 1,
                    image: placeholder.image_id,
                    placement: placeholder.placement_id,
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
        // A hidden cell (SGR 8) hides its underline and overline too. Both take the underline's
        // color when the program set one (SGR 58), else the text's.
        let visible = !cell.flags.contains(Flags::HIDDEN);
        let line_color = || {
            extra
                .underline_color
                .filter(|_| !in_selection || palette.selection_fg.is_none())
                .map_or(fg, |color| palette.resolve(color, overrides))
        };
        if let Some(kind) = UnderlineKind::of(cell.flags).filter(|_| visible) {
            let color = line_color();
            match row.underlines.last_mut() {
                Some(last) if last.end == col && last.kind == kind && last.color == color => last.end = end,
                _ => row.underlines.push(Underline { first: col, end, kind, color }),
            }
        }
        if visible && cell.flags.contains(Flags::OVERLINE) {
            let color = line_color();
            match row.overlines.last_mut() {
                Some((_, last_end, last)) if *last_end == col && *last == color => *last_end = end,
                _ => row.overlines.push((col, end, color)),
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
        let zerowidth = extra.zerowidth;
        // A blank draws nothing: it ends the word before it, and the next
        // character starts one at its own column.
        if shown == ' ' && zerowidth.is_empty() {
            continue;
        }

        let continues = row.segments.last().is_some_and(|seg| seg.wide == wide && seg.end == col);
        if !continues {
            let mut segment = idle.pop().unwrap_or_else(|| Segment {
                col,
                wide,
                text: String::new(),
                runs: Vec::new(),
                end: col,
            });
            (segment.col, segment.wide, segment.end) = (col, wide, col);
            segment.text.clear();
            segment.runs.clear();
            row.segments.push(segment);
        }
        let Some(segment) = row.segments.last_mut() else { continue };
        let start = segment.text.len();
        segment.text.push(shown);
        segment.text.extend(zerowidth);
        segment.end = end;
        let len = segment.text.len() - start;
        row.blinks |= style.blink && visible;
        match segment.runs.last_mut() {
            Some((run_len, run_style)) if *run_style == style => *run_len += len,
            _ => segment.runs.push((len, style)),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::Hsla;
    use wormhole_core::theme;
    use wormhole_term::types::{Cell, Color, Colors, Flags, NamedColor, RowExtras};

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
        build_with(cells, &RowExtras::default())
    }

    fn build_with(cells: &[Cell], extras: &RowExtras) -> BuiltRow {
        build_row(cells, extras, &palette(), &Colors::default(), &Marks::default())
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
        let mut extras = RowExtras::default();
        extras.set_underline_color(7, Some(Color::Named(NamedColor::Red)));
        let palette = palette();
        let built = build_with(&row, &extras);
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
    fn overlines_take_the_underline_color_else_the_text_color() {
        let mut row = cells("ab cd e");
        for cell in &mut row[..2] {
            cell.flags = Flags::OVERLINE;
        }
        for cell in &mut row[3..] {
            cell.flags = Flags::OVERLINE | Flags::HIDDEN;
        }
        row[3].flags = Flags::OVERLINE;
        row[4].flags = Flags::OVERLINE;
        let mut extras = RowExtras::default();
        extras.set_underline_color(4, Some(Color::Named(NamedColor::Red)));
        let palette = palette();
        let built = build_with(&row, &extras);
        assert_eq!(
            built.overlines,
            [(0, 2, palette.foreground), (3, 4, palette.foreground), (4, 5, palette.ansi[1])],
            "merged where the color continues; the hidden cells draw none"
        );
        assert!(built.underlines.is_empty(), "an overline is not an underline");
        let mut blank = cells("   ");
        blank[1].flags = Flags::OVERLINE;
        let built = build(&blank);
        assert_eq!((built.overlines.len(), built.segments.len()), (1, 0), "no text needed");
    }

    #[test]
    fn a_sized_row_shows_half_its_cells() {
        use wormhole_term::types::LineSize;
        let mut row = cells("abcdefgh");
        row[6].bg = Color::Named(NamedColor::Red);
        let marks = Marks { size: LineSize::DoubleWidth, ..Marks::default() };
        let built = build_row(&row, &RowExtras::default(), &palette(), &Colors::default(), &marks);
        assert_eq!(texts(&built), [(0, false, "abcd")], "the cells past the half are hidden");
        assert!(built.backgrounds.is_empty(), "and so are their backgrounds");
        let single =
            build_row(&row, &RowExtras::default(), &palette(), &Colors::default(), &Marks::default());
        assert_eq!(texts(&single), [(0, false, "abcdefgh")]);
    }

    #[test]
    fn blinking_cells_are_marked_on_their_run_and_the_row() {
        let mut row = cells("ab cd");
        row[0].flags = Flags::BLINK;
        let built = build(&row);
        assert!(built.blinks);
        let styles: Vec<bool> = built.segments[0].runs.iter().map(|(_, style)| style.blink).collect();
        assert_eq!(styles, [true, false]);
        assert!(!build(&cells("ab cd")).blinks);
        let mut blank = cells("   ");
        blank[1].flags = Flags::BLINK;
        assert!(!build(&blank).blinks, "a blinking blank draws nothing, so nothing blinks");
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
        let built = build_row(&row, &RowExtras::default(), &palette, &Colors::default(), &marks);
        assert_eq!(built.backgrounds, [(1, 3, palette.ansi[1]), (4, 6, palette.selection)]);
        let fg: Vec<Hsla> = built.segments[0].runs.iter().map(|(_, style)| style.fg).collect();
        let selected = palette.selection_fg.unwrap_or(palette.foreground);
        assert_eq!(fg.last(), Some(&selected));
    }

    #[test]
    fn matches_tint_and_the_current_one_stands_out() {
        let palette = palette();
        let marks = Marks { found: vec![(0, 1, false), (4, 5, true)], ..Marks::default() };
        let extras = RowExtras::default();
        let built = build_row(&cells("ab  cd"), &extras, &palette, &Colors::default(), &marks);
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
        let marks =
            Marks { selected: Some((0, 5)), found: vec![(0, 1, false), (4, 5, true)], ..Marks::default() };
        let built = build_row(&cells("ab  cd"), &extras, &palette, &Colors::default(), &marks);
        assert_eq!(built.backgrounds, [(0, 4, palette.selection), (4, 6, palette.find_current)]);
    }

    #[test]
    fn image_placeholders_become_runs_of_tiles_and_no_text() {
        use wormhole_term::images::PLACEHOLDER;
        use wormhole_term::types::Rgb;
        // Row 1 of a placement of image 0x000005, tiles 0..3 at columns 1..4, then an unrelated
        // tile (another image) and text.
        let mut row = cells("a      b");
        let marks = ['\u{030D}', '\u{0305}', '\u{030D}', '\u{030E}', '\u{0310}'];
        let mut extras = RowExtras::default();
        for (k, col) in (1..=4).enumerate() {
            row[col].c = PLACEHOLDER;
            row[col].fg = Color::Rgb(Rgb { r: 0, g: 0, b: 5 });
            extras.push_zerowidth(col, marks[0]);
            extras.push_zerowidth(col, marks[k + 1]);
        }
        row[5].c = PLACEHOLDER;
        row[5].fg = Color::Rgb(Rgb { r: 0, g: 0, b: 6 });
        extras.push_zerowidth(5, marks[1]);
        extras.push_zerowidth(5, marks[1]);
        row[5].flags = Flags::empty();
        let built = build_with(&row, &extras);
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
        let row = cells("e x");
        let mut extras = RowExtras::default();
        extras.push_zerowidth(0, '\u{301}');
        let built = build_with(&row, &extras);
        assert_eq!(texts(&built), [(0, false, "e\u{301}"), (2, false, "x")]);
    }

    #[test]
    fn a_blank_that_only_has_a_link_or_an_underline_color_is_still_a_blank() {
        // Everything a shell with integration prints carries a mark, spaces included; none of it
        // makes a blank into something to shape.
        let row = cells("a b");
        let mut extras = RowExtras::default();
        extras.set_underline_color(1, Some(Color::Indexed(2)));
        let built = build_with(&row, &extras);
        assert_eq!(texts(&built), [(0, false, "a"), (2, false, "b")]);
    }

    /// A builder that has built other rows builds the same row as a new one: nothing of an earlier
    /// row (its words, spans, blink or shapes) is left in the buffers it reuses.
    #[test]
    fn a_builder_that_built_other_rows_builds_the_same_as_a_new_one() {
        use super::RowBuilder;

        let blink = {
            let mut row = cells("blink ┌─┐ x");
            row[0].flags = Flags::BLINK | Flags::UNDERLINE | Flags::STRIKEOUT | Flags::OVERLINE;
            colored(row, 2..8, NamedColor::Red)
        };
        let rows = [
            blink,
            cells("a b c d e f g"),
            cells("日本 mixed wide"),
            colored(cells("short"), 0..2, NamedColor::Green),
            cells(""),
            cells("one"),
        ];
        let mut builder = RowBuilder::default();
        let (palette, colors, marks) = (palette(), Colors::default(), Marks::default());
        for _ in 0..2 {
            for row in &rows {
                let again = builder.build(row, &RowExtras::default(), &palette, &colors, &marks);
                assert_eq!(*again, build(row), "{:?}", row.iter().map(|c| c.c).collect::<String>());
            }
        }
    }
}
