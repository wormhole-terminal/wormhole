//! A row's size: single, double-width, or one half of a double-height line.

/// How big a row is drawn. DEC terminals set it per row with `ESC # n`: a row is drawn with every
/// cell twice as wide (DECDWL, `ESC # 6`), or twice as wide and twice as tall (DECDHL), the
/// tall line being two rows, the upper half drawn on the first (`ESC # 3`) and the lower half on
/// the one below it (`ESC # 4`), which carries the same text. DECSWL (`ESC # 5`) makes a row
/// single again.
///
/// A host that draws a row of another size than [`Single`](LineSize::Single) shows only the first
/// half of its columns, each cell twice as wide ([`visible_columns`](LineSize::visible_columns));
/// the cells beyond them are still in the row, hidden (a row made single again shows what it had),
/// and the cursor never goes there. The tall rows' glyphs are drawn at twice the height and cut to
/// the row: the top half on a [`DoubleHeightTop`](LineSize::DoubleHeightTop) row, the bottom half
/// on a [`DoubleHeightBottom`](LineSize::DoubleHeightBottom) one.
///
/// Spec: the VT510 manual (DECSWL, DECDWL, DECDHL) and xterm's ctlseqs ("ESC # 3" to "ESC # 6").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LineSize {
    /// Normal: one cell is one column. What every row is until a program says otherwise.
    #[default]
    Single,
    /// DECDWL: every cell is two columns wide.
    DoubleWidth,
    /// DECDHL, the upper half of the line: two columns wide and twice as tall, the top half shown.
    DoubleHeightTop,
    /// DECDHL, the lower half of the line: two columns wide and twice as tall, the bottom half
    /// shown.
    DoubleHeightBottom,
}

impl LineSize {
    /// Whether cells are drawn twice as wide (every size but [`Single`](LineSize::Single)).
    #[must_use]
    pub fn is_double(self) -> bool {
        self != LineSize::Single
    }

    /// Whether the row is one half of a double-height line.
    #[must_use]
    pub fn is_double_height(self) -> bool {
        matches!(self, LineSize::DoubleHeightTop | LineSize::DoubleHeightBottom)
    }

    /// How many of a screen's `cols` columns a row of this size shows and lets the cursor reach:
    /// all of them for a single row, the first half (rounded down, at least 1) for the others.
    #[must_use]
    pub fn visible_columns(self, cols: usize) -> usize {
        if self.is_double() { (cols / 2).max(1) } else { cols }
    }

    /// The two bits a row record stores.
    #[inline]
    pub(crate) fn to_bits(self) -> u8 {
        match self {
            LineSize::Single => 0,
            LineSize::DoubleWidth => 1,
            LineSize::DoubleHeightTop => 2,
            LineSize::DoubleHeightBottom => 3,
        }
    }

    #[inline]
    pub(crate) fn from_bits(bits: u8) -> LineSize {
        match bits & 3 {
            0 => LineSize::Single,
            1 => LineSize::DoubleWidth,
            2 => LineSize::DoubleHeightTop,
            _ => LineSize::DoubleHeightBottom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LineSize;

    #[test]
    fn only_the_two_halves_of_a_tall_line_are_double_height() {
        let heights = [
            (LineSize::Single, false),
            (LineSize::DoubleWidth, false),
            (LineSize::DoubleHeightTop, true),
            (LineSize::DoubleHeightBottom, true),
        ];
        for (size, tall) in heights {
            assert_eq!(size.is_double_height(), tall, "{size:?}");
            assert_eq!(size.is_double(), size != LineSize::Single, "{size:?}");
        }
    }
}
