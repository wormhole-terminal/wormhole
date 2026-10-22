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
//! - **Glyphs of a double-width row** are stretched: GPUI paints a glyph at a font size (both
//!   dimensions) and has no transform for glyphs, so each glyph is its outline from the font
//!   with the stretch baked in, painted as an SVG mask (`outline`). A glyph with no outline to
//!   use (a bitmap emoji, a font that couldn't be pinned down, or any glyph off macOS) keeps its
//!   size and sits in the middle of its doubled cells, so the line still has the width a
//!   double-width line has and the text its normal height.
//! - **Box drawing and block elements** are shapes, not glyphs (`shape_metrics`): a tall line's
//!   are made for its doubled cell; a double-width row's are made for a normal cell and stretched
//!   to twice the width, thickness of the vertical lines included, as its glyphs are.
//! - Pictures are not drawn on these rows (their tiles are a cell big).

use gpui_kit::{
    App, Bounds, ContentMask, Hsla, Pixels, Point, StrikethroughStyle, TransformationMatrix, Window, point,
    px, size,
};
use wormhole_term::types::LineSize;

use super::blink;
use super::frame::Geometry;
use super::outline::{Outlines, Stretch};
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
    /// How many columns the row shows, and where the grid's right edge is: a colour that runs to
    /// the last shown column runs on to the edge, as on a normal row.
    pub shown: usize,
    pub right: Pixels,
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
            shown: size.visible_columns(columns),
            right: origin.x + cell_width * columns as f32,
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
    /// How many times wider the shapes are drawn than they are made.
    pub stretch: f32,
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
    // A double-width row's shapes are made for a normal cell and stretched, so that their lines
    // are as much thicker across as the text's stems are; a tall line's are made for its doubled
    // cell, whose lines are thicker both ways with the font size.
    let stretch = if size.is_double_height() { 1.0 } else { sx };
    ShapeMetrics {
        origin: point(origin.x, origin.y + (line_top - painter_top)),
        cell_width: cell_width * sx / stretch,
        line_height: line_height * sy,
        font_size: font_size * sy,
        stretch,
    }
}

/// Where a sized row's colour run over columns `start..end` is painted. The edge cells' colour
/// runs on into the padding on the left and at the top and bottom of the grid, as on any row, and
/// on the right when the run reaches the last column the row shows: those cells are drawn twice as
/// wide, and a selection or background that covers the rest of the row must end where a normal
/// row's does, not short of it (with an odd number of columns by a whole cell).
pub(super) fn background_bounds(
    b: &RowBox,
    (start, end): (usize, usize),
    reach: gpui_kit::Edges<Pixels>,
    (row, rows_in_view): (usize, usize),
) -> Bounds<Pixels> {
    let mut from = point(b.x(start), b.row_top);
    let mut width = b.cell_width * (end - start) as f32;
    let mut height = b.row_height;
    if start == 0 {
        from.x -= reach.left;
        width += reach.left;
    }
    if end >= b.shown {
        width = b.right + reach.right - from.x;
    }
    if row == 0 {
        from.y -= reach.top;
        height += reach.top;
    }
    if row + 1 >= rows_in_view {
        height += reach.bottom;
    }
    Bounds::new(from, size(width, height))
}

/// A sized row's coloured cells.
pub(super) fn backgrounds(
    window: &mut Window,
    runs: &[(usize, usize, Hsla)],
    b: &RowBox,
    reach: gpui_kit::Edges<Pixels>,
    place: (usize, usize),
) {
    for (start, end, color) in runs {
        window.paint_quad(gpui_kit::fill(background_bounds(b, (*start, *end), reach, place), *color));
    }
}

/// What a sized row's text is drawn from: how far below a line's top the baseline sits at the
/// normal size, the normal font size, a normal cell, and the window's scale factor.
#[derive(Clone, Copy, Debug)]
pub(super) struct TextMetrics {
    pub baseline_y: Pixels,
    pub font_size: Pixels,
    pub cell_width: Pixels,
    pub scale: f32,
}

