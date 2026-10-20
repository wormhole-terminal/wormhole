//! The second half of a frame (the canvas's paint): the layers of a grid, in order, from what
//! prepaint built.
//!
//! The order is the design: the faint rows above the viewport, the command blocks' tint, then
//! the cells' backgrounds (one layer), the pictures under the text, box drawing and block
//! elements (one layer), the cursor, the text (one layer), the pictures over it, the lines through
//! and under the text (one layer), links, the cursor's character and find's ticks. Every per-cell
//! paint sits in a `paint_layer`: see `.claude/rules/terminal-perf.md` for why.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use gpui_kit::{
    App, BorderStyle, Bounds, Corners, Hsla, Pixels, Point, StrikethroughStyle, TextAlign, WeakEntity,
    Window, fill, outline, point, px, size,
};

use super::frame::FrameCache;
use super::outline::Outlines;
use super::overlays::{drag_outside_listener, paint_blocks, paint_ticks};
use super::prepare::Frame;
use super::underline::{self, paint_underlines, underline_paint};
use super::{RenderState, blink, blur, rows, sized};
use crate::settings::UiColors;
use crate::terminal_view::TerminalView;

/// What a frame's paint reads from the view, copied out of `render` so the canvas's closure owns
/// it.
pub(super) struct Paint {
    pub state: Rc<RefCell<RenderState>>,
    pub hovered_block: Option<u64>,
    pub ui: UiColors,
    pub focused: bool,
    /// The rows (and columns) of the link under the pointer, and the colour its underline takes.
    pub link_rows: Vec<(usize, Range<usize>)>,
    pub link_color: Hsla,
    pub find_tick: Hsla,
    pub selecting: bool,
    pub view: WeakEntity<TerminalView>,
}

type Cache = FrameCache<super::shaping::ShapeKey, Vec<super::words::Glyph>, gpui_kit::ShapedLine>;

impl Paint {
    pub fn run(&self, bounds: Bounds<Pixels>, frame: &Frame, window: &mut Window, cx: &mut App) {
        // Paint borrows the state mutably, for the stretched glyphs' cache to fill as it goes;
        // nothing else touches it while a frame is painted.
        let mut state = self.state.borrow_mut();
        let RenderState { frame: cache, outlines, .. } = &mut *state;
        let (origin, line_height) = (frame.geometry.origin, frame.geometry.line_height);
        for (k, line) in cache.blurred() {
            let row_origin = point(origin.x, origin.y - line_height * k as f32);
            for (dx, dy) in blur::BLUR_TAPS {
                let tap = point(row_origin.x + px(dx), row_origin.y + px(dy));
                let _ = line.paint(tap, line_height, TextAlign::Left, None, window, cx);
            }
        }
        paint_blocks(&cache.snapshot, self.hovered_block, self.ui, bounds, origin, line_height, window);
        // Each quad painted outside a layer goes into the scene's bounds tree to find its
        // z-order, and a screen of colored cells is thousands of them: in a full-screen
        // truecolor animation that tree took nine tenths of the main thread, and the window
        // stopped answering. A layer gives everything in it one order. The cells' quads
        // never overlap one another, so nothing about the stacking changes; the images
        // stay between the two layers, where they are now.
        let grid = window.content_mask().bounds;
        backgrounds(cache, frame, grid, window);
        for paint in &frame.images {
            paint_image(paint, "an inline image", window);
        }
        // Block elements and box drawing, drawn as quads and strokes: one layer too.
        boxes(cache, frame, grid, window);
        // Unfocused cursor: hollow block.
        if let Some(cursor) = &frame.cursor {
            if self.focused {
                window.paint_quad(fill(cursor.rect, cursor.color));
            } else {
                window.paint_quad(outline(cursor.rect, cursor.color, BorderStyle::Solid));
            }
        }
        text(cache, frame, grid, outlines, window, cx);
        for paint in &frame.overlay {
            paint_image(paint, "a placed image", window);
        }
        let mut underlines = Vec::new();
        lines(cache, frame, grid, &mut underlines, window);
        self.links(cache, frame, &mut underlines, window);
        if let Some(text) = frame.cursor.as_ref().and_then(|cursor| cursor.text.as_ref()) {
            window.with_content_mask(text.clip, |window| {
                let _ = text.line.paint(text.at, text.line_height, TextAlign::Left, None, window, cx);
            });
        }
        paint_ticks(&frame.ticks, bounds, self.find_tick, window);
        if self.selecting {
            drag_outside_listener(self.view.clone(), window);
        }
    }

