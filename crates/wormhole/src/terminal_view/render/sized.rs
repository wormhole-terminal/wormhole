//! Painting rows that are double-width or half of a double-height line (DECDWL, DECDHL).
//!
//! A row of the cache is built in cell columns whatever its size; only the painter knows that a
//! column of such a row is twice as wide. The rows of normal size never come here: the painter asks
//! `size.is_double()` once per row and keeps its own loops for them.
//!
//! - **Backgrounds, selection, find, underlines, strikethrough, overlines** scale their columns by
//!   two. On a tall line the lines through and under the text are twice as thick and sit at twice
//!   the distance from the line's top, as the glyphs do.
//! - **Glyphs of a tall line** are the ones the cache shaped at the normal size, painted at twice
//!   the size: `paint_glyph` takes a font size, and each cell then holds a glyph twice as wide and
//!   as tall, which is what DECDHL asks for. Each row of the line draws the whole line's text,
//!   cut to its own rectangle by a content mask: the top row the upper half, the row below the
//!   lower half.
//! - **Glyphs of a double-width row** are not stretched: GPUI paints a glyph at a font size (both
//!   dimensions) and has no transform for glyphs, so a glyph twice as wide would be twice as tall,
//!   which the row cannot hold. They keep their size and sit in the middle of their doubled
//!   cells, so the line has the width a double-width line has and the text its normal height.
//! - **Box drawing and block elements** are shapes, not glyphs: they are made for the scaled cell
//!   (`shape_metrics`) and so do stretch.
//! - Pictures are not drawn on these rows (their tiles are a cell big).

use gpui_kit::{Bounds, ContentMask, Hsla, Pixels, Point, StrikethroughStyle, Window, point, px, size};
use wormhole_core::terminal::types::LineSize;

use super::blink;
use super::frame::Geometry;
use super::underline::paint_underlines;
use super::underline::{self, UnderlinePaint, underline_paint};
use super::words::Glyph;
use crate::terminal_view::line_size::{Half, half, height_scale, width_scale};

/// Where a sized row and the line it belongs to are on screen.
#[derive(Clone, Copy, Debug)]
pub(super) struct RowBox {
    /// The grid's left edge.
    pub left: Pixels,
    /// The row's own top and its height.
    pub row_top: Pixels,
    pub row_height: Pixels,
    /// The top of the line the row draws: its own top, or the row above's for the bottom half of
    /// a double-height line.
    pub top: Pixels,
    /// A cell as drawn: twice a normal one.
    pub cell_width: Pixels,
    /// How many times the usual size the line's text and the lines on it are drawn at.
    pub scale_y: f32,
    /// The row's rectangle, which what a double-height line draws is cut to.
    pub clip: Option<ContentMask<Pixels>>,
}

impl RowBox {
    pub fn new(size: LineSize, geometry: &Geometry, row: usize, columns: usize) -> RowBox {
        let Geometry { origin, cell_width, line_height, .. } = *geometry;
        let row_top = origin.y + line_height * row as f32;
        let top = match half(size) {
            Some(Half::Bottom) => row_top - line_height,
            _ => row_top,
        };
        let clip = size.is_double_height().then(|| ContentMask {
            bounds: Bounds::new(
                point(origin.x, row_top),
                gpui_kit::size(cell_width * columns as f32, line_height),
            ),
        });
        RowBox {
            left: origin.x,
            row_top,
            row_height: line_height,
            top,
            cell_width: cell_width * width_scale(size),
            scale_y: height_scale(size),
            clip,
        }
    }

    /// The left edge of column `col`.
    pub fn x(&self, col: usize) -> Pixels {
        self.left + self.cell_width * col as f32
    }

    /// A point `dy` below the line's top, at the left edge of column `col`.
    fn at(&self, col: usize, dy: Pixels) -> Point<Pixels> {
        point(self.x(col), self.top + dy)
    }
}

/// What the shapes of a sized row (box drawing, block elements) are made for: where the painter
/// puts the cell of column 0 of row `row`, how big a cell is, and the font size its line weights
/// follow. The painter finds a cell at `origin + cell * (col, row)`, and a tall line's cell is
/// two rows high, so the origin is moved to where that lands on the line's top.
pub(super) struct ShapeMetrics {
    pub origin: Point<Pixels>,
    pub cell_width: Pixels,
    pub line_height: Pixels,
    pub font_size: f32,
}

