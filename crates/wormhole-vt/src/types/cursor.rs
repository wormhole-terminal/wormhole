//! The cursor's look.

/// How the cursor is drawn (DECSCUSR's block, underline and bar, and two a host adds: an outline for a window that is not in front, and none at all).
#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Hash)]
pub enum CursorShape {
    /// A filled cell (DECSCUSR 1 and 2).
    #[default]
    Block,
    /// A line under the cell (DECSCUSR 3 and 4).
    Underline,
    /// A vertical bar at the cell's left edge (DECSCUSR 5 and 6).
    Beam,
    /// The outline of a block: a block cursor in a window that isn't in front.
    HollowBlock,
    /// Not drawn (DECTCEM off).
    Hidden,
}
