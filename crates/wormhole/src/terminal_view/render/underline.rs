//! Underlines, painted by the terminal in every style SGR 4 asks for (see
//! `rows::Underline` for why not by the text shaper).
//!
//! Straight and curly ones go through GPUI's own underline primitive (it draws
//! the wave); double, dotted and dashed ones are quads. Dots and dashes are laid
//! out per cell, so they line up from row to row like the characters do.

use gpui_kit::{Bounds, Hsla, Pixels, Point, UnderlineStyle, Window, fill, point, px, size};

use super::rows::UnderlineKind;

/// The line's thickness; one point, as the shaper drew it before.
pub(super) const THICKNESS: f32 = 1.0;

/// One thing to paint for an underline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum UnderlinePaint {
    /// GPUI's underline from `origin`, `width` long.
    Line {
        origin: Point<Pixels>,
        width: Pixels,
        wavy: bool,
    },
    Quad(Bounds<Pixels>),
}

/// Where an underline of `kind` over `cells` cells is painted. `start` is its
/// left end at the top of the row; `y` is how far below the row's top the
/// font's underline sits; `line_height` bounds the second line of a double one. `weight` is how
/// many times the usual thickness: 2 on a double-height line, whose lines are as thick as its
/// text is tall.
#[allow(clippy::too_many_arguments)]
pub(super) fn underline_paint(
    kind: UnderlineKind,
    start: Point<Pixels>,
    cells: usize,
    cell_width: Pixels,
    y: Pixels,
    line_height: Pixels,
    weight: f32,
    out: &mut Vec<UnderlinePaint>,
) {
    let t = px(THICKNESS * weight);
    let width = cell_width * cells as f32;
    let at = |dy: Pixels| point(start.x, start.y + dy);
    match kind {
        UnderlineKind::Single => out.push(UnderlinePaint::Line { origin: at(y), width, wavy: false }),
        UnderlineKind::Curly => out.push(UnderlinePaint::Line { origin: at(y), width, wavy: true }),
        UnderlineKind::Double => {
            // The second line goes below the first, unless that leaves the row.
            let lower = (y + t * 2.0).min(line_height - t);
            let upper = lower - t * 2.0;
            for dy in [upper, lower] {
                out.push(UnderlinePaint::Quad(Bounds::new(at(dy), size(width, t))));
            }
        }
        UnderlineKind::Dotted => {
            // Square dots with gaps as wide; whole dots per cell.
            let per_cell = (f32::from(cell_width) / (THICKNESS * weight * 2.0)).floor().max(1.0) as usize;
            let step = cell_width / per_cell as f32;
            for i in 0..cells * per_cell {
                let x = start.x + step * i as f32 + (step - t) / 2.0;
                out.push(UnderlinePaint::Quad(Bounds::new(point(x, start.y + y), size(t, t))));
            }
        }
        UnderlineKind::Dashed => {
            // One dash per cell, centred, so a dashed word reads as dashes rather than a line.
            let dash = cell_width * 0.6;
            for i in 0..cells {
                let x = start.x + cell_width * i as f32 + (cell_width - dash) / 2.0;
                out.push(UnderlinePaint::Quad(Bounds::new(point(x, start.y + y), size(dash, t))));
            }
        }
    }
}

/// `weight` is how many times the usual thickness the lines are drawn (2 on a double-height line).
pub(super) fn paint_underlines(paints: &[UnderlinePaint], color: Hsla, weight: f32, window: &mut Window) {
    for paint in paints {
        match *paint {
            UnderlinePaint::Line { origin, width, wavy } => {
                let thickness = px(THICKNESS * weight);
                let style = UnderlineStyle { thickness, color: Some(color), wavy };
                window.paint_underline(origin, width, &style);
            }
            UnderlinePaint::Quad(bounds) => window.paint_quad(fill(bounds, color)),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{point, px};

    use super::{UnderlinePaint, underline_paint};
    use crate::terminal_view::render::rows::UnderlineKind;

    fn paint(kind: UnderlineKind, cells: usize, y: f32) -> Vec<UnderlinePaint> {
        let mut out = Vec::new();
        underline_paint(kind, point(px(10.0), px(100.0)), cells, px(8.0), px(y), px(18.0), 1.0, &mut out);
        out
    }

    #[test]
    fn straight_and_curly_are_one_line_across_the_cells() {
        let single = paint(UnderlineKind::Single, 3, 15.0);
        assert_eq!(
            single,
            [UnderlinePaint::Line { origin: point(px(10.0), px(115.0)), width: px(24.0), wavy: false }]
        );
        assert!(matches!(
            paint(UnderlineKind::Curly, 3, 15.0)[..],
            [UnderlinePaint::Line { wavy: true, .. }]
        ));
    }

    #[test]
    fn double_stays_inside_the_row() {
        let quads = |y| -> Vec<f32> {
            paint(UnderlineKind::Double, 2, y)
                .iter()
                .map(|p| match p {
                    UnderlinePaint::Quad(b) => f32::from(b.top()) - 100.0,
                    UnderlinePaint::Line { .. } => panic!("double is quads"),
                })
                .collect()
        };
        assert_eq!(quads(14.0), [14.0, 16.0]);
        assert_eq!(quads(16.5), [15.0, 17.0], "moved up so the lower line ends at the row's bottom");
    }

    #[test]
    fn a_heavier_line_is_thicker() {
        let mut out = Vec::new();
        underline_paint(
            UnderlineKind::Double,
            point(px(0.0), px(0.0)),
            1,
            px(16.0),
            px(30.0),
            px(36.0),
            2.0,
            &mut out,
        );
        let tops: Vec<(f32, f32)> = out
            .iter()
            .map(|p| match p {
                UnderlinePaint::Quad(b) => (f32::from(b.top()), f32::from(b.size.height)),
                UnderlinePaint::Line { .. } => panic!("double is quads"),
            })
            .collect();
        assert_eq!(tops, [(30.0, 2.0), (34.0, 2.0)], "twice as thick, twice as far apart");
    }

    #[test]
    fn dots_and_dashes_repeat_per_cell() {
        assert_eq!(paint(UnderlineKind::Dotted, 2, 15.0).len(), 8, "4 dots in an 8 pt cell");
        let dashes = paint(UnderlineKind::Dashed, 3, 15.0);
        assert_eq!(dashes.len(), 3);
        let UnderlinePaint::Quad(second) = dashes[1] else { panic!("a dash is a quad") };
        assert!(second.left() > px(18.0) && second.right() < px(26.0), "inside the second cell");
    }
}
