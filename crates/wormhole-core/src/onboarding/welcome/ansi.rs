//! The welcome's screen: a grid of styled cells that the pages draw into, and the bytes that put it
//! on the terminal.
//!
//! Every character the program shows goes through [`Grid::put`], which drops control characters and
//! characters with no width, so nothing a folder is called (or an agent, a theme) can reach the
//! terminal as an escape sequence: the only escapes in the output are the ones [`Grid::encode_row`]
//! writes itself.
//!
//! Colours are the terminal's 16 (`0..=15`), never 24-bit, so the theme applies: a theme change in the
//! look step re-colours what is already on screen, and the page follows it on the next draw.

use std::fmt::Write as _;

/// The ANSI colours, by the number the terminal's palette has for them.
pub(super) mod color {
    pub(crate) const RED: u8 = 1;
    pub(crate) const GREEN: u8 = 2;
    /// The sidebar's amber, in the palette: a terminal that needs input.
    pub(crate) const YELLOW: u8 = 3;
    pub(crate) const BLUE: u8 = 4;
    pub(crate) const BRIGHT_BLACK: u8 = 8;
    pub(crate) const BRIGHT_WHITE: u8 = 15;
}

/// How a cell is drawn: palette colours (`None` is the terminal's default), bold, dim, reverse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Style {
    pub fg: Option<u8>,
    pub bg: Option<u8>,
    pub bold: bool,
    pub dim: bool,
    pub reverse: bool,
}

impl Style {
    pub(super) const PLAIN: Style = Style { fg: None, bg: None, bold: false, dim: false, reverse: false };

    pub(super) const fn fg(mut self, colour: u8) -> Self {
        self.fg = Some(colour);
        self
    }

    pub(super) const fn bg(mut self, colour: u8) -> Self {
        self.bg = Some(colour);
        self
    }

