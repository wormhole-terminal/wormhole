//! A region of the screen: the scroll margins.

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