pub(super) fn shape_metrics(size: LineSize, geometry: &Geometry, row: usize) -> ShapeMetrics {
    let Geometry { origin, cell_width, line_height, font_size, .. } = *geometry;
    let (sx, sy) = (width_scale(size), height_scale(size));
    // The line's top, and where the painter would put it.
    let line_top = match half(size) {
        Some(Half::Bottom) => origin.y + line_height * (row as f32 - 1.0),
        _ => origin.y + line_height * row as f32,
    };
    let painter_top = origin.y + line_height * sy * row as f32;
    ShapeMetrics {
        origin: point(origin.x, origin.y + (line_top - painter_top)),
        cell_width: cell_width * sx,
        line_height: line_height * sy,
        font_size: font_size * sy,
    }
}

/// A sized row's coloured cells. The edge cells' colour runs on into the padding on the left and
/// at the top and bottom of the grid, as on any row, but not on the right: the cells past the half
/// are not shown, and the row's right half is the terminal's own background.
pub(super) fn backgrounds(
    window: &mut Window,
    runs: &[(usize, usize, Hsla)],
    b: &RowBox,
    reach: gpui_kit::Edges<Pixels>,
    (row, rows_in_view): (usize, usize),
) {
    for (start, end, color) in runs {
        let mut from = point(b.x(*start), b.row_top);
        let mut width = b.cell_width * (end - start) as f32;
        let mut height = b.row_height;
        if *start == 0 {
            from.x -= reach.left;
            width += reach.left;
        }
        if row == 0 {
            from.y -= reach.top;
            height += reach.top;
        }
        if row + 1 >= rows_in_view {
            height += reach.bottom;
        }
        window.paint_quad(gpui_kit::fill(Bounds::new(from, size(width, height)), *color));
    }
}

/// A sized row's text. `baseline_y` is how far below a line's top the baseline sits at the normal
/// size, `font_size` the normal size, `cell_width` a normal cell.
pub(super) fn glyphs(
    window: &mut Window,
    words: &[(usize, Vec<Glyph>)],
    b: &RowBox,
    (baseline_y, font_size, cell_width): (Pixels, Pixels, Pixels),
    blink_faded: bool,
) {
    let font_size = font_size * b.scale_y;
    let tall = b.scale_y > 1.0;
    window.with_content_mask(b.clip, |window| {
        for (col, glyphs) in words {
            let word = b.at(*col, baseline_y * b.scale_y);
            for glyph in glyphs {
                // A tall line's glyphs are drawn twice as big and spaced twice as far; a
                // double-width row's keep their size and are centred in their cells.
                let x = if tall {
                    word.x + glyph.position.x * 2.0
                } else {
                    let spare = if glyph.wide { cell_width } else { cell_width * 0.5 };
                    word.x + glyph.position.x * 2.0 + spare
                };
                let origin = point(x, word.y + glyph.position.y * b.scale_y);
                let _ = if glyph.emoji {
                    window.paint_emoji(origin, glyph.font_id, glyph.id, font_size)
                } else {
                    let color =
                        if blink_faded && glyph.blink { blink::faded(glyph.color) } else { glyph.color };
                    window.paint_glyph(origin, glyph.font_id, glyph.id, font_size, color)
                };
            }
        }
    });
}

/// What is drawn through and under a sized row's text: strikethrough, overlines and underlines.
pub(super) struct Lines<'a> {
    pub struck: &'a [(usize, usize, Hsla)],
    pub overlined: &'a [(usize, usize, Hsla)],
    pub underlines: &'a [super::rows::Underline],
}

