//! The grid's own cell: eight bytes, `Copy`, and zero when blank.
//!
//! Bits 0..32 are the code point (0 for a cell nothing was written to), bits 32..48 the id of the
//! cell's style in its page's table, bits 48..64 the flags below. A blank cell is all zero bits, so
//! fresh memory (a `vec![0; n]` is a lazily zeroed `calloc`) is a screen of blank cells, and a row
//! is blank when its words are.
//!
//! Everything a cell can have beyond that lives beside it in the page, found by the cell's
//! position: the zero-width characters of a cluster, a hyperlink. The flags say whether to look.
//! The types a host draws ([`crate::types::Cell`], 16 bytes) are made from these in the snapshot
//! copy.

use std::fmt;

use bitflags::bitflags;

use crate::types::Flags;

/// An index into a page's style table; 0 is the default style (default colors, no attributes) of
/// every page.
pub type StyleId = u16;

bitflags! {
    /// The flag bits of a cell (bits 48..64 of its word). Bits 6 and 7 are a [`BlockPart`], which has
    /// no flag of its own: they pass through [`CellFlags::from_bits_retain`] and
    /// [`CellFlags::content`].
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(super) struct CellFlags: u16 {
        /// The first cell of a double-width character; the next cell is its `SPACER_TAIL`.
        const WIDE = 1;
        /// The second cell of a double-width character: no character of its own, the same style.
        const SPACER_TAIL = 1 << 1;
        /// The last cell of a row, left blank because a double-width character didn't fit and went
        /// to the next row (the row is soft-wrapped).
        const SPACER_HEAD = 1 << 2;
        /// The cell has characters after its first in the page's grapheme table.
        const GRAPHEME = 1 << 3;
        /// The cell is under a hyperlink in the page's link table.
        const HYPERLINK = 1 << 4;
        /// DECSCA: selective erase leaves the cell alone.
        const PROTECTED = 1 << 5;
        /// Unused: placements are a table of the grid's, kept by row (`images`), and the cells under
        /// one are ordinary. The bit is kept free for a use that needs a mark on the cell itself.
        const IMAGE = 1 << 8;
        /// HT started on this blank cell: a copy of the text turns it and the blanks up to the next
        /// tab stop, while they are all still blank, into one `\t`. Writing the cell or erasing it
        /// takes the mark away with the rest of its flags.
        const TAB = 1 << 9;
        /// SPA/EPA (ISO 6429 guarded area): ED, EL and ECH leave the cell alone. Separate from
        /// `PROTECTED`: DECSCA's protection is for the selective erases only (VT510, DECSCA).
        const GUARDED = 1 << 10;
    }
}

/// Where the two bits of a [`BlockPart`] sit in the flags.
const CONTENT_SHIFT: u16 = 6;

impl CellFlags {
    /// The flags that mean a cell has an entry beside the page's cells.
    pub(super) const SIDE_ENTRY: CellFlags =
        CellFlags::GRAPHEME.union(CellFlags::HYPERLINK).union(CellFlags::IMAGE);

    /// What a cell that is part of `content` carries.
    #[inline]
    pub(super) const fn with_content(content: BlockPart) -> CellFlags {
        CellFlags::from_bits_retain((content as u16) << CONTENT_SHIFT)
    }

    /// What the cell is part of.
    #[inline]
    pub(super) const fn content(self) -> BlockPart {
        BlockPart::from_bits(self.bits() >> CONTENT_SHIFT)
    }

    /// The flags of the OR of some cells' words (see [`Cell::bits`]): what any of them carries.
    #[inline]
    pub(super) const fn in_word(word: u64) -> CellFlags {
        CellFlags::from_bits_retain((word >> FLAGS_SHIFT) as u16)
    }
}

// The cell word: code point in bits 0..32, style id in 32..48, flags in 48..64.
const STYLE_SHIFT: u32 = 32;
const FLAGS_SHIFT: u32 = 48;
const CODEPOINT_MASK: u64 = 0xFFFF_FFFF;
/// The style id's bits of a cell word.
const STYLE_BITS: u64 = 0xFFFF << STYLE_SHIFT;
/// The bits of a cell word that mean a cell can't just be overwritten: it holds a count in the
/// style table, an entry in a side table or an image. One OR over a range's words, one test.
const NEEDS_RELEASE: u64 = STYLE_BITS | ((CellFlags::SIDE_ENTRY.bits() as u64) << FLAGS_SHIFT);