    pub(super) const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub(super) const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    /// The escape that sets exactly this style from the plain one.
    pub(super) fn escape(self) -> String {
        let mut out = String::from("\x1b[0");
        if self.bold {
            out.push_str(";1");
        }
        if self.dim {
            out.push_str(";2");
        }
        if self.reverse {
            out.push_str(";7");
        }
        // 0..=7 are 30..=37 (40..=47 for a background), 8..=15 the bright ones at 90 (100).
        let code = |colour: u8, normal: u8, bright: u8| match colour {
            0..=7 => normal + colour,
            _ => bright + (colour.min(15) - 8),
        };
        if let Some(fg) = self.fg {
            let _ = write!(out, ";{}", code(fg, 30, 90));
        }
        if let Some(bg) = self.bg {
            let _ = write!(out, ";{}", code(bg, 40, 100));
        }
        out.push('m');
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    ch: char,
    style: Style,
    /// The right half of a wide character: not written, the character before it covers it.
    cont: bool,
}

const BLANK: Cell = Cell { ch: ' ', style: Style::PLAIN, cont: false };

/// How many cells `c` takes: 0 for what has no width (those are dropped), 2 for the wide ones
/// (CJK, emoji), else 1. An approximation of the terminal's own table, enough for the program's
/// words and the names of folders: a name that is off by a cell only shifts the rest of its row.
pub(super) fn width(c: char) -> usize {
    let u = c as u32;
    match u {
        0..=0x1f | 0x7f..=0x9f => 0,
        // Combining marks, the zero-width space and joiners, variation selectors.
        0x300..=0x36f | 0x200b..=0x200f | 0x2028..=0x202e | 0x2060..=0x206f | 0xfe00..=0xfe0f | 0xfeff => 0,
        0x1100..=0x115f
        | 0x2e80..=0xa4cf
        | 0xac00..=0xd7a3
        | 0xf900..=0xfaff
        | 0xfe30..=0xfe6f
        | 0xff00..=0xff60
        | 0xffe0..=0xffe6
        | 0x1f300..=0x1f64f
        | 0x1f900..=0x1f9ff
        | 0x20000..=0x3fffd => 2,
        _ => 1,
    }
}

/// The cells `text` takes.
pub(super) fn text_width(text: &str) -> usize {
    text.chars().map(width).sum()
}

/// `text` broken into lines of at most `max` cells at its spaces. A word wider than a line is cut.
pub(super) fn wrap(text: &str, max: usize) -> Vec<String> {
    let max = max.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut taken = 0;
    for word in text.split_whitespace() {
        let w = text_width(word);
        if taken > 0 && taken + 1 + w > max {
            lines.push(std::mem::take(&mut line));
            taken = 0;
        }
        if taken > 0 {
            line.push(' ');
            taken += 1;
        }
        for c in word.chars() {
            let cw = width(c);
            if taken + cw > max {
                lines.push(std::mem::take(&mut line));
                taken = 0;
            }
            line.push(c);
            taken += cw;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// `text` in at most `max` cells, an ellipsis in place of what does not fit.
pub(super) fn fit(text: &str, max: usize) -> String {
    if text_width(text) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut taken = 0;
    for c in text.chars() {
        let cw = width(c);
        if taken + cw + 1 > max {
            break;
        }
        out.push(c);
        taken += cw;
    }
    if max > 0 {
        out.push('…');
    }
    out
}

/// The screen: `rows` rows of `cols` cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
}

impl Grid {
    pub(super) fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows, cells: vec![BLANK; cols * rows] }
    }

    #[cfg(test)]
    pub(super) fn rows(&self) -> usize {
        self.rows
    }

    /// Write `text` from (`row`, `col`), clipped at the right edge; returns the column after it.
    /// Characters with no width are dropped, so what is written is only ever printable.
    pub(super) fn put(&mut self, row: usize, col: usize, text: &str, style: Style) -> usize {
        let mut at = col;
        if row >= self.rows {
            return at;
        }
        for c in text.chars() {
            let w = width(c);
            if w == 0 {
                continue;
            }
            // A wide character that would be cut by the edge is left out.
            if at + w > self.cols {
                break;
            }
            let ix = row * self.cols + at;
            self.cells[ix] = Cell { ch: c, style, cont: false };
            if w == 2 {
                self.cells[ix + 1] = Cell { ch: ' ', style, cont: true };
            }
            at += w;
        }
        at
    }

    /// The style of the cell at (`row`, `col`).
    #[cfg(test)]
    pub(super) fn style_at(&self, row: usize, col: usize) -> Style {
        self.cells.get(row * self.cols + col).map_or(Style::PLAIN, |cell| cell.style)
    }

    /// The row's text without its styles, trailing spaces cut (what a snapshot test compares).
    #[cfg(test)]
    pub(super) fn text_row(&self, row: usize) -> String {
        let cells = &self.cells[row * self.cols..(row + 1) * self.cols];
        let text: String = cells.iter().filter(|cell| !cell.cont).map(|cell| cell.ch).collect();
        text.trim_end().to_string()
    }

    /// Every row as plain text.
    #[cfg(test)]
    pub(super) fn text(&self) -> Vec<String> {
        (0..self.rows).map(|row| self.text_row(row)).collect()
    }

    /// The bytes that put `row` on the screen: move there, draw it with its styles and clear the rest
    /// of the line. The cursor stays where the next row's move puts it.
    pub(super) fn encode_row(&self, row: usize, out: &mut Vec<u8>) {
        let cells = &self.cells[row * self.cols..(row + 1) * self.cols];
        out.extend_from_slice(format!("\x1b[{};1H\x1b[0m", row + 1).as_bytes());
        // Up to the last cell that is not a plain blank: the rest is cleared, not drawn.
        let end = cells.iter().rposition(|cell| *cell != BLANK).map_or(0, |ix| ix + 1);
        let mut current = Style::PLAIN;
        let mut buf = [0u8; 4];
        for cell in &cells[..end] {
            if cell.cont {
                continue;
            }
            if cell.style != current {
                out.extend_from_slice(cell.style.escape().as_bytes());
                current = cell.style;
            }
            out.extend_from_slice(cell.ch.encode_utf8(&mut buf).as_bytes());
        }
        out.extend_from_slice(b"\x1b[0m\x1b[K");
    }

    /// The bytes that turn what `previous` showed into this: the rows that differ, or all of them
    /// after an erase from the top when there was no previous grid or the size changed. Never a full
    /// clear (`2J`): a terminal keeps what that wipes in its scrollback, which a resize would fill
    /// with copies of the page.
    pub(super) fn encode(&self, previous: Option<&Grid>) -> Vec<u8> {
        let mut out = Vec::new();
        let same_size = previous.is_some_and(|prev| prev.cols == self.cols && prev.rows == self.rows);
        if !same_size {
            // Erase from the top, not `2J`: a terminal keeps what a full clear wipes in its scrollback.
            out.extend_from_slice(b"\x1b[H\x1b[J");
        }
        for row in 0..self.rows {
            let unchanged = same_size
                && previous.is_some_and(|prev| {
                    prev.cells[row * prev.cols..(row + 1) * prev.cols]
                        == self.cells[row * self.cols..(row + 1) * self.cols]
                });
            if !unchanged {
                self.encode_row(row, &mut out);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_clipped_at_the_edge_and_controls_are_dropped() {
        let mut grid = Grid::new(8, 2);
        assert_eq!(grid.put(0, 2, "abcdefghij", Style::PLAIN), 8);
        assert_eq!(grid.text_row(0), "  abcdef");
        // An escape in a name never reaches the screen: only printable characters are written.
        grid.put(1, 0, "a\x1b[31mb\x07\r\nc", Style::PLAIN);
        assert_eq!(grid.text_row(1), "a[31mbc");
        assert_eq!(grid.put(5, 0, "x", Style::PLAIN), 0, "a row below the screen is ignored");
    }

    #[test]
    fn wide_characters_take_two_cells_and_are_not_cut_by_the_edge() {
        let mut grid = Grid::new(5, 1);
        assert_eq!(grid.put(0, 0, "a日本語", Style::PLAIN), 5);
        assert_eq!(grid.text_row(0), "a日本");
        let mut tight = Grid::new(3, 1);
        tight.put(0, 0, "ab日", Style::PLAIN);
        assert_eq!(tight.text_row(0), "ab", "the wide character would overhang");
        assert_eq!(text_width("a日"), 3);
        assert_eq!(width('\u{301}'), 0);
    }

    #[test]
    fn styles_become_palette_escapes_and_never_truecolor() {
        assert_eq!(Style::PLAIN.escape(), "\x1b[0m");
        assert_eq!(Style::PLAIN.bold().fg(color::BLUE).escape(), "\x1b[0;1;34m");
        assert_eq!(Style::PLAIN.dim().fg(color::BRIGHT_BLACK).bg(color::RED).escape(), "\x1b[0;2;90;41m");
        assert_eq!(Style { reverse: true, ..Style::PLAIN }.bg(15).escape(), "\x1b[0;7;107m");
        let mut grid = Grid::new(10, 1);
        for colour in 0..16u8 {
            grid.put(0, 0, "x", Style::PLAIN.fg(colour).bg(colour));
            let mut out = Vec::new();
            grid.encode_row(0, &mut out);
            let text = String::from_utf8(out).expect("utf-8");
            assert!(!text.contains("38;") && !text.contains("48;"), "{text:?}");
        }
    }

    /// A row is: move there, plain reset, runs with a style change only where the style changes,
    /// then clear the rest. Trailing blanks are not written.
    #[test]
    fn a_row_is_encoded_with_its_style_changes_and_a_clear() {
        let mut grid = Grid::new(10, 2);
        grid.put(1, 1, "hi", Style::PLAIN.bold());
        grid.put(1, 3, " you", Style::PLAIN);
        let mut out = Vec::new();
        grid.encode_row(1, &mut out);
        let text = String::from_utf8(out).expect("utf-8");
        assert_eq!(text, "\x1b[2;1H\x1b[0m \x1b[0;1mhi\x1b[0m you\x1b[0m\x1b[K");
        let mut blank = Vec::new();
        grid.encode_row(0, &mut blank);
        assert_eq!(String::from_utf8(blank).expect("utf-8"), "\x1b[1;1H\x1b[0m\x1b[0m\x1b[K");
    }

    #[test]
    fn only_the_rows_that_changed_are_sent() {
        let mut before = Grid::new(6, 3);
        before.put(0, 0, "one", Style::PLAIN);
        before.put(1, 0, "two", Style::PLAIN);
        let mut after = before.clone();
        after.put(1, 0, "twp", Style::PLAIN);
        let diff = String::from_utf8(after.encode(Some(&before))).expect("utf-8");
        assert!(diff.contains("twp") && !diff.contains("one") && !diff.contains("\x1b[J"), "{diff:?}");
        assert!(after.encode(Some(&after)).is_empty(), "nothing changed, nothing sent");
        // The first frame, and a resize, clear the screen first and draw every row.
        let first = String::from_utf8(after.encode(None)).expect("utf-8");
        assert!(first.starts_with("\x1b[H\x1b[J") && first.contains("one") && first.contains("twp"));
        let other = Grid::new(7, 3);
        assert!(other.encode(Some(&after)).starts_with(b"\x1b[H\x1b[J"));
    }

    #[test]
    fn text_wraps_at_spaces_and_cuts_a_word_that_is_too_long() {
        assert_eq!(wrap("one two three four", 9), ["one two", "three", "four"]);
        assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap("", 5), Vec::<String>::new());
        assert_eq!(wrap("  spaced   out  ", 20), ["spaced out"]);
    }

    #[test]
    fn long_names_are_cut_with_an_ellipsis() {
        assert_eq!(fit("Catppuccin Mocha", 20), "Catppuccin Mocha");
        assert_eq!(fit("Catppuccin Mocha", 10), "Catppucci…");
        assert_eq!(text_width(&fit("日本語日本語", 7)), 7);
        assert_eq!(fit("abc", 0), "");
    }
}