pub(super) fn lines(
    window: &mut Window,
    lines: Lines<'_>,
    b: &RowBox,
    (underline_y, strike_y): (Pixels, Pixels),
    scratch: &mut Vec<UnderlinePaint>,
) {
    let weight = b.scale_y;
    window.with_content_mask(b.clip, |window| {
        for (first, end, color) in lines.struck {
            let style = StrikethroughStyle { thickness: px(1.0 * weight), color: Some(*color) };
            let start = b.at(*first, strike_y * weight);
            window.paint_strikethrough(start, b.cell_width * (end - first) as f32, &style);
        }
        for (first, end, color) in lines.overlined {
            let width = b.cell_width * (end - first) as f32;
            let rule = size(width, px(underline::THICKNESS * weight));
            window.paint_quad(gpui_kit::fill(Bounds::new(b.at(*first, Pixels::ZERO), rule), *color));
        }
        for span in lines.underlines {
            scratch.clear();
            underline_paint(
                span.kind,
                b.at(span.first, Pixels::ZERO),
                span.end - span.first,
                b.cell_width,
                underline_y * weight,
                b.row_height * weight,
                weight,
                scratch,
            );
            paint_underlines(scratch, span.color, weight, window);
        }
    });
}

/// A link's underline on a sized row, over `cells` cells from column `first`.
pub(super) fn link_underline(
    window: &mut Window,
    (first, cells): (usize, usize),
    b: &RowBox,
    underline_y: Pixels,
    color: Hsla,
    scratch: &mut Vec<UnderlinePaint>,
) {
    scratch.clear();
    let weight = b.scale_y;
    let kind = super::rows::UnderlineKind::Single;
    let start = b.at(first, Pixels::ZERO);
    underline_paint(
        kind,
        start,
        cells,
        b.cell_width,
        underline_y * weight,
        b.row_height * weight,
        weight,
        scratch,
    );
    window.with_content_mask(b.clip, |window| paint_underlines(scratch, color, weight, window));
}

#[cfg(test)]
mod tests {
    use gpui_kit::px;
    use wormhole_core::terminal::types::LineSize;

    use super::{RowBox, shape_metrics};
    use crate::terminal_view::render::frame::Geometry;

    const GEOMETRY: Geometry = Geometry {
        origin: gpui_kit::Point { x: px(10.0), y: px(40.0) },
        cell_width: px(8.0),
        line_height: px(20.0),
        scale: 2.0,
        font_size: 14.0,
    };

    #[test]
    fn a_double_width_row_has_cells_twice_as_wide_and_the_usual_height() {
        let b = RowBox::new(LineSize::DoubleWidth, &GEOMETRY, 2, 80);
        assert_eq!((b.cell_width, b.top, b.row_top, b.scale_y), (px(16.0), px(80.0), px(80.0), 1.0));
        assert!(b.clip.is_none(), "nothing to cut");
        assert_eq!(b.x(3), px(58.0));
    }

    #[test]
    fn the_two_rows_of_a_tall_line_draw_the_same_line_a_row_apart() {
        let top = RowBox::new(LineSize::DoubleHeightTop, &GEOMETRY, 2, 80);
        let bottom = RowBox::new(LineSize::DoubleHeightBottom, &GEOMETRY, 3, 80);
        assert_eq!((top.top, top.scale_y), (px(80.0), 2.0));
        assert_eq!(bottom.top, px(80.0), "the bottom row draws from the line's top, a row above its own");
        assert_eq!(bottom.row_top, px(100.0));
        let clip = bottom.clip.expect("cut to the row").bounds;
        assert_eq!((clip.top(), clip.bottom()), (px(100.0), px(120.0)));
        assert_eq!(clip.size.width, px(640.0));
    }

    #[test]
    fn shapes_land_on_the_line_whatever_row_the_painter_counts_from() {
        for (size, row, line_top) in [
            (LineSize::DoubleHeightTop, 4, 40.0 + 80.0),
            (LineSize::DoubleHeightBottom, 5, 40.0 + 80.0),
            (LineSize::DoubleWidth, 4, 40.0 + 80.0),
        ] {
            let m = shape_metrics(size, &GEOMETRY, row);
            let painted = f32::from(m.origin.y) + f32::from(m.line_height) * row as f32;
            assert_eq!(painted, line_top, "{size:?}");
        }
        let tall = shape_metrics(LineSize::DoubleHeightTop, &GEOMETRY, 1);
        assert_eq!((tall.cell_width, tall.line_height, tall.font_size), (px(16.0), px(40.0), 28.0));
        assert_eq!(tall.origin.x, px(10.0));
    }
}
