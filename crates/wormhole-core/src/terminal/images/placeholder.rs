//! Where an image sits, written into the grid as kitty's Unicode placeholders.
//!
//! A placement is a rectangle of cells showing `U+10EEEE`, each with the image id in its
//! foreground color (the low 24 bits; the high byte in a third combining mark), the placement id
//! in its underline color, and its row and column within the image as the first two combining
//! marks, from a table of 297 diacritics kitty publishes. Every cell says on its own which tile of
//! which image it shows, so a row that wraps on a resize, scrolls into the history or is partly
//! overwritten still draws the right tiles. The same cells are what a program prints itself for
//! kitty's virtual placements (`U=1`), so those work alike.
//!
//! [`placeholder_bytes`] is what the PTY filter prints in a sequence's place: the rows go in
//! first with line feeds, so the screen scrolls before the cursor is saved, then each row is
//! written from the saved position with auto-wrap off, and the program's own attributes come
//! back with the cursor, which ends right of the placement's last row as kitty leaves it. The
//! filter doesn't know the cursor's column, so a row wider than what is left of the line isn't
//! clipped as kitty clips it: the last column takes the row's last tile (the filter keeps a
//! placement within the grid's width, so this needs one started mid-row). Auto-wrap is turned on
//! again at the end: a program that had turned it off would have to turn it off once more.

use std::borrow::Cow;

use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::vte::ansi::Color;

/// The placeholder character (a private-use code point; no font shows it).
pub const PLACEHOLDER: char = '\u{10EEEE}';

pub(super) const DIACRITICS_LEN: usize = 297;

