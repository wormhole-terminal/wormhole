//! The pen: what the terminal writes cells with.

use super::cell::{BlockPart, CellFlags};
use super::style::Style;
use crate::types::Hyperlink;

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
