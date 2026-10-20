//! kitty's Unicode placeholders: a picture a program prints as text.
//!
//! Spec: kitty, "Unicode placeholders". A program that cannot place a picture itself (it is
//! behind tmux, or draws into a scrolling text UI) declares a *virtual placement* (`a=p` or
//! `a=T` with `U=1`: the picture, an optional placement id, and the cells it spans) and then
//! prints `U+10EEEE` in cells. The cell's foreground color carries the picture's id (its low 24
//! bits; the high byte rides in a third combining mark), its underline color the placement id,
//! and the first two combining marks, drawn from a table of 297 diacritics, the row and the
//! column of the picture's tile the cell shows. The cells are ordinary text: they scroll, wrap,
//! reflow and are overwritten as text does, and every cell says on its own which tile it is.
//!
//! The host draws each placeholder cell it meets as that tile of that picture, sized by the
//! virtual placement ([`Terminal::virtual_placement`](crate::terminal::Terminal::virtual_placement)).
//! The engine only decodes the cell ([`placeholder_at`]) and strips the placeholders from text
//! that is copied ([`strip_placeholders`]).

use std::borrow::Cow;

use crate::types::{Cell, CellExtra, Color};

pub use crate::types::PLACEHOLDER;

/// How many rows and columns a placement of placeholders can count: the table's length.
pub const DIACRITICS_LEN: usize = 297;

/// kitty's row and column diacritics (the published `rowcolumn-diacritics.txt`), in order and
/// ascending.
const DIACRITICS: [char; DIACRITICS_LEN] = [
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

/// One cell of a placement: which tile of which picture it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placeholder {
    /// The picture's id as the program numbered it.
    pub image_id: u32,
    /// The placement's id, 0 when the program gave none.
    pub placement_id: u32,
    /// The tile's row in the picture.
    pub row: u16,
    /// The tile's column in the picture.
    pub col: u16,
}

/// The 24 bits a color carries for an id: a true color's RGB, or an indexed color's number.
fn id_bits(color: Color) -> u32 {
    match color {
        Color::Rgb(rgb) => u32::from(rgb.r) << 16 | u32::from(rgb.g) << 8 | u32::from(rgb.b),
        Color::Indexed(index) => u32::from(index),
        Color::Named(_) => 0,
    }
}

/// What `cell` shows, if it is a placeholder with its row and column marks. `extra` is what its
/// row carries beside it ([`RowExtras::cell`](crate::types::RowExtras::cell)): the marks and the
/// placement id.
pub fn placeholder_at(cell: &Cell, extra: CellExtra<'_>) -> Option<Placeholder> {
    if cell.c != PLACEHOLDER {
        return None;
    }
    let mut marks = extra.zerowidth.iter().filter_map(|c| index_of(*c));
    let row = marks.next()?;
    let col = marks.next()?;
    let high = u32::from(marks.next().unwrap_or(0));
    let image_id = high << 24 | id_bits(cell.fg);
    let placement_id = extra.underline_color.map_or(0, id_bits);
    Some(Placeholder { image_id, placement_id, row, col })
}

/// `text` without its placeholders and their marks, for the clipboard and a saved scrollback: a
/// copied picture would otherwise be a row of private-use characters.
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
    use super::{DIACRITICS, DIACRITICS_LEN, PLACEHOLDER, Placeholder, placeholder_at, strip_placeholders};
    use crate::types::{Cell, CellExtra, Color, Flags, Rgb};

    fn cell(fg: Color) -> Cell {
        Cell { c: PLACEHOLDER, fg, bg: Color::Indexed(0), flags: Flags::empty() }
    }

    #[test]
    fn the_table_is_kittys_297_marks_ascending() {
        assert_eq!(DIACRITICS.len(), DIACRITICS_LEN);
        assert!(DIACRITICS.windows(2).all(|w| w[0] < w[1]));
        assert_eq!((DIACRITICS[0], DIACRITICS[296]), ('\u{0305}', '\u{1D244}'));
    }

    #[test]
    fn a_cell_says_its_picture_placement_and_tile() {
        let marks = [DIACRITICS[2], DIACRITICS[5]];
        let extra =
            CellExtra { zerowidth: &marks, underline_color: Some(Color::Rgb(Rgb { r: 0, g: 1, b: 2 })) };
        let at = placeholder_at(&cell(Color::Rgb(Rgb { r: 1, g: 2, b: 3 })), extra);
        assert_eq!(at, Some(Placeholder { image_id: 0x01_02_03, placement_id: 0x01_02, row: 2, col: 5 }));
        // The high byte of a big id rides in the third mark; a 256-color id is its number.
        let marks = [DIACRITICS[0], DIACRITICS[1], DIACRITICS[0xAB]];
        let extra = CellExtra { zerowidth: &marks, underline_color: None };
        assert_eq!(placeholder_at(&cell(Color::Indexed(9)), extra).map(|p| p.image_id), Some(0xAB00_0009));
        // Marks that are not in the table are not tile numbers; without two, it says nothing.
        let other = ['\u{0301}', DIACRITICS[1]];
        let extra = CellExtra { zerowidth: &other, underline_color: None };
        assert_eq!(placeholder_at(&cell(Color::Indexed(9)), extra), None);
        let plain = Cell { c: 'x', ..cell(Color::Indexed(9)) };
        assert_eq!(placeholder_at(&plain, CellExtra::default()), None);
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
