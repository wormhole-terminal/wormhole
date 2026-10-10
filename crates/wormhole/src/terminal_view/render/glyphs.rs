//! Characters the terminal paints as shapes instead of font glyphs.
//!
//! Fonts draw block elements and box-drawing lines only ascent+descent tall,
//! which is shorter than a row once the line height is above 1.0; the gaps show
//! as stripes in block art and broken vertical borders. Drawing them as exact
//! cell-sized shapes (geometry lives in `wormhole_core`) makes them join.

use gpui_kit::*;
use wormhole_core::terminal::glyphs::block_elements::block_element;
use wormhole_core::terminal::glyphs::box_drawing::{BoxShape, CellBox, box_shapes_into};

/// Collects the shapes of one frame. Kept between frames so its buffers are reused.
#[derive(Default)]
pub(super) struct GlyphPainter {
    origin: Point<Pixels>,
    cell_width: Pixels,
    line_height: Pixels,
    scale: f32,
    /// Thickness of a light line in device pixels, following the font size.
    light: f32,
    /// Scratch for one cell's box-drawing shapes.
    shapes: Vec<BoxShape>,
    pub rects: Vec<(Bounds<Pixels>, Hsla)>,
    pub strokes: Vec<(Path<Pixels>, Hsla)>,
}

impl GlyphPainter {
    #[cfg(test)]
    pub fn new(
        origin: Point<Pixels>,
        cell_width: Pixels,
        line_height: Pixels,
        scale: f32,
        font_size: f32,
    ) -> Self {
        let mut painter = Self::default();
        painter.reset(origin, cell_width, line_height, scale, font_size);
        painter
    }

    /// Start a frame with these metrics; keeps the buffers' memory.
    pub fn reset(
        &mut self,
        origin: Point<Pixels>,
        cell_width: Pixels,
        line_height: Pixels,
        scale: f32,
        font_size: f32,
    ) {
        self.origin = origin;
        self.cell_width = cell_width;
        self.line_height = line_height;
        self.scale = scale;
        self.light = (font_size * scale / 14.0).round().max(1.0);
        self.rects.clear();
        self.strokes.clear();
    }

    /// Queues the shapes for `c` in the cell at `col`, `row`. Returns whether
    /// it did, so the caller leaves the cell blank in the text layer.
    pub fn add(&mut self, c: char, col: usize, row: usize, fg: Hsla) -> bool {
        // Cheap reject: this runs for every cell of every frame.
        if !('\u{2500}'..='\u{259F}').contains(&c) {
            return false;
        }
        if let Some(element) = block_element(c) {
            self.add_block(element.rects(), element.alpha, col, row, fg);
            return true;
        }
        let device = |v: Pixels| f32::from(v) * self.scale;
        let cell = CellBox {
            left: device(self.origin.x + self.cell_width * col as f32),
            top: device(self.origin.y + self.line_height * row as f32),
            right: device(self.origin.x + self.cell_width * (col + 1) as f32),
            bottom: device(self.origin.y + self.line_height * (row + 1) as f32),
        };
        let mut shapes = std::mem::take(&mut self.shapes);
        shapes.clear();
        let drawn = box_shapes_into(c, cell, self.light, &mut shapes);
        for shape in shapes.drain(..) {
            match shape {
                BoxShape::Rect([left, top, right, bottom]) => {
                    let rect = Bounds::from_corners(self.logical(left, top), self.logical(right, bottom));
                    self.push_rect(rect, fg);
                }
                BoxShape::Stroke { points, width } => {
                    let mut builder = PathBuilder::stroke(px(width / self.scale));
                    for (i, [x, y]) in points.into_iter().enumerate() {
                        let at = self.logical(x, y);
                        if i == 0 {
                            builder.move_to(at);
                        } else {
                            builder.line_to(at);
                        }
                    }
                    if let Ok(path) = builder.build() {
                        self.strokes.push((path, fg));
                    }
                }
            }
        }
        self.shapes = shapes;
        drawn
    }