/// kitty's row/column diacritics (`gen/rowcolumn-diacritics.txt`), in order and ascending.
static DIACRITICS: [char; DIACRITICS_LEN] = [
    '\u{0305}',
    '\u{030D}',
    '\u{030E}',
    '\u{0310}',
    '\u{0312}',
    '\u{033D}',
    '\u{033E}',
    '\u{033F}',
    '\u{0346}',
    '\u{034A}',
    '\u{034B}',
    '\u{034C}',
    '\u{0350}',
    '\u{0351}',
    '\u{0352}',
    '\u{0357}',
    '\u{035B}',
    '\u{0363}',
    '\u{0364}',
    '\u{0365}',
    '\u{0366}',
    '\u{0367}',
    '\u{0368}',
    '\u{0369}',
    '\u{036A}',
    '\u{036B}',
    '\u{036C}',
    '\u{036D}',
    '\u{036E}',
    '\u{036F}',
    '\u{0483}',
    '\u{0484}',
    '\u{0485}',
    '\u{0486}',
    '\u{0487}',
    '\u{0592}',
    '\u{0593}',
    '\u{0594}',
    '\u{0595}',
    '\u{0597}',
    '\u{0598}',
    '\u{0599}',
    '\u{059C}',
    '\u{059D}',
    '\u{059E}',
    '\u{059F}',
    '\u{05A0}',
    '\u{05A1}',
    '\u{05A8}',
    '\u{05A9}',
    '\u{05AB}',
    '\u{05AC}',
    '\u{05AF}',
    '\u{05C4}',
    '\u{0610}',
    '\u{0611}',
    '\u{0612}',
    '\u{0613}',
    '\u{0614}',
    '\u{0615}',
    '\u{0616}',
    '\u{0617}',
    '\u{0657}',
    '\u{0658}',
    '\u{0659}',
    '\u{065A}',
    '\u{065B}',
    '\u{065D}',
    '\u{065E}',
    '\u{06D6}',
    '\u{06D7}',
    '\u{06D8}',
    '\u{06D9}',
    '\u{06DA}',
    '\u{06DB}',
    '\u{06DC}',
    '\u{06DF}',
    '\u{06E0}',
    '\u{06E1}',
    '\u{06E2}',
    '\u{06E4}',
    '\u{06E7}',
    '\u{06E8}',
    '\u{06EB}',
    '\u{06EC}',
    '\u{0730}',
    '\u{0732}',
    '\u{0733}',
    '\u{0735}',
    '\u{0736}',
    '\u{073A}',
    '\u{073D}',
    '\u{073F}',
    '\u{0740}',
    '\u{0741}',
    '\u{0743}',
    '\u{0745}',
    '\u{0747}',
    '\u{0749}',
    '\u{074A}',
    '\u{07EB}',
    '\u{07EC}',
    '\u{07ED}',
    '\u{07EE}',
    '\u{07EF}',
    '\u{07F0}',
    '\u{07F1}',
    '\u{07F3}',
    '\u{0816}',
    '\u{0817}',
    '\u{0818}',
    '\u{0819}',
    '\u{081B}',
    '\u{081C}',
    '\u{081D}',
    '\u{081E}',
    '\u{081F}',
    '\u{0820}',
    '\u{0821}',
    '\u{0822}',
    '\u{0823}',
    '\u{0825}',
    '\u{0826}',
    '\u{0827}',
    '\u{0829}',
    '\u{082A}',
    '\u{082B}',
    '\u{082C}',
    '\u{082D}',
    '\u{0951}',
    '\u{0953}',
    '\u{0954}',
    '\u{0F82}',
    '\u{0F83}',
    '\u{0F86}',
    '\u{0F87}',
    '\u{135D}',
    '\u{135E}',
    '\u{135F}',
    '\u{17DD}',
    '\u{193A}',
    '\u{1A17}',
    '\u{1A75}',
    '\u{1A76}',
    '\u{1A77}',
    '\u{1A78}',
    '\u{1A79}',
    '\u{1A7A}',
    '\u{1A7B}',
    '\u{1A7C}',
    '\u{1B6B}',
    '\u{1B6D}',
    '\u{1B6E}',
    '\u{1B6F}',
    '\u{1B70}',
    '\u{1B71}',
    '\u{1B72}',
    '\u{1B73}',
    '\u{1CD0}',
    '\u{1CD1}',
    '\u{1CD2}',
    '\u{1CDA}',
    '\u{1CDB}',
    '\u{1CE0}',
    '\u{1DC0}',
    '\u{1DC1}',
    '\u{1DC3}',
    '\u{1DC4}',
    '\u{1DC5}',
    '\u{1DC6}',
    '\u{1DC7}',
    '\u{1DC8}',
    '\u{1DC9}',
    '\u{1DCB}',
    '\u{1DCC}',
    '\u{1DD1}',
    '\u{1DD2}',
    '\u{1DD3}',
    '\u{1DD4}',
    '\u{1DD5}',
    '\u{1DD6}',
    '\u{1DD7}',
    '\u{1DD8}',
    '\u{1DD9}',
    '\u{1DDA}',
    '\u{1DDB}',
    '\u{1DDC}',
    '\u{1DDD}',
    '\u{1DDE}',
    '\u{1DDF}',
    '\u{1DE0}',
    '\u{1DE1}',
    '\u{1DE2}',
    '\u{1DE3}',
    '\u{1DE4}',
    '\u{1DE5}',
    '\u{1DE6}',
    '\u{1DFE}',
    '\u{20D0}',
    '\u{20D1}',
    '\u{20D4}',
    '\u{20D5}',
    '\u{20D6}',
    '\u{20D7}',
    '\u{20DB}',
    '\u{20DC}',
    '\u{20E1}',
    '\u{20E7}',
    '\u{20E9}',
    '\u{20F0}',
    '\u{2CEF}',
    '\u{2CF0}',
    '\u{2CF1}',
    '\u{2DE0}',
    '\u{2DE1}',
    '\u{2DE2}',
    '\u{2DE3}',
    '\u{2DE4}',
    '\u{2DE5}',
    '\u{2DE6}',
    '\u{2DE7}',
    '\u{2DE8}',
    '\u{2DE9}',
    '\u{2DEA}',
    '\u{2DEB}',
    '\u{2DEC}',
    '\u{2DED}',
    '\u{2DEE}',
    '\u{2DEF}',
    '\u{2DF0}',
    '\u{2DF1}',
    '\u{2DF2}',
    '\u{2DF3}',
    '\u{2DF4}',
    '\u{2DF5}',
    '\u{2DF6}',
    '\u{2DF7}',
    '\u{2DF8}',
    '\u{2DF9}',
    '\u{2DFA}',
    '\u{2DFB}',
    '\u{2DFC}',
    '\u{2DFD}',
    '\u{2DFE}',
    '\u{2DFF}',
    '\u{A66F}',
    '\u{A67C}',
    '\u{A67D}',
    '\u{A6F0}',
    '\u{A6F1}',
    '\u{A8E0}',
    '\u{A8E1}',
    '\u{A8E2}',
    '\u{A8E3}',
    '\u{A8E4}',
    '\u{A8E5}',
    '\u{A8E6}',
    '\u{A8E7}',
    '\u{A8E8}',
    '\u{A8E9}',
    '\u{A8EA}',
    '\u{A8EB}',
    '\u{A8EC}',
    '\u{A8ED}',
    '\u{A8EE}',
    '\u{A8EF}',
    '\u{A8F0}',
    '\u{A8F1}',
    '\u{AAB0}',
    '\u{AAB2}',
    '\u{AAB3}',
    '\u{AAB7}',
    '\u{AAB8}',
    '\u{AABE}',
    '\u{AABF}',
    '\u{AAC1}',
    '\u{FE20}',
    '\u{FE21}',
    '\u{FE22}',
    '\u{FE23}',
    '\u{FE24}',
    '\u{FE25}',
    '\u{FE26}',
    '\u{10A0F}',
    '\u{10A38}',
    '\u{1D185}',
    '\u{1D186}',
    '\u{1D187}',
    '\u{1D188}',
    '\u{1D189}',
    '\u{1D1AA}',
    '\u{1D1AB}',
    '\u{1D1AC}',
    '\u{1D1AD}',
    '\u{1D242}',
    '\u{1D243}',
    '\u{1D244}',
];

