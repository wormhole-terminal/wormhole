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
    /// How many times wider than made the shapes are drawn: 2 on a double-width row, whose box
    /// drawing is the normal cell's shapes stretched, thickness of the vertical lines included, as
    /// its text is. The cell is then a normal one and `origin.x` is the point the stretch is from.
    stretch: f32,
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
        painter.reset(origin, cell_width, line_height, scale, font_size, 1.0);
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
        stretch: f32,
    ) {
        self.origin = origin;
        self.cell_width = cell_width;
        self.line_height = line_height;
        self.scale = scale;
        self.stretch = stretch;
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
                    let path = if self.stretch == 1.0 {
                        let mut builder = PathBuilder::stroke(px(width / self.scale));
                        for (i, [x, y]) in points.into_iter().enumerate() {
                            let at = self.logical(x, y);
                            if i == 0 {
                                builder.move_to(at);
                            } else {
                                builder.line_to(at);
                            }
                        }
                        builder.build()
                    } else {
                        self.stretched_stroke(&points, width)
                    };
                    if let Ok(path) = path {
                        self.strokes.push((path, fg));
                    }
                }
            }
        }
        self.shapes = shapes;
        drawn
    }

    /// A stroke under a stretch: GPUI strokes a path with one width all round, but a stretched
    /// line is thicker across than along, so the stroke is its outline (the polyline's two edges,
    /// butt caps) moved by the stretch and filled.
    fn stretched_stroke(&self, points: &[[f32; 2]], width: f32) -> Result<Path<Pixels>, anyhow::Error> {
        let half = width / 2.0;
        let normal = |a: [f32; 2], b: [f32; 2]| {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = dx.hypot(dy).max(f32::EPSILON);
            [-dy / len, dx / len]
        };
        let n = points.len();
        let mut left = Vec::with_capacity(n);
        let mut right = Vec::with_capacity(n);
        for (i, p) in points.iter().enumerate() {
            // The edges at a bend follow the average of the two segments' normals.
            let before = normal(points[i.saturating_sub(1)], *p);
            let after = normal(*p, points[(i + 1).min(n - 1)]);
            let (nx, ny) = if i == 0 {
                (after[0], after[1])
            } else if i + 1 == n {
                (before[0], before[1])
            } else {
                let (sx, sy) = (before[0] + after[0], before[1] + after[1]);
                let len = sx.hypot(sy).max(f32::EPSILON);
                (sx / len, sy / len)
            };
            left.push([p[0] + nx * half, p[1] + ny * half]);
            right.push([p[0] - nx * half, p[1] - ny * half]);
        }
        let mut builder = PathBuilder::fill();
        for (i, [x, y]) in left.into_iter().chain(right.into_iter().rev()).enumerate() {
            let at = self.logical(x, y);
            if i == 0 {
                builder.move_to(at);
            } else {
                builder.line_to(at);
            }
        }
        builder.close();
        builder.build()
    }

    fn add_block(&mut self, rects: &[[f32; 4]], alpha: f32, col: usize, row: usize, fg: Hsla) {
        let mut color = fg;
        color.a *= alpha;
        let (origin, cell_width, line_height, scale) =
            (self.origin, self.cell_width * self.stretch, self.line_height, self.scale);
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

    /// A point in device pixels as logical ones, stretched from the grid's left edge.
    fn logical(&self, x: f32, y: f32) -> Point<Pixels> {
        let left = f32::from(self.origin.x);
        point(px(left + (x / self.scale - left) * self.stretch), px(y / self.scale))
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
        painter.reset(point(px(0.0), px(0.0)), px(8.0), px(16.0), 1.0, 14.0, 1.0);
        assert!(painter.rects.is_empty());
    }

    /// A painter for a double-width row: shapes made for a normal cell, drawn twice as wide.
    fn stretched() -> GlyphPainter {
        let mut painter = GlyphPainter::default();
        painter.reset(point(px(10.0), px(42.0)), px(8.4), px(18.0), 2.0, 14.0, 2.0);
        painter
    }

    #[test]
    fn a_stretched_line_is_twice_as_thick_across_and_as_thick_along() {
        let (mut normal, mut wide) = (painter(), stretched());
        normal.add('│', 3, 1, RED);
        wide.add('│', 3, 1, RED);
        let (n, w) = (normal.rects[0].0, wide.rects[0].0);
        assert_eq!(w.size.width, n.size.width * 2.0, "the vertical line's thickness stretches");
        assert_eq!((w.top(), w.bottom()), (n.top(), n.bottom()), "its height does not");
        // Its column is twice as far from the grid's left edge.
        let from_left = |b: Bounds<Pixels>| f32::from(b.left() - px(10.0));
        assert!((from_left(w) - from_left(n) * 2.0).abs() < 0.01, "{w:?} {n:?}");
        normal.add('─', 0, 0, RED);
        wide.add('─', 0, 0, RED);
        let (n, w) = (normal.rects[1].0, wide.rects[1].0);
        assert_eq!(w.size.height, n.size.height, "a horizontal line is as thick as on a normal row");
    }

    #[test]
    fn stretched_cells_join_as_normal_ones_do() {
        let mut wide = stretched();
        for (col, c) in "┌──┬──┐".chars().enumerate() {
            assert!(wide.add(c, col, 0, RED));
        }
        // The rule runs unbroken through the junctions, from the first cell's middle to the last's.
        let mut rules: Vec<Bounds<Pixels>> =
            wide.rects.iter().map(|(rect, _)| *rect).filter(|r| r.size.height < px(2.0)).collect();
        rules.sort_by(|a, b| f32::from(a.left()).total_cmp(&f32::from(b.left())));
        for pair in rules.windows(2) {
            assert!(pair[1].left() <= pair[0].right(), "a gap in the rule: {rules:?}");
        }
        let cell = px(8.4) * 2.0;
        let end = rules.iter().map(|r| r.right()).fold(px(0.0), Pixels::max);
        assert!((end - (px(10.0) + cell * 6.5)).abs() < px(2.5), "{rules:?}");
        let rects: Vec<Bounds<Pixels>> = wide.rects.iter().map(|(rect, _)| *rect).collect();
        for rect in &rects {
            assert!(rects.iter().filter(|other| *other != rect).any(|other| touches(rect, other)));
        }
    }

    #[test]
    fn stretched_blocks_and_curves_fill_the_doubled_cell() {
        let mut wide = stretched();
        assert!(wide.add('█', 2, 0, RED));
        let (block, _) = wide.rects[0];
        assert!((block.size.width - px(8.4) * 2.0).abs() < px(0.6), "{block:?}");
        assert!((block.left() - (px(10.0) + px(8.4) * 4.0)).abs() < px(0.6), "{block:?}");
        // A rounded corner is its two stubs and an arc, the arc filled as a ribbon.
        wide.add('╭', 0, 1, RED);
        assert_eq!((wide.rects.len(), wide.strokes.len()), (3, 1));
        wide.add('╱', 1, 1, RED);
        assert_eq!(wide.strokes.len(), 2, "a diagonal too");
    }
}