// `copy_page_row` hands the three layout flags to the host as `types::Flags` by shifting them to
// bits 13..16, which only works while they sit in the same order.
const DRAW_SHIFT: u32 = 13;
const _: () = {
    assert!((CellFlags::WIDE.bits() as u32) << DRAW_SHIFT == Flags::WIDE_CHAR.bits());
    assert!((CellFlags::SPACER_TAIL.bits() as u32) << DRAW_SHIFT == Flags::WIDE_CHAR_SPACER.bits());
    assert!((CellFlags::SPACER_HEAD.bits() as u32) << DRAW_SHIFT == Flags::LEADING_WIDE_CHAR_SPACER.bits());
};
/// The three layout flags (wide, tail, head) as one mask.
const LAYOUT: CellFlags = CellFlags::WIDE.union(CellFlags::SPACER_TAIL).union(CellFlags::SPACER_HEAD);

/// The names of the cell's flags, for the test that every one has a case in the repaint
/// (`term::repaint::CELL_FLAGS`).
#[doc(hidden)]
#[cfg(any(test, feature = "internals"))]
pub fn flag_names() -> Vec<&'static str> {
    CellFlags::all().iter_names().map(|(name, _)| name).collect()
}

/// Which cells an erase leaves alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Spare {
    /// None: every cell is erased.
    #[default]
    Nothing,
    /// The cells DECSCA protected (DECSED, DECSEL, DECSERA).
    Selective,
    /// The cells inside a guarded area, SPA to EPA (ED, EL, ECH).
    Guarded,
}

impl From<bool> for Spare {
    /// `true` is the selective erase, `false` the plain one.
    fn from(selective: bool) -> Spare {
        if selective { Spare::Selective } else { Spare::Nothing }
    }
}

/// The part of a command block (shell integration, OSC 133) a cell belongs to: the prompt the shell
/// printed, the command line as it was typed, or the output of the command. The terminal writes it
/// from the shell's marks; a row's [`RowMark`](super::RowMark) says which parts it carries, and
/// [`Terminal::block_text`](crate::Terminal::block_text) reads one part back as text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum BlockPart {
    /// Not part of any block: text from before the first mark, or on the alternate screen.
    #[default]
    None = 0,
    /// The prompt (OSC 133 ; A).
    Prompt = 1,
    /// The command line the user typed (OSC 133 ; B).
    Command = 2,
    /// The command's output (OSC 133 ; C until D).
    Output = 3,
}

impl BlockPart {
    #[inline]
    pub(super) const fn from_bits(bits: u16) -> BlockPart {
        match bits & 3 {
            1 => BlockPart::Prompt,
            2 => BlockPart::Command,
            3 => BlockPart::Output,
            _ => BlockPart::None,
        }
    }
}

/// One cell; see the module docs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct Cell(u64);

impl Cell {
    /// Nothing written, the default style.
    pub const EMPTY: Cell = Cell(0);

    /// A cell from its parts; `flags` is [`Cell::flags`]'s form.
    #[inline]
    #[cfg(any(test, feature = "internals"))]
    pub const fn new(codepoint: u32, style: StyleId, flags: u16) -> Cell {
        Cell::compose(codepoint, style, CellFlags::from_bits_retain(flags))
    }

    /// A cell from its parts, with typed flags.
    #[inline]
    pub(super) const fn compose(codepoint: u32, style: StyleId, flags: CellFlags) -> Cell {
        Cell(codepoint as u64 | (style as u64) << STYLE_SHIFT | (flags.bits() as u64) << FLAGS_SHIFT)
    }

    /// The code point, 0 for a cell nothing was written to.
    #[inline]
    pub const fn codepoint(self) -> u32 {
        self.0 as u32
    }

    /// The character, `None` for an empty cell.
    #[inline]
    pub fn character(self) -> Option<char> {
        match self.codepoint() {
            0 => None,
            cp => char::from_u32(cp),
        }
    }

    #[inline]
    pub const fn style_id(self) -> StyleId {
        (self.0 >> STYLE_SHIFT) as u16
    }

    #[inline]
    pub const fn flags(self) -> u16 {
        (self.0 >> FLAGS_SHIFT) as u16
    }

    #[inline]
    pub const fn is_wide(self) -> bool {
        self.flag_set().contains(CellFlags::WIDE)
    }

    #[inline]
    pub const fn is_spacer_tail(self) -> bool {
        self.flag_set().contains(CellFlags::SPACER_TAIL)
    }