/// The index a diacritic stands for (the table is ascending).
fn index_of(c: char) -> Option<u16> {
    DIACRITICS.binary_search(&c).ok().map(|i| i as u16)
}

/// One cell of a placement: which tile of which image it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placeholder {
    pub image: u32,
    /// 0 when the program gave none.
    pub placement: u32,
    pub row: u16,
    pub col: u16,
}

/// The 24 bits a color carries for an id: a true color's RGB, or an indexed color's number.
fn id_bits(color: Color) -> u32 {
    match color {
        Color::Spec(rgb) => u32::from(rgb.r) << 16 | u32::from(rgb.g) << 8 | u32::from(rgb.b),
        Color::Indexed(index) => u32::from(index),
        Color::Named(_) => 0,
    }
}

/// What `cell` shows, if it is a placeholder with its row and column marks.
pub fn placeholder_at(cell: &Cell) -> Option<Placeholder> {
    if cell.c != PLACEHOLDER {
        return None;
    }
    let mut marks = cell.zerowidth().unwrap_or_default().iter().filter_map(|c| index_of(*c));
    let row = marks.next()?;
    let col = marks.next()?;
    let high = u32::from(marks.next().unwrap_or(0));
    let image = high << 24 | id_bits(cell.fg);
    let placement = cell.underline_color().map_or(0, id_bits);
    Some(Placeholder { image, placement, row, col })
}

