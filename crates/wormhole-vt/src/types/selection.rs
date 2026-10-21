//! Selecting: what a mouse selection extends by, and the cells it covers.

use super::index::Point;

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
