//! How many cells a picture covers, and how it is sized in them. Pure arithmetic.
//!
//! The rules are the protocols': kitty ("Controlling displayed image layout": `c` and `r` give
//! cells, one alone keeps the picture's shape, both stretch it, neither shows it at its own size;
//! `x y w h` cut a part out; `X Y` shift it inside the first cell), iTerm2's width and height
//! units, and the VT340's pixels for a sixel, which cover the cells they touch.

use crate::types::{Extent, SourceRect};

/// Most rows one placement covers. A 10,000-pixel picture at an 8-pixel row is 1,250.
pub const ROWS_MAX: usize = 1024;
/// Most columns one placement covers (the widest grid).
const COLS_MAX: u64 = crate::grid::MAX_COLS as u64;

/// A placement's shape before it meets the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Geometry {
    pub cols: usize,
    pub rows: usize,
    pub source: SourceRect,
    pub extent: Extent,
    pub offset_x: u32,
    pub offset_y: u32,
}

fn ceil_div(a: u64, b: u64) -> u64 {
    a.div_ceil(b.max(1))
}

/// `value * num / den` rounded, at least 1.
fn scaled(value: u64, num: u64, den: u64) -> u64 {
    let den = u128::from(den.max(1));
    ((u128::from(value) * u128::from(num) + den / 2) / den).clamp(1, u128::from(u64::MAX)) as u64
}

/// The cells and size of a part of a picture shown as the program asked.
///
/// `source` is the part to show (already inside the picture), `cols` and `rows` what the program
/// asked for (0 for "from the picture"), `offset` the pixels from the first cell's corner, `cell`
/// the size of a cell in pixels.
pub(super) fn requested(
    source: SourceRect,
    cols: u32,
    rows: u32,
    (offset_x, offset_y): (u32, u32),
    (cell_w, cell_h): (u32, u32),
) -> Geometry {
    let (cell_w, cell_h) = (u64::from(cell_w.max(1)), u64::from(cell_h.max(1)));
    let (sw, sh) = (u64::from(source.width.max(1)), u64::from(source.height.max(1)));
    let (ox, oy) = (u64::from(offset_x), u64::from(offset_y));
    // Counts from the stream are clamped before they are multiplied.
    let (cols, rows) = (u64::from(cols).min(COLS_MAX), u64::from(rows).min(1 << 16));
    let (cols, rows, extent) = match (cols, rows) {
        (0, 0) => (
            ceil_div(ox + sw, cell_w),
            ceil_div(oy + sh, cell_h),
            Extent::Pixels { width: sw as u32, height: sh as u32 },
        ),
        (c, 0) => {
            let (width, height) = (c * cell_w, scaled(c * cell_w, sh, sw));
            (c, ceil_div(oy + height, cell_h), pixels(width, height))
        }
        (0, r) => {
            let (width, height) = (scaled(r * cell_h, sw, sh), r * cell_h);
            (ceil_div(ox + width, cell_w), r, pixels(width, height))
        }
        (c, r) => (c, r, Extent::Fill),
    };
    Geometry {
        cols: cols.clamp(1, COLS_MAX) as usize,
        rows: rows.max(1) as usize,
        source,
        extent,
        offset_x,
        offset_y,
    }
    .limited()
}

fn pixels(width: u64, height: u64) -> Extent {
    Extent::Pixels {
        width: width.min(u64::from(u32::MAX)) as u32,
        height: height.min(u64::from(u32::MAX)) as u32,
    }
}

impl Geometry {
    /// Cut to [`ROWS_MAX`] rows.
    pub fn limited(self) -> Geometry {
        self.cut_rows(ROWS_MAX)
    }

    /// Keep the first `rows` rows (at least one) with the part of the picture they show.
    pub fn cut_rows(mut self, rows: usize) -> Geometry {
        let rows = rows.max(1);
        if self.rows <= rows {
            return self;
        }
        let (keep, of) = (rows as u64, self.rows as u64);
        self.source.height = scaled(u64::from(self.source.height), keep, of).min(u64::from(u32::MAX)) as u32;
        if let Extent::Pixels { width, height } = self.extent {
            let height = scaled(u64::from(height), keep, of).min(u64::from(u32::MAX)) as u32;
            self.extent = Extent::Pixels { width, height };
        }
        self.rows = rows;
        self
    }
}

