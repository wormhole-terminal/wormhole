//! The terminal's cell (`wormhole_core::view::layout::cell`): its size and where text sits in it, from the font's
//! measurements, and how many cells a pane holds. The GPUI app sizes its grid by the same rule; Swift measures the font
//! with CoreText (as GPUI does) and asks here, so the same font, size and window give the same grid in both.

use wormhole_core::view::layout::cell::Cell;

/// One cell, in points from its top.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct CellRecord {
    pub width: f64,
    pub height: f64,
    pub baseline: f64,
    pub underline: f64,
    pub strike: f64,
}

/// The whole cells a pane holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct GridFitRecord {
    pub cols: u32,
    pub rows: u32,
}

fn core(cell: CellRecord) -> Cell {
    Cell {
        width: cell.width as f32,
        height: cell.height as f32,
        baseline: cell.baseline as f32,
        underline: cell.underline as f32,
        strike: cell.strike as f32,
    }
}

/// The cell of a font whose `m` advances `advance` and whose ascent and descent (positive) are these, at `font_size`
/// with lines `line_height` times the size.
#[uniffi::export]
pub fn terminal_cell(
    font_size: f64,
    line_height: f64,
    advance: f64,
    ascent: f64,
    descent: f64,
) -> CellRecord {
    let cell = Cell::new(font_size as f32, line_height as f32, advance as f32, ascent as f32, descent as f32);
    CellRecord {
        width: f64::from(cell.width),
        height: f64::from(cell.height),
        baseline: f64::from(cell.baseline),
        underline: f64::from(cell.underline),
        strike: f64::from(cell.strike),
    }
}

/// The whole cells an area of `width` x `height` points holds; `slack` short of a cell still counts.
#[uniffi::export]
pub fn terminal_grid_fit(cell: CellRecord, width: f64, height: f64, slack: f64) -> GridFitRecord {
    let (cols, rows) = core(cell).fit(width as f32, height as f32, slack as f32);
    GridFitRecord {
        cols: u32::try_from(cols).unwrap_or(u32::MAX),
        rows: u32::try_from(rows).unwrap_or(u32::MAX),
    }
}

/// One shape of a box-drawing or block-element cell, in device pixels from the cell's left whole pixel and its top: a
/// rectangle (`rect` = left, top, right, bottom) or a stroked polyline with butt caps (`points` = x, y pairs, `width`).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CellShapeRecord {
    pub rect: Vec<f32>,
    pub points: Vec<f32>,
    pub width: f32,
}

/// What the GPUI app paints for a character it draws as shapes (`terminal_view/render/glyphs.rs`), and how opaque.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CellShapesRecord {
    pub shapes: Vec<CellShapeRecord>,
    pub alpha: f32,
}

/// The shapes of `scalar` in a cell `cell_width` x `height` device pixels whose left edge is `phase` (0..1) past a whole
/// pixel, at `font_size` points and `scale`; `None` for a character drawn from the font. Block elements are snapped to
/// device pixels on the fractional cell and box drawing is made on the cell rounded to whole pixels, as GPUI's painter does
/// (core `wormhole_term::glyphs`).
#[uniffi::export]
pub fn cell_shapes(
    scalar: u32,
    cell_width: f32,
    height: f32,
    phase: f32,
    font_size: f32,
    scale: f32,
) -> Option<CellShapesRecord> {
    use wormhole_term::glyphs::block_elements::block_element;
    use wormhole_term::glyphs::box_drawing::{BoxShape, CellBox, box_shapes_into};

    let c = char::from_u32(scalar).filter(|c| ('\u{2500}'..='\u{259F}').contains(c))?;
    let left = phase.round();
    if let Some(element) = block_element(c) {
        let x = |frac: f32| (phase + cell_width * frac).round() - left;
        let y = |frac: f32| (height * frac).round();
        let shapes = element
            .rects()
            .iter()
            .map(|[l, t, r, b]| CellShapeRecord {
                rect: vec![x(*l), y(*t), x(*r), y(*b)],
                points: Vec::new(),
                width: 0.0,
            })
            .collect();
        return Some(CellShapesRecord { shapes, alpha: element.alpha });
    }
    let light = (font_size * scale / 14.0).round().max(1.0);
    let cell = CellBox { left: phase, top: 0.0, right: phase + cell_width, bottom: height };
    let mut out = Vec::new();
    if !box_shapes_into(c, cell, light, &mut out) {
        return None;
    }
    let shapes = out
        .into_iter()
        .map(|shape| match shape {
            BoxShape::Rect([l, t, r, b]) => {
                CellShapeRecord { rect: vec![l - left, t, r - left, b], points: Vec::new(), width: 0.0 }
            }
            BoxShape::Stroke { points, width } => CellShapeRecord {
                rect: Vec::new(),
                points: points.iter().flat_map(|[x, y]| [x - left, *y]).collect(),
                width,
            },
        })
        .collect();
    Some(CellShapesRecord { shapes, alpha: 1.0 })
}
