//! Moving the view through the history.

/// A move of the view through the history.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Scroll {
    /// This many lines up (positive) or down (negative).
    Delta(i32),
    /// To the oldest line of the history.
    Top,
    /// To the live screen.
    Bottom,
}