    /// The underline of the link under the pointer.
    fn links(
        &self,
        cache: &Cache,
        frame: &Frame,
        underlines: &mut Vec<underline::UnderlinePaint>,
        window: &mut Window,
    ) {
        let (origin, cell_width, line_height) = frame_cells(frame);
        for (row, columns) in &self.link_rows {
            let size = cache.snapshot.rows().get(*row).map_or(Default::default(), |row| row.size());
            if size.is_double() {
                let at = sized::RowBox::new(size, &frame.geometry, *row, frame.cols);
                let cells = (columns.start, columns.len());
                sized::link_underline(window, cells, &at, frame.underline_y, self.link_color, underlines);
                continue;
            }
            underlines.clear();
            let start = cell_at(origin, cell_width, line_height, columns.start, *row);
            let (cells, y) = (columns.len(), frame.underline_y);
            let kind = rows::UnderlineKind::Single;
            underline_paint(kind, start, cells, cell_width, y, line_height, 1.0, underlines);
            paint_underlines(underlines, self.link_color, 1.0, window);
        }
    }
}

/// The grid's origin and cell size.
fn frame_cells(frame: &Frame) -> (Point<Pixels>, Pixels, Pixels) {
    (frame.geometry.origin, frame.geometry.cell_width, frame.geometry.line_height)
}

/// The top left of the cell at `col`, `row`.
fn cell_at(
    origin: Point<Pixels>,
    cell_width: Pixels,
    line_height: Pixels,
    col: usize,
    row: usize,
) -> Point<Pixels> {
    point(origin.x + cell_width * col as f32, origin.y + line_height * row as f32)
}

fn paint_image(paint: &super::images::ImagePaint, what: &str, window: &mut Window) {
    let painted = window.paint_image(
        paint.bounds,
        paint.image_bounds,
        Corners::default(),
        paint.image.clone(),
        0,
        false,
    );
    if let Err(err) = painted {
        tracing::debug!("{what} didn't paint: {err}");
    }
}

/// The cells' colored backgrounds.
fn backgrounds(cache: &Cache, frame: &Frame, grid: Bounds<Pixels>, window: &mut Window) {
    let (origin, cell_width, line_height) = frame_cells(frame);
    let (reach, cols, rows_in_view) = (frame.reach, frame.cols, frame.rows);
    window.paint_layer(grid, |window| {
        for (row, painted) in cache.rows() {
            if painted.size.is_double() {
                let at = sized::RowBox::new(painted.size, &frame.geometry, row, cols);
                sized::backgrounds(window, &painted.backgrounds, &at, reach, (row, rows_in_view));
                continue;
            }
            for (start, end, color) in &painted.backgrounds {
                // The edge cells' colour runs on into the padding (Ghostty's
                // window-padding-color = extend); the default background is the theme's.
                let mut from = cell_at(origin, cell_width, line_height, *start, row);
                let mut width = cell_width * (end - start) as f32;
                let mut height = line_height;
                if *start == 0 {
                    from.x -= reach.left;
                    width += reach.left;
                }
                if *end >= cols {
                    width += reach.right;
                }
                if row == 0 {
                    from.y -= reach.top;
                    height += reach.top;
                }
                if row + 1 >= rows_in_view {
                    height += reach.bottom;
                }
                window.paint_quad(fill(Bounds::new(from, size(width, height)), *color));
            }
        }
    });
}

