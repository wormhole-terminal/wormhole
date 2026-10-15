//! Positions in the grid.
//!
//! Lines are signed: line 0 is the top row of the live screen and the history above it is
//! negative, so a position keeps its meaning while the view scrolls (the view's offset is added
//! only to draw).

use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// A line of the grid: 0 is the live screen's top row, -1 the newest line of history.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Ord, PartialOrd, Hash)]
pub struct Line(pub i32);

/// A column of the grid, from 0.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Ord, PartialOrd, Hash)]
pub struct Column(pub usize);

impl Line {
    /// `self + lines`, stopping at the ends of `i32` instead of overflowing (a count from the
    /// stream can be anything).
    #[inline]
    pub const fn saturating_add(self, lines: i32) -> Line {
        Line(self.0.saturating_add(lines))
    }

    /// `self - lines`, stopping at the ends of `i32`.
    #[inline]
    pub const fn saturating_sub(self, lines: i32) -> Line {
        Line(self.0.saturating_sub(lines))
    }
}

impl Column {
    /// `self + columns`, stopping at `usize::MAX`.
    #[inline]
    pub const fn saturating_add(self, columns: usize) -> Column {
        Column(self.0.saturating_add(columns))
    }

    /// `self - columns`, stopping at column 0.
    #[inline]
    pub const fn saturating_sub(self, columns: usize) -> Column {
        Column(self.0.saturating_sub(columns))
    }
}

impl Add<i32> for Line {
    type Output = Line;

    #[inline]
    fn add(self, lines: i32) -> Line {
        Line(self.0 + lines)
    }
}

impl Sub<i32> for Line {
    type Output = Line;

    #[inline]
    fn sub(self, lines: i32) -> Line {
        Line(self.0 - lines)
    }
}

impl AddAssign<i32> for Line {
    #[inline]
    fn add_assign(&mut self, lines: i32) {
        self.0 += lines;
    }
}

impl SubAssign<i32> for Line {
    #[inline]
    fn sub_assign(&mut self, lines: i32) {
        self.0 -= lines;
    }
}

impl Add<usize> for Column {
    type Output = Column;

    #[inline]
    fn add(self, columns: usize) -> Column {
        Column(self.0 + columns)
    }
}

impl Sub<usize> for Column {
    type Output = Column;

    #[inline]
    fn sub(self, columns: usize) -> Column {
        Column(self.0 - columns)
    }
}

impl AddAssign<usize> for Column {
    #[inline]
    fn add_assign(&mut self, columns: usize) {
        self.0 += columns;
    }
}

impl SubAssign<usize> for Column {
    #[inline]
    fn sub_assign(&mut self, columns: usize) {
        self.0 -= columns;
    }
}

/// A cell's place in the grid. Points order the way text reads: by line, then by column.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct Point {
    pub line: Line,
    pub column: Column,
}

impl Point {
    #[inline]
    pub const fn new(line: Line, column: Column) -> Point {
        Point { line, column }
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Point) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Point) -> Ordering {
        self.line.cmp(&other.line).then(self.column.cmp(&other.column))
    }
}

/// Which half of a cell a pointer is on. A selection that starts on a cell's right half begins
/// after that cell, and one that ends on its left half stops before it.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Side {
    Left,
    Right,
}

#[cfg(test)]
mod tests {
    use super::{Column, Line, Point};

    #[test]
    fn points_order_the_way_text_reads() {
        let at = |line, column| Point::new(Line(line), Column(column));
        assert!(at(1, 0) > at(0, 9));
        assert!(at(0, 1) > at(0, 0));
        assert!(at(0, 0) > at(-1, 5), "history is above the screen");
        assert_eq!(at(2, 3), at(2, 3));
    }

    #[test]
    fn saturating_steps_stop_at_the_ends() {
        assert_eq!(Line(i32::MAX).saturating_add(5), Line(i32::MAX));
        assert_eq!(Line(i32::MIN).saturating_sub(5), Line(i32::MIN));
        assert_eq!(Line(-3).saturating_add(5), Line(2));
        assert_eq!(Line(0).saturating_sub(1), Line(-1));
        assert_eq!(Column(2).saturating_sub(5), Column(0));
        assert_eq!(Column(usize::MAX).saturating_add(1), Column(usize::MAX));
        assert_eq!(Column(2).saturating_add(5), Column(7));
    }

    #[test]
    fn lines_and_columns_step() {
        let mut line = Line(0);
        line -= 1;
        assert_eq!(line + 3, Line(2));
        assert_eq!(Line(5) - 6, Line(-1));
        let mut column = Column(2);
        column += 1;
        assert_eq!((column - 1, column + 4), (Column(2), Column(7)));
    }
}
