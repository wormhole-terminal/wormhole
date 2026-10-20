//! A row's metadata, and the types the terminal writes cells with.

use super::cell::{BlockPart, CellFlags};
use super::style::Style;
use crate::types::{Hyperlink, LineSize};

/// The row continues in the next one: the line was wrapped by the terminal at the margin, not
/// ended by the program.
pub(super) const WRAPPED: u8 = 1;
/// The row changed since the snapshot last copied it.
pub(super) const DIRTY: u8 = 1 << 1;
/// Where the row's [`LineSize`] sits in the flags: two bits, so a normal row (zero) costs nothing
/// and the size moves with the record when scrolling rotates records.
pub(super) const SIZE_SHIFT: u8 = 2;
pub(super) const SIZE_MASK: u8 = 0b11 << SIZE_SHIFT;

/// A row of a page: which slot of the page's cell buffer holds its cells, and what the terminal
/// knows about the line.
///
/// A row is moved by moving this record: scrolling a region inside a page rotates records and
/// copies no cell. The slot stays with the cells, and every side-table key is built from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Row {
    /// The number of the command block (OSC 133) whose cells are on the row, 0 for none. Where two
    /// blocks meet on one row it is the later one's.
    pub(super) block: u32,
    /// A page has fewer cells than 65,535, so fewer slots than a `u16` counts.
    pub(super) slot: u16,
    pub(super) flags: u8,
}

impl Row {
    #[inline]
    pub(super) fn new(slot: usize) -> Row {
        Row { slot: slot as u16, block: 0, flags: DIRTY }
    }

    #[inline]
    pub(super) fn wrapped(&self) -> bool {
        self.flags & WRAPPED != 0
    }

    /// How big the row is drawn.
    #[inline]
    pub(super) fn size(&self) -> LineSize {
        LineSize::from_bits((self.flags & SIZE_MASK) >> SIZE_SHIFT)
    }

    /// What a row copied over another takes along.
    #[inline]
    pub(super) fn meta(&self) -> Meta {
        Meta { wrapped: self.wrapped(), block: self.block, size: self.size() }
    }
}

/// What a row carries besides its cells, for the copies that move a row whole (a scroll across
/// pages, a crop): the wrap flag, the command block and the line size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Meta {
    pub(super) wrapped: bool,
    pub(super) block: u32,
    pub(super) size: LineSize,
}

/// What a row carries of a command block (shell integration, OSC 133), read back from its cells:
/// the block's number and which parts of it are on the row. A prompt row carries the prompt and,
/// once something is typed, the command; a long command's wrapped row the command alone; the rest
/// the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RowMark {
    /// The block's number: counts up from 1 as prompts are marked, never 0.
    pub block: u32,
    /// The row has cells of the block's [prompt](BlockPart::Prompt).
    pub prompt: bool,
    /// The row has cells of the block's [command line](BlockPart::Command).
    pub command: bool,
    /// The row has cells of the block's [output](BlockPart::Output).
    pub output: bool,
}

impl RowMark {
    pub(super) fn from_parts(block: u32, parts: u8) -> Option<RowMark> {
        (block != 0 && parts != 0).then_some(RowMark {
            block,
            prompt: parts & (1 << BlockPart::Prompt as u8) != 0,
            command: parts & (1 << BlockPart::Command as u8) != 0,
            output: parts & (1 << BlockPart::Output as u8) != 0,
        })
    }
}

/// What the terminal writes cells with: its pen. Kept by the terminal, borrowed for each write.
#[derive(Clone, Debug, Default)]
pub struct Pen {
    pub style: Style,
    /// The hyperlink (OSC 8) the cells are under.
    pub link: Option<Hyperlink>,
    /// What the cells are part of in a shell-integrated session; [`BlockPart::None`] without one.
    pub content: BlockPart,
    /// The command block the content belongs to; recorded on the row when `content` is set.
    pub block: u32,
    /// DECSCA: selective erase leaves the cells alone.
    pub protected: bool,
    /// SPA to EPA: ED, EL and ECH leave the cells alone.
    pub guarded: bool,
}

impl Pen {
    #[inline]
    pub(super) fn cell_flags(&self) -> CellFlags {
        let protected = if self.protected { CellFlags::PROTECTED } else { CellFlags::empty() };
        let guarded = if self.guarded { CellFlags::GUARDED } else { CellFlags::empty() };
        protected | guarded | CellFlags::with_content(self.content)
    }
}

/// A rectangle of the screen in cells, edges included: the scroll region between the top and bottom
/// margins (DECSTBM) and the left and right ones (DECLRMM).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub top: usize,
    pub bottom: usize,
    pub left: usize,
    pub right: usize,
}

impl Region {
    /// The whole screen of this size.
    pub fn full(cols: usize, rows: usize) -> Region {
        Region { top: 0, bottom: rows.saturating_sub(1), left: 0, right: cols.saturating_sub(1) }
    }
}
