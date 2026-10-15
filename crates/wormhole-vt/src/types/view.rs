//! How the screen is looked at and pointed at: scrolling, selecting, the cursor.

use super::index::Point;

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

/// What a mouse selection extends by.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectionType {
    /// Cell to cell, in reading order.
    Simple,
    /// A rectangle.
    Block,
    /// Whole words (a double click).
    Semantic,
    /// Whole lines (a triple click).
    Lines,
}

/// The cells a selection covers, in grid coordinates.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SelectionRange {
    /// The first cell in reading order (top left of a block).
    pub start: Point,
    /// The last (bottom right of a block).
    pub end: Point,
    /// A rectangle: every row covers the same columns.
    pub is_block: bool,
}

/// How the cursor is drawn (DECSCUSR's block, underline and bar, and two the app adds).
#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Hash)]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Beam,
    /// The outline of a block: a block cursor in a window that isn't in front.
    HollowBlock,
    /// Not drawn (DECTCEM off).
    Hidden,
}
