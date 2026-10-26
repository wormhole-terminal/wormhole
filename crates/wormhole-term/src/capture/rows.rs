//! Rows copied out of the screen under its lock and turned into text and style runs outside it.

use wormhole_vt::{Cell, Color, Flags, Line, NamedColor, Rgb, RowExtras};

use crate::images::PLACEHOLDER;
use crate::screen::Screen;

/// How many rows are copied under one hold of the lock.
pub(crate) const CHUNK: usize = 256;

/// One row as copied: its cells, what they carry beside them and whether the line goes on in the
/// next row.
#[derive(Default)]
pub(crate) struct RowBuf {
    cells: Vec<Cell>,
    extras: RowExtras,
    pub(crate) wrapped: bool,
}

/// The buffers a read copies rows into, kept between reads so a read allocates only when it needs
/// more rows than any before it.
#[derive(Default)]
pub struct ReadScratch {
    rows: Vec<RowBuf>,
}

impl ReadScratch {
    pub(crate) fn rows(&self, n: usize) -> &[RowBuf] {
        &self.rows[..n]
    }
}

/// Copy grid lines `from..=to` (at most [`CHUNK`]) into `scratch`; how many rows that is.
pub(crate) fn copy_rows(screen: &Screen, from: i32, to: i32, scratch: &mut ReadScratch) -> usize {
    let n = usize::try_from(to - from + 1).unwrap_or(0).min(CHUNK);
    if scratch.rows.len() < n {
        scratch.rows.resize_with(n, RowBuf::default);
    }
    for (offset, row) in scratch.rows.iter_mut().take(n).enumerate() {
        screen.copy_row(Line(from + offset as i32), &mut row.cells, Some(&mut row.extras));
        row.wrapped = row.cells.last().is_some_and(|cell| cell.flags.contains(Flags::WRAPLINE));
    }
    n
}

impl RowBuf {
    /// The text of the first `upto` columns: wide characters once, what the program drew hidden
    /// (SGR 8) and the inline images' placeholders as blanks, marks beside their characters.
    pub(crate) fn push_text(&self, out: &mut String, upto: usize) {
        for (col, cell) in self.cells.iter().enumerate().take(upto) {
            if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                continue;
            }
            if cell.flags.contains(Flags::HIDDEN) || cell.c == PLACEHOLDER {
                out.push(' ');
                continue;
            }
            out.push(cell.c);
            out.extend(self.extras.zerowidth(col));
        }
    }

    /// The column after the last cell that holds anything but a blank.
    pub(crate) fn text_end(&self) -> usize {
        self.cells.iter().rposition(|cell| cell.c != ' ').map_or(0, |at| at + 1)
    }

    /// How many columns the row has.
    pub(crate) fn len(&self) -> usize {
        self.cells.len()
    }

    /// The row's colours and attributes as runs of cells that share them; cells in the default
    /// style are left out.
    pub(crate) fn style_runs(&self) -> Vec<StyleRun> {
        let mut runs: Vec<StyleRun> = Vec::new();
        for (col, cell) in self.cells.iter().enumerate() {
            let style = Style::of(cell);
            if style == Style::default() {
                continue;
            }
            let col = u16::try_from(col).unwrap_or(u16::MAX);
            match runs.last_mut() {
                Some(run) if run.style == style && run.col + run.len == col => run.len += 1,
                _ => runs.push(StyleRun { col, len: 1, style }),
            }
        }
        runs
    }
}

/// How a cell is drawn, as far as reading the screen cares.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    /// `None`: the terminal's default colour.
    pub fg: Option<Hue>,
    pub bg: Option<Hue>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub dim: bool,
}

impl Style {
    fn of(cell: &Cell) -> Self {
        Self {
            fg: hue(cell.fg),
            bg: hue(cell.bg),
            bold: cell.flags.contains(Flags::BOLD),
            italic: cell.flags.contains(Flags::ITALIC),
            underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
            inverse: cell.flags.contains(Flags::INVERSE),
            dim: cell.flags.contains(Flags::DIM),
        }
    }
}

/// A colour a cell asks for: a palette entry or a true colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hue {
    Indexed(u8),
    Rgb(Rgb),
}

fn hue(color: Color) -> Option<Hue> {
    match color {
        Color::Named(NamedColor::Foreground | NamedColor::Background | NamedColor::Cursor) => None,
        Color::Named(named) => Some(Hue::Indexed(named as u16 as u8)),
        Color::Indexed(index) => Some(Hue::Indexed(index)),
        Color::Rgb(rgb) => Some(Hue::Rgb(rgb)),
    }
}

/// Cells `col..col + len` of a row, drawn in `style`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleRun {
    pub col: u16,
    pub len: u16,
    pub style: Style,
}