/// The bytes that put a `cols` × `rows` placement of `image` at the cursor (see the module docs).
/// `cursor_moves`: the cursor ends right of the last row (kitty's default); otherwise it is back
/// where it was.
pub fn placeholder_bytes(image: u32, placement: u32, cols: u16, rows: u16, cursor_moves: bool) -> Vec<u8> {
    let cols = cols.clamp(1, DIACRITICS_LEN as u16);
    let rows = rows.clamp(1, DIACRITICS_LEN as u16);
    let mut out = String::new();
    out.push_str("\x1b[?7l");
    for _ in 1..rows {
        out.push('\n');
    }
    if rows > 1 {
        out.push_str(&format!("\x1b[{}A", rows - 1));
    }
    out.push_str("\x1b7");
    let low = image & 0xFF_FFFF;
    let high = (image >> 24) as usize;
    for row in 0..rows {
        if row > 0 {
            out.push_str(&format!("\x1b8\x1b[{row}B"));
        }
        out.push_str(&format!("\x1b[38;2;{};{};{}m", low >> 16, (low >> 8) & 0xFF, low & 0xFF));
        if placement != 0 {
            let p = placement & 0xFF_FFFF;
            out.push_str(&format!("\x1b[58;2;{};{};{}m", p >> 16, (p >> 8) & 0xFF, p & 0xFF));
        }
        for col in 0..cols {
            out.push(PLACEHOLDER);
            out.push(DIACRITICS[usize::from(row)]);
            out.push(DIACRITICS[usize::from(col)]);
            if high > 0 {
                out.push(DIACRITICS[high.min(DIACRITICS_LEN - 1)]);
            }
        }
    }
    out.push_str("\x1b8");
    if cursor_moves {
        if rows > 1 {
            out.push_str(&format!("\x1b[{}B", rows - 1));
        }
        out.push_str(&format!("\x1b[{cols}C"));
    }
    out.push_str("\x1b[?7h");
    out.into_bytes()
}