/// Box drawing and block elements.
fn boxes(cache: &Cache, frame: &Frame, grid: Bounds<Pixels>, window: &mut Window) {
    window.paint_layer(grid, |window| {
        for (row, painted) in cache.rows() {
            // A tall line's shapes are made for the whole line: cut to this row.
            let clip = painted
                .size
                .is_double_height()
                .then(|| sized::RowBox::new(painted.size, &frame.geometry, row, frame.cols).clip);
            window.with_content_mask(clip.flatten(), |window| {
                for (rect, color) in &painted.quads {
                    window.paint_quad(fill(*rect, *color));
                }
                for (path, color) in &painted.strokes {
                    window.paint_path(path.clone(), *color);
                }
            });
        }
    });
}

/// The text in a layer of its own, for the same reason as the backgrounds: a glyph painted on its
/// own goes into the bounds tree, and a full screen of text is thousands of them. The lines through
/// and under it go in the next layer, so they still draw over the glyphs.
fn text(
    cache: &Cache,
    frame: &Frame,
    grid: Bounds<Pixels>,
    outlines: &mut Outlines,
    window: &mut Window,
    cx: &App,
) {
    let (origin, cell_width, line_height) = frame_cells(frame);
    let font_size = px(frame.geometry.font_size);
    window.paint_layer(grid, |window| {
        for (row, painted) in cache.rows() {
            if painted.size.is_double() {
                let at = sized::RowBox::new(painted.size, &frame.geometry, row, frame.cols);
                let metrics = sized::TextMetrics {
                    baseline_y: frame.baseline_y,
                    font_size,
                    cell_width,
                    scale: frame.geometry.scale,
                };
                sized::glyphs(window, &painted.lines, &at, metrics, frame.blink_faded, outlines, cx);
                continue;
            }
            for (col, glyphs) in &painted.lines {
                let word = cell_at(origin, cell_width, line_height, *col, row);
                for glyph in glyphs {
                    let origin =
                        point(word.x + glyph.position.x, word.y + frame.baseline_y + glyph.position.y);
                    let _ = if glyph.emoji {
                        window.paint_emoji(origin, glyph.font_id, glyph.id, font_size)
                    } else {
                        let color = if frame.blink_faded && glyph.blink {
                            blink::faded(glyph.color)
                        } else {
                            glyph.color
                        };
                        window.paint_glyph(origin, glyph.font_id, glyph.id, font_size, color)
                    };
                }
            }
        }
    });
}

/// Strikethrough, overlines and underlines.
fn lines(
    cache: &Cache,
    frame: &Frame,
    grid: Bounds<Pixels>,
    underlines: &mut Vec<underline::UnderlinePaint>,
    window: &mut Window,
) {
    let (origin, cell_width, line_height) = frame_cells(frame);
    window.paint_layer(grid, |window| {
        for (row, painted) in cache.rows() {
            if painted.size.is_double() {
                let at = sized::RowBox::new(painted.size, &frame.geometry, row, frame.cols);
                let lines = sized::Lines {
                    struck: &painted.struck,
                    overlined: &painted.overlined,
                    underlines: &painted.underlines,
                };
                let heights = (frame.underline_y, frame.strike_y);
                sized::lines(window, lines, &at, heights, underlines);
                continue;
            }
            for (first, end, color) in &painted.struck {
                let width = cell_width * (end - first) as f32;
                let start = cell_at(origin, cell_width, line_height, *first, row);
                let style = StrikethroughStyle { thickness: px(1.0), color: Some(*color) };
                window.paint_strikethrough(point(start.x, start.y + frame.strike_y), width, &style);
            }
            for (first, end, color) in &painted.overlined {
                // The same weight as an underline, along the top of the cell.
                let width = cell_width * (end - first) as f32;
                let size = size(width, px(underline::THICKNESS));
                let start = cell_at(origin, cell_width, line_height, *first, row);
                window.paint_quad(fill(Bounds::new(start, size), *color));
            }
            for span in &painted.underlines {
                underlines.clear();
                let cells = span.end - span.first;
                let start = cell_at(origin, cell_width, line_height, span.first, row);
                let y = frame.underline_y;
                underline_paint(span.kind, start, cells, cell_width, y, line_height, 1.0, underlines);
                paint_underlines(underlines, span.color, 1.0, window);
            }
        }
    });
}