/// A picture at its own size in pixels, as a sixel is: the cells it touches.
pub(super) fn native(width: u32, height: u32, cell: (u32, u32)) -> Geometry {
    let source = SourceRect { x: 0, y: 0, width, height };
    requested(source, 0, 0, (0, 0), cell)
}

#[cfg(test)]
mod tests {
    use super::{Geometry, ROWS_MAX, native, requested};
    use crate::types::{Extent, SourceRect};

    const CELL: (u32, u32) = (8, 16);

    fn whole(w: u32, h: u32) -> SourceRect {
        SourceRect { x: 0, y: 0, width: w, height: h }
    }

    #[test]
    fn a_picture_without_a_size_covers_the_cells_it_touches() {
        let g = requested(whole(100, 40), 0, 0, (0, 0), CELL);
        assert_eq!((g.cols, g.rows), (13, 3));
        assert_eq!(g.extent, Extent::Pixels { width: 100, height: 40 });
        // The offset inside the first cell pushes it over the edge of a cell.
        let g = requested(whole(16, 16), 0, 0, (4, 8), CELL);
        assert_eq!((g.cols, g.rows), (3, 2));
        assert_eq!((g.offset_x, g.offset_y), (4, 8));
        assert_eq!(native(8, 16, CELL).cols, 1);
        assert_eq!((native(9, 17, CELL).cols, native(9, 17, CELL).rows), (2, 2));
    }

    #[test]
    fn one_dimension_keeps_the_shape_and_both_stretch() {
        // 80x40 is 2:1; 10 columns are 80 pixels wide, so 40 tall: 3 rows of 16.
        let g = requested(whole(80, 40), 10, 0, (0, 0), CELL);
        assert_eq!((g.cols, g.rows), (10, 3));
        assert_eq!(g.extent, Extent::Pixels { width: 80, height: 40 });
        // 2 rows are 32 pixels tall: 64 wide, 8 columns.
        let g = requested(whole(80, 40), 0, 2, (0, 0), CELL);
        assert_eq!((g.cols, g.rows), (8, 2));
        assert_eq!(g.extent, Extent::Pixels { width: 64, height: 32 });
        let g = requested(whole(80, 40), 5, 7, (0, 0), CELL);
        assert_eq!((g.cols, g.rows, g.extent), (5, 7, Extent::Fill));
    }

    #[test]
    fn a_part_of_the_picture_is_what_is_sized() {
        let part = SourceRect { x: 10, y: 10, width: 30, height: 20 };
        let g = requested(part, 0, 0, (0, 0), CELL);
        assert_eq!((g.cols, g.rows, g.source), (4, 2, part));
    }

    #[test]
    fn rows_are_capped_and_the_picture_is_cut_with_them() {
        let tall = requested(whole(8, 16 * 3000), 0, 0, (0, 0), CELL);
        assert_eq!(tall.rows, ROWS_MAX);
        assert!(tall.source.height < 16 * 3000 && tall.source.height >= 16 * 1000);
        let Extent::Pixels { height, .. } = tall.extent else { panic!("{:?}", tall.extent) };
        assert_eq!(height, tall.source.height);
        let g = requested(whole(80, 160), 0, 0, (0, 0), CELL);
        let cut = g.cut_rows(4);
        assert_eq!((cut.rows, cut.source.height), (4, 64));
        assert_eq!(Geometry::cut_rows(g, 0).rows, 1, "at least a row");
    }

    #[test]
    fn nothing_overflows() {
        let g = requested(whole(u32::MAX, u32::MAX), u32::MAX, 0, (u32::MAX, u32::MAX), CELL);
        assert!(g.cols >= 1 && g.rows >= 1);
        let g = requested(whole(1, 1), 0, u32::MAX, (0, 0), (1, 1));
        assert!(g.rows <= ROWS_MAX);
        let _ = requested(whole(0, 0), 0, 0, (0, 0), (0, 0));
    }
}
