//! What a row's size (DECDWL, DECDHL) means for drawing and pointing: how much wider its cells
//! are, which part of a double-height line it shows, and which cells it shows at all.
//!
//! A double-width or double-height row shows the first half of the columns, each cell twice as
//! wide; the cells past the half are in the engine's row, hidden. A double-height line is two rows,
//! and each draws the whole line's text at twice the size, cut to the row: the top half on the first
//! row, the bottom half on the second.

use wormhole_core::terminal::types::LineSize;

/// How many times wider a cell of a row of this size is drawn than a normal one.
pub(super) fn width_scale(size: LineSize) -> f32 {
    if size.is_double() { 2.0 } else { 1.0 }
}

/// Which part of its line a row shows, for the rows of a double-height line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Half {
    Top,
    Bottom,
}

/// The half of a double-height line `size` shows; `None` for a row that is not one.
pub(super) fn half(size: LineSize) -> Option<Half> {
    match size {
        LineSize::DoubleHeightTop => Some(Half::Top),
        LineSize::DoubleHeightBottom => Some(Half::Bottom),
        _ => None,
    }
}

/// How many times taller than a normal row the text of this row is drawn (the line the row is
/// half of, for a double-height one).
pub(super) fn height_scale(size: LineSize) -> f32 {
    if size.is_double_height() { 2.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use wormhole_core::terminal::types::LineSize;

    use super::{Half, half, height_scale, width_scale};

    #[test]
    fn a_sized_row_is_twice_as_wide_and_a_tall_one_twice_as_tall() {
        assert_eq!((width_scale(LineSize::Single), height_scale(LineSize::Single)), (1.0, 1.0));
        assert_eq!((width_scale(LineSize::DoubleWidth), height_scale(LineSize::DoubleWidth)), (2.0, 1.0));
        for size in [LineSize::DoubleHeightTop, LineSize::DoubleHeightBottom] {
            assert_eq!((width_scale(size), height_scale(size)), (2.0, 2.0));
        }
        assert_eq!(half(LineSize::DoubleHeightTop), Some(Half::Top));
        assert_eq!(half(LineSize::DoubleHeightBottom), Some(Half::Bottom));
        assert_eq!(half(LineSize::DoubleWidth), None);
    }
}
