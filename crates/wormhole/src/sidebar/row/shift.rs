//! A row whose terminal's status is changing: how the change is played over the row's parts (`face`
//! reads it into the tile, the wash and the second line), and the second line the row had, which the
//! Line rolls variant rolls away from.

use gpui_kit::SharedString;

use crate::style::{RowTone, StatusShift};

/// What a row's second line said before its status changed: kept as the words that were drawn, so asking
/// for it again each frame of the change is a count and no copy.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::sidebar) struct Was {
    pub(in crate::sidebar) text: SharedString,
    pub(in crate::sidebar) tone: RowTone,
    /// The words were a command, set in the code font.
    pub(in crate::sidebar) mono: bool,
}

/// The change a row is playing: what its tile came from and how far along it is, and the second line it
/// said before (kept only for the variant that rolls it).
#[derive(Clone, Debug)]
pub(in crate::sidebar) struct RowShift {
    pub(in crate::sidebar) shift: StatusShift,
    pub(in crate::sidebar) was: Option<Was>,
}