    fn add_block(&mut self, rects: &[[f32; 4]], alpha: f32, col: usize, row: usize, fg: Hsla) {
        let mut color = fg;
        color.a *= alpha;
        let (origin, cell_width, line_height, scale) =
            (self.origin, self.cell_width, self.line_height, self.scale);
        // Snap to device pixels so neighbouring blocks meet without seams.
        let snap = |v: Pixels| px((f32::from(v) * scale).round() / scale);
        for [left, top, right, bottom] in rects {
            let x = |frac: f32| snap(origin.x + cell_width * (col as f32 + frac));
            let y = |frac: f32| snap(origin.y + line_height * (row as f32 + frac));
            let rect = Bounds::from_corners(point(x(*left), y(*top)), point(x(*right), y(*bottom)));
            self.push_rect(rect, color);
        }
    }

    /// Queue a rect, extending the previous one instead when they share color
    /// and rows and touch: a 200-column `─` rule is one quad, not 400 (each
    /// cell's line is two half-arms that overlap in the middle).
    fn push_rect(&mut self, rect: Bounds<Pixels>, color: Hsla) {
        if let Some((last, last_color)) = self.rects.last_mut()
            && *last_color == color
            && last.top() == rect.top()
            && last.bottom() == rect.bottom()
            && rect.left() <= last.right()
            && rect.right() >= last.left()
        {
            let (left, right) = (last.left().min(rect.left()), last.right().max(rect.right()));
            *last = Bounds::from_corners(point(left, last.top()), point(right, last.bottom()));
            return;
        }
        self.rects.push((rect, color));
    }

    fn logical(&self, x: f32, y: f32) -> Point<Pixels> {
        point(px(x / self.scale), px(y / self.scale))
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, Hsla, Pixels, point, px};

    use super::GlyphPainter;

    const RED: Hsla = Hsla { h: 0.0, s: 1.0, l: 0.5, a: 1.0 };
    const BLUE: Hsla = Hsla { h: 0.66, s: 1.0, l: 0.5, a: 1.0 };

    fn painter() -> GlyphPainter {
        GlyphPainter::new(point(px(10.0), px(42.0)), px(8.4), px(18.0), 2.0, 14.0)
    }

    fn touches(a: &Bounds<Pixels>, b: &Bounds<Pixels>) -> bool {
        a.left() <= b.right() && b.left() <= a.right() && a.top() <= b.bottom() && b.top() <= a.bottom()
    }

    #[test]
    fn a_rule_is_one_quad() {
        let mut painter = painter();
        for col in 0..200 {
            assert!(painter.add('─', col, 3, RED));
        }
        assert_eq!(painter.rects.len(), 1);
        let (rect, _) = painter.rects[0];
        assert!((rect.left() - px(10.0)).abs() < px(0.5));
        assert!((rect.right() - (px(10.0) + px(8.4) * 200.0)).abs() < px(0.5));
    }

    #[test]
    fn colors_and_rows_are_not_merged() {
        let mut painter = painter();
        painter.add('─', 0, 0, RED);
        painter.add('─', 1, 0, BLUE);
        painter.add('─', 2, 1, BLUE);
        assert_eq!(painter.rects.len(), 3);
    }

    #[test]
    fn junctions_stay_connected() {
        let mut painter = painter();
        painter.add('─', 0, 0, RED);
        painter.add('┼', 1, 0, RED);
        painter.add('─', 2, 0, RED);
        let rects: Vec<Bounds<Pixels>> = painter.rects.iter().map(|(rect, _)| *rect).collect();
        assert!(rects.len() >= 2, "the vertical arms stay separate: {rects:?}");
        for rect in &rects {
            assert!(rects.iter().filter(|other| *other != rect).any(|other| touches(rect, other)));
        }
    }

    #[test]
    fn blocks_and_text_are_told_apart() {
        let mut painter = painter();
        assert!(painter.add('█', 0, 0, RED));
        assert!(painter.add('█', 1, 0, RED));
        assert_eq!(painter.rects.len(), 1, "a run of full blocks is one quad");
        assert!(!painter.add('a', 2, 0, RED));
        painter.reset(point(px(0.0), px(0.0)), px(8.0), px(16.0), 1.0, 14.0);
        assert!(painter.rects.is_empty());
    }
}