    #[inline]
    pub const fn is_spacer_head(self) -> bool {
        self.flag_set().contains(CellFlags::SPACER_HEAD)
    }

    #[inline]
    pub const fn has_grapheme(self) -> bool {
        self.flag_set().contains(CellFlags::GRAPHEME)
    }

    #[inline]
    pub const fn has_hyperlink(self) -> bool {
        self.flag_set().contains(CellFlags::HYPERLINK)
    }

    #[inline]
    pub const fn is_protected(self) -> bool {
        self.flag_set().contains(CellFlags::PROTECTED)
    }

    #[inline]
    pub const fn is_guarded(self) -> bool {
        self.flag_set().contains(CellFlags::GUARDED)
    }

    /// Whether an erase that spares `spare` must leave the cell as it is.
    #[inline]
    pub const fn is_spared(self, spare: Spare) -> bool {
        match spare {
            Spare::Nothing => false,
            Spare::Selective => self.is_protected(),
            Spare::Guarded => self.is_guarded(),
        }
    }

    #[inline]
    pub const fn content(self) -> BlockPart {
        self.flag_set().content()
    }

    /// Nothing written and no style: the cell of fresh memory.
    #[inline]
    pub const fn is_blank(self) -> bool {
        self.0 == 0
    }

    /// Blank but for a tab mark, which is invisible: nothing a line holds or a reflow keeps.
    #[inline]
    pub const fn is_blank_but_tab(self) -> bool {
        self.0 & !((CellFlags::TAB.bits() as u64) << FLAGS_SHIFT) == 0
    }

    /// The flags, typed.
    #[inline]
    pub(super) const fn flag_set(self) -> CellFlags {
        CellFlags::from_bits_retain(self.flags())
    }

    /// The cell's whole word, for loops that OR a row's words together to learn what any cell
    /// holds ([`CellFlags::in_word`], [`run_of`]).
    #[inline]
    pub(super) const fn bits(self) -> u64 {
        self.0
    }

    /// The style id's bits of the word, in place: equal for two cells exactly when their styles
    /// are, which is cheaper to compare than the ids are to extract.
    #[inline]
    pub(super) const fn style_bits(self) -> u64 {
        self.0 & STYLE_BITS
    }

    /// Whether the cell has an entry in one of the page's side tables (or an image).
    #[inline]
    pub(super) const fn has_side_entry(self) -> bool {
        self.flag_set().intersects(CellFlags::SIDE_ENTRY)
    }

    /// The flags other than the tab mark: what makes a blank more than a blank.
    #[inline]
    pub const fn flags_but_tab(self) -> u16 {
        self.flags() & !CellFlags::TAB.bits()
    }

    /// A tab (HT) started on this cell; see [`CellFlags::TAB`].
    #[inline]
    pub const fn is_tab_start(self) -> bool {
        self.flag_set().contains(CellFlags::TAB)
    }

    /// The wide, tail and head flags as a host draws them, for the snapshot copy.
    #[inline]
    pub(super) const fn layout_flags(self) -> Flags {
        Flags::from_bits_retain(((self.flags() & LAYOUT.bits()) as u32) << DRAW_SHIFT)
    }

    #[inline]
    pub(super) const fn with_flags_set(self, flags: CellFlags) -> Cell {
        Cell(self.0 | (flags.bits() as u64) << FLAGS_SHIFT)
    }

    #[inline]
    pub(super) const fn without_flags(self, flags: CellFlags) -> Cell {
        Cell(self.0 & !((flags.bits() as u64) << FLAGS_SHIFT))
    }

    #[inline]
    pub(super) const fn with_style(self, style: StyleId) -> Cell {
        Cell(self.0 & !STYLE_BITS | (style as u64) << STYLE_SHIFT)
    }

    /// The same cell holding `codepoint`.
    #[inline]
    pub(super) const fn with_codepoint(self, codepoint: u32) -> Cell {
        Cell(self.0 & !CODEPOINT_MASK | codepoint as u64)
    }
}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cell({:?} style {} flags {:#06x})", self.character(), self.style_id(), self.flags())
    }
}

/// What a run of cells holds that has to be given back before they are overwritten, from one pass
/// over their words (an OR and an AND, which vectorize).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StyleRun {
    /// Every cell has the default style and nothing in a side table: nothing to give back.
    Plain,
    /// Every cell has this one style (not the default) and nothing in a side table: one count
    /// stands for the whole run.
    Uniform(StyleId),
    /// Several styles, or entries in the side tables: the cells are visited one by one.
    Mixed,
}