/// A sized row's text.
pub(super) fn glyphs(
    window: &mut Window,
    words: &[(usize, Vec<Glyph>)],
    b: &RowBox,
    metrics: TextMetrics,
    blink_faded: bool,
    outlines: &mut Outlines,
    cx: &App,
) {
    let TextMetrics { baseline_y, font_size, cell_width, scale } = metrics;
    let font_size = font_size * b.scale_y;
    let tall = b.scale_y > 1.0;
    window.with_content_mask(b.clip, |window| {
        for (col, glyphs) in words {
            let word = b.at(*col, baseline_y * b.scale_y);
            for glyph in glyphs {
                let color = if blink_faded && glyph.blink { blink::faded(glyph.color) } else { glyph.color };
                // A tall line's glyphs are drawn twice as big, which is the stretch it asks for,
                // and spaced twice as far.
                if tall {
                    let origin =
                        point(word.x + glyph.position.x * 2.0, word.y + glyph.position.y * b.scale_y);
                    let _ = if glyph.emoji {
                        window.paint_emoji(origin, glyph.font_id, glyph.id, font_size)
                    } else {
                        window.paint_glyph(origin, glyph.font_id, glyph.id, font_size, color)
                    };
                    continue;
                }
                // A double-width row's glyph is its outline stretched to twice the width, so it
                // fills its doubled cells; the first cell's left edge is where its unstretched
                // glyph's was, twice as far from the word's start.
                let origin = point(word.x + glyph.position.x * 2.0, word.y + glyph.position.y);
                if !glyph.emoji {
                    let id = (glyph.font_id, glyph.id);
                    if let Some((sprite, bounds)) =
                        outlines.stretched(id, f32::from(font_size), scale, Stretch::WIDE, origin)
                    {
                        let svg = Some(sprite.svg());
                        let _ = window.paint_svg(
                            bounds,
                            sprite.path().clone(),
                            svg,
                            TransformationMatrix::unit(),
                            color,
                            cx,
                        );
                        continue;
                    }
                }
                // No outline to stretch (a bitmap emoji, a font that couldn't be pinned down): the
                // glyph keeps its size and sits in the middle of its doubled cells.
                let spare = if glyph.wide { cell_width } else { cell_width * 0.5 };
                let origin = point(origin.x + spare, origin.y);
                let _ = if glyph.emoji {
                    window.paint_emoji(origin, glyph.font_id, glyph.id, font_size)
                } else {
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
    use wormhole_term::types::LineSize;

    use super::{RowBox, background_bounds, shape_metrics};
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
        assert_eq!((tall.origin.x, tall.stretch), (px(10.0), 1.0));
        let wide = shape_metrics(LineSize::DoubleWidth, &GEOMETRY, 1);
        assert_eq!((wide.cell_width, wide.line_height, wide.stretch), (px(8.0), px(20.0), 2.0));
    }

    #[test]
    fn a_colour_to_the_last_shown_column_runs_to_the_grids_edge() {
        let reach = gpui_kit::Edges { top: px(0.0), right: px(6.0), bottom: px(0.0), left: px(4.0) };
        // 81 columns: a double-width row shows 40 of them, 640 px, and a normal one's edge is 648 px
        // from the grid's left.
        let b = RowBox::new(LineSize::DoubleWidth, &GEOMETRY, 2, 81);
        let edge = px(10.0) + px(8.0) * 81.0 + px(6.0);
        let tail = background_bounds(&b, (10, 40), reach, (2, 10));
        assert_eq!(tail.right(), edge, "to the edge, padding included");
        assert_eq!(tail.left(), px(10.0 + 16.0 * 10.0));
        let first = background_bounds(&b, (0, 3), reach, (2, 10));
        assert_eq!((first.left(), first.right()), (px(6.0), px(58.0)), "the left padding is covered too");
        let middle = background_bounds(&b, (3, 8), reach, (2, 10));
        assert_eq!((middle.left(), middle.right()), (px(58.0), px(138.0)), "nothing else moves");
    }
}
