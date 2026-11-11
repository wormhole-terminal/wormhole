//! The terminal's cell, from the font's own measurements, and how many of them a pane holds: the one rule
//! both apps size and place a grid by (the GPUI app's `terminal_view/render/prepare.rs`, the Swift renderer
//! through the FFI), so the same font, size and window give the same columns, rows and baselines in each.
//!
//! Everything is in points. The cell is as wide as the font's `m` advances, unrounded: 8.4 pt for JetBrains
//! Mono at 14 pt, so at 2x the cells' edges fall on 16 and 17 device pixels by turns, and a glyph is placed at
//! its cell's own fractional position (the renderers keep subpixel variants for that) rather than on a
//! rounded grid that would hold other columns than GPUI's.

/// The thickness of the lines the terminal paints through and under text (underline, strikethrough,
/// overline), in points, whatever the font says.
pub const LINE: f32 = 1.0;

/// How far into a font's descent an underline sits (GPUI's `UNDERLINE_DESCENT_OFFSET_FACTOR`).
const UNDERLINE_INTO_DESCENT: f32 = 0.618;

/// One cell and where text sits in it, in points from the cell's top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub width: f32,
    pub height: f32,
    /// The baseline: the text is centred in the line height, as CSS does.
    pub baseline: f32,
    /// The top of an underline.
    pub underline: f32,
    /// The top of a strikethrough.
    pub strike: f32,
}

impl Cell {
    /// `advance` is the advance of `m` at `font_size`; `ascent` and `descent` are the font's at that size
    /// (descent positive); `line_height` is a multiple of the size (1.3 by default).
    pub fn new(font_size: f32, line_height: f32, advance: f32, ascent: f32, descent: f32) -> Self {
        let height = (font_size * line_height).round();
        let baseline = (height - ascent - descent) / 2.0 + ascent;
        Self {
            width: advance,
            height,
            baseline,
            // GPUI's `underline_y_offset` over the font's metrics, whose descent is negative: it lands in the
            // centring padding with that sign, which puts the line a little higher than "0.618 into the descent".
            // Kept as it is drawn today.
            underline: (height - ascent + descent) / 2.0 + ascent - descent * UNDERLINE_INTO_DESCENT,
            strike: (ascent * 0.5 + baseline) * 0.5,
        }
    }

    /// The whole cells an area of `width` x `height` points holds: at least two columns and one row. `slack`
    /// is how far short of a whole cell the area may be and still have it (a screen pixel, for a window whose
    /// size AppKit rounds to pixels; zero otherwise).
    pub fn fit(&self, width: f32, height: f32, slack: f32) -> (usize, usize) {
        let cols = ((width + slack) / self.width).floor().max(2.0) as usize;
        let rows = ((height + slack) / self.height).floor().max(1.0) as usize;
        (cols, rows)
    }
}

#[cfg(test)]
mod tests {
    use super::Cell;

    /// JetBrains Mono at 14 pt: 600/1000 em advance, ascent 1020 and descent 300.
    fn jetbrains() -> Cell {
        Cell::new(14.0, 1.3, 8.4, 14.28, 4.2)
    }

    #[test]
    fn the_cell_is_the_advance_wide_and_the_rounded_line_high() {
        let cell = jetbrains();
        assert_eq!((cell.width, cell.height), (8.4, 18.0));
        assert!((cell.baseline - 14.04).abs() < 1e-4, "{}", cell.baseline);
        assert!(cell.underline > cell.baseline && cell.underline < cell.height);
        assert!(cell.strike < cell.baseline);
    }

    #[test]
    fn a_pane_holds_whole_cells_and_never_fewer_than_two_columns() {
        let cell = jetbrains();
        assert_eq!(cell.fit(952.0, 774.0, 0.0), (113, 43));
        assert_eq!(cell.fit(8.4 * 20.0 - 0.3, 18.0, 0.0).0, 19);
        assert_eq!(cell.fit(8.4 * 20.0 - 0.3, 18.0, 0.5).0, 20);
        assert_eq!(cell.fit(1.0, 1.0, 0.0), (2, 1));
    }
}
