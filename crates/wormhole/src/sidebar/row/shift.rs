//! A row whose terminal's status is changing: how the change is played over the row's parts (`face`
//! reads it into the tile, the wash and the second line), and the second line the row had, which the
//! Line rolls variant rolls away from.

use super::second_line::SecondLine;
use crate::style::StatusShift;

/// The change a row is playing: what its tile came from and how far along it is, and the second line it
/// said before (kept only for the variant that rolls it).
#[derive(Clone, Debug)]
pub(in crate::sidebar) struct RowShift {
    pub(in crate::sidebar) shift: StatusShift,
    pub(in crate::sidebar) was: Option<SecondLine>,
}