/// Which [`StyleRun`] `cells` are.
#[inline]
pub(super) fn run_of(cells: &[Cell]) -> StyleRun {
    let (mut any, mut all) = (0u64, !0u64);
    for cell in cells {
        any |= cell.0;
        all &= cell.0;
    }
    if any & NEEDS_RELEASE == 0 {
        StyleRun::Plain
    } else if any & !STYLE_BITS & NEEDS_RELEASE == 0 && (any ^ all) & STYLE_BITS == 0 {
        StyleRun::Uniform((any >> STYLE_SHIFT) as u16)
    } else {
        StyleRun::Mixed
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockPart, Cell, CellFlags, StyleRun, run_of};
    use crate::types::Flags;

    #[test]
    fn a_cell_is_eight_bytes_and_blank_is_zero() {
        assert_eq!(std::mem::size_of::<Cell>(), 8);
        assert!(Cell::EMPTY.is_blank());
        assert_eq!(Cell::default(), Cell::EMPTY);
        assert_eq!(Cell::EMPTY.character(), None);
    }

    #[test]
    fn the_fields_do_not_overlap() {
        let flags = CellFlags::WIDE | CellFlags::PROTECTED | CellFlags::with_content(BlockPart::Output);
        let cell = Cell::compose('日' as u32, 0xBEEF, flags);
        assert_eq!(cell, Cell::new('日' as u32, 0xBEEF, flags.bits()));
        assert_eq!(cell.character(), Some('日'));
        assert_eq!(cell.style_id(), 0xBEEF);
        assert!(cell.is_wide() && cell.is_protected() && !cell.has_grapheme());
        assert_eq!(cell.content(), BlockPart::Output);
        assert_eq!(cell.with_style(3).style_id(), 3);
        assert_eq!(cell.with_style(3).character(), Some('日'));
        assert_eq!(cell.with_codepoint('x' as u32).flag_set(), flags);
        assert_eq!(cell.with_codepoint('x' as u32).style_id(), 0xBEEF);
        assert!(cell.with_flags_set(CellFlags::GRAPHEME).has_grapheme());
        assert_eq!(cell.with_flags_set(CellFlags::GRAPHEME).without_flags(CellFlags::GRAPHEME), cell);
        assert_eq!(cell.without_flags(CellFlags::WIDE).content(), BlockPart::Output);
        assert_eq!(cell.style_bits() >> 32, 0xBEEF);
        assert_eq!(CellFlags::in_word(cell.bits()), flags);
    }

    #[test]
    fn the_layout_flags_are_the_drawn_ones() {
        let cell = |flags| Cell::compose(0, 0, flags);
        assert_eq!(cell(CellFlags::WIDE).layout_flags(), Flags::WIDE_CHAR);
        assert_eq!(cell(CellFlags::SPACER_TAIL).layout_flags(), Flags::WIDE_CHAR_SPACER);
        assert_eq!(cell(CellFlags::SPACER_HEAD).layout_flags(), Flags::LEADING_WIDE_CHAR_SPACER);
        let others = CellFlags::GRAPHEME | CellFlags::HYPERLINK | CellFlags::PROTECTED | CellFlags::IMAGE;
        assert_eq!(cell(others | CellFlags::with_content(BlockPart::Prompt)).layout_flags(), Flags::empty());
    }

    #[test]
    fn what_must_be_released_is_found_in_one_pass() {
        let run = |cells: &[Cell]| run_of(cells);
        let plain = Cell::new('a' as u32, 0, 0);
        let styled = Cell::new('a' as u32, 1, 0);
        assert_eq!(run(&[plain, Cell::new('b' as u32, 0, CellFlags::WIDE.bits())]), StyleRun::Plain);
        assert_eq!(run(&[styled, styled]), StyleRun::Uniform(1));
        assert_eq!(run(&[styled, plain]), StyleRun::Mixed);
        assert_eq!(run(&[styled, Cell::new('a' as u32, 2, 0)]), StyleRun::Mixed);
        // An entry in a side table, or an image, always takes the walk, even in a uniform run.
        for flag in [CellFlags::GRAPHEME, CellFlags::HYPERLINK, CellFlags::IMAGE] {
            assert_eq!(run(&[plain, Cell::new(0, 0, flag.bits())]), StyleRun::Mixed);
            assert_eq!(run(&[styled, Cell::new(0, 1, flag.bits())]), StyleRun::Mixed);
        }
    }
}