/// `text` without its placeholders and their marks (for the clipboard and a saved scrollback).
pub fn strip_placeholders(text: &str) -> Cow<'_, str> {
    if !text.contains(PLACEHOLDER) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut in_placeholder = false;
    for c in text.chars() {
        if c == PLACEHOLDER {
            in_placeholder = true;
            continue;
        }
        if in_placeholder && index_of(c).is_some() {
            continue;
        }
        in_placeholder = false;
        out.push(c);
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Column, Line};
    use alacritty_terminal::term::{Config, Term};
    use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

    use super::{
        DIACRITICS, PLACEHOLDER, Placeholder, placeholder_at, placeholder_bytes, strip_placeholders,
    };
    use crate::terminal::GridSize;

    fn term(cols: u16, rows: u16) -> Term<VoidListener> {
        let size = GridSize { cols, rows, cell_width: 8.0, cell_height: 16.0 };
        Term::new(Config { scrolling_history: 100, ..Config::default() }, &size, VoidListener)
    }

    fn feed(term: &mut Term<VoidListener>, bytes: &[u8]) {
        Processor::<StdSyncHandler>::new().advance(term, bytes);
    }

    /// The placeholder on each cell of `line`, as (col, image, placement, row, col-in-image).
    fn placeholders(term: &Term<VoidListener>, line: i32) -> Vec<(usize, u32, u32, u16, u16)> {
        (0..term.columns())
            .filter_map(|col| {
                let p = placeholder_at(&term.grid()[Line(line)][Column(col)])?;
                Some((col, p.image, p.placement, p.row, p.col))
            })
            .collect()
    }

    #[test]
    fn the_table_is_kittys_297_marks_ascending() {
        assert_eq!(DIACRITICS.len(), 297);
        assert!(DIACRITICS.windows(2).all(|w| w[0] < w[1]));
        assert_eq!((DIACRITICS[0], DIACRITICS[296]), ('\u{0305}', '\u{1D244}'));
        // Every mark is zero-width to alacritty, and the placeholder takes one cell.
        let mut t = term(10, 2);
        feed(&mut t, "a\u{10EEEE}\u{0305}\u{030D}b".as_bytes());
        assert_eq!(t.grid().cursor.point.column, Column(3));
        assert_eq!(t.grid()[Line(0)][Column(2)].c, 'b');
    }

    #[test]
    fn a_placement_is_printed_at_the_cursor_and_read_back_cell_by_cell() {
        let mut t = term(10, 4);
        feed(&mut t, b"$ ");
        feed(&mut t, &placeholder_bytes(0x0102_0304, 7, 3, 2, true));
        feed(&mut t, b"x");
        // Two rows of three, from column 2; the cursor ended right of the last row.
        assert_eq!(
            placeholders(&t, 0),
            [(2, 0x0102_0304, 7, 0, 0), (3, 0x0102_0304, 7, 0, 1), (4, 0x0102_0304, 7, 0, 2)]
        );
        assert_eq!(
            placeholders(&t, 1),
            [(2, 0x0102_0304, 7, 1, 0), (3, 0x0102_0304, 7, 1, 1), (4, 0x0102_0304, 7, 1, 2)]
        );
        assert_eq!(t.grid()[Line(1)][Column(5)].c, 'x');
        assert_eq!(t.grid()[Line(0)][Column(0)].c, '$', "the prompt before it stays");
        // The program's attributes are back: `x` is in the default color.
        assert_eq!(
            t.grid()[Line(1)][Column(5)].fg,
            alacritty_terminal::vte::ansi::Color::Named(
                alacritty_terminal::vte::ansi::NamedColor::Foreground
            )
        );
        assert!(t.mode().contains(alacritty_terminal::term::TermMode::LINE_WRAP));
    }

    #[test]
    fn a_placement_at_the_bottom_scrolls_the_screen_first_and_a_wide_one_is_clipped() {
        let mut t = term(6, 3);
        feed(&mut t, b"one\r\ntwo\r\n> ");
        feed(&mut t, &placeholder_bytes(5, 0, 8, 2, true));
        // `one` scrolled into the history; the placement starts on the `> ` row, clipped at the
        // right edge, and the cursor is on its last row at the edge.
        assert_eq!(t.grid().history_size(), 1);
        assert_eq!(t.grid()[Line(0)][Column(0)].c, 't');
        let row1: Vec<u16> = placeholders(&t, 1).into_iter().map(|p| p.4).collect();
        // Columns 2..5 show tiles 0..3; the last column was written over by every tile after,
        // so it holds the row's last one (kitty would clip there; see the module docs).
        assert_eq!(row1, [0, 1, 2, 7]);
        assert_eq!(placeholders(&t, 2).len(), 4);
        assert_eq!(t.grid().cursor.point.line, Line(2));
        // Without a cursor move it is back where it started (after the scroll).
        let mut t = term(6, 3);
        feed(&mut t, b"a\r\nb\r\nc");
        feed(&mut t, &placeholder_bytes(5, 0, 2, 2, false));
        assert_eq!((t.grid().cursor.point.line, t.grid().cursor.point.column), (Line(1), Column(1)));
    }

    #[test]
    fn the_high_byte_of_a_big_id_rides_in_a_third_mark() {
        let mut t = term(4, 1);
        feed(&mut t, &placeholder_bytes(0xAB00_0001, 0, 1, 1, true));
        assert_eq!(placeholders(&t, 0), [(0, 0xAB00_0001, 0, 0, 0)]);
        // A program's own placeholders with an indexed color (256-color mode) read too.
        let mut t = term(4, 1);
        feed(&mut t, "\x1b[38;5;9m\u{10EEEE}\u{0305}\u{030D}".as_bytes());
        assert_eq!(
            placeholder_at(&t.grid()[Line(0)][Column(0)]),
            Some(Placeholder { image: 9, placement: 0, row: 0, col: 1 })
        );
        // A placeholder without its marks says nothing.
        let mut t = term(4, 1);
        feed(&mut t, "\u{10EEEE}".as_bytes());
        assert_eq!(placeholder_at(&t.grid()[Line(0)][Column(0)]), None);
    }

    #[test]
    fn placeholders_leave_copied_text() {
        let text = format!("ok {PLACEHOLDER}\u{0305}\u{030D}{PLACEHOLDER}\u{0305}\u{030E} done");
        assert_eq!(strip_placeholders(&text), "ok  done");
        assert_eq!(strip_placeholders("plain"), "plain");
        // A real combining mark after text stays.
        assert_eq!(strip_placeholders("e\u{0305}"), "e\u{0305}");
    }
}
