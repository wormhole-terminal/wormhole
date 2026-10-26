//! The first half of a frame (the canvas's prepaint): lay the grid out in the pane, bring the
//! snapshot up to date, build the rows that changed (shaping only new words) and work out what
//! the paint needs that isn't in the rows (the cursor, the pictures, where baselines sit).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::rc::Rc;

use gpui_kit::{App, Bounds, Entity, Font, Pixels, Point, Size, Window, point, px, size};
use wormhole_term::GridSize;
use wormhole_term::images::Tiles;

use super::cursor::{CursorPaint, cursor_frame};
use super::frame::Geometry;
use super::rows::Style;
use super::shaping::{ShapeKey, colors_key, shape_blurred, shape_word, text_run};
use super::{RenderState, blink, images, words};
use crate::palette::Palette;
use crate::perf::{self, Count, Part};
use crate::terminal_view::{Layout, PADDING_X, PADDING_Y, TerminalView};

/// The per-frame part, handed from prepaint to paint.
pub(super) struct Frame {
    pub geometry: Geometry,
    /// How far the first and last columns' and rows' backgrounds run out past the grid.
    pub reach: gpui_kit::Edges<Pixels>,
    pub cols: usize,
    pub rows: usize,
    pub cursor: Option<CursorPaint>,
    /// Find's matches along the scrollback, 0 at its top and 1 at the screen's bottom.
    pub ticks: Vec<f32>,
    /// How far below a row's top the font's underline sits.
    pub underline_y: Pixels,
    /// How far below a row's top its baseline sits, and its strikethrough.
    pub baseline_y: Pixels,
    pub strike_y: Pixels,
    /// Blinking text is in the clock's off half (never while nothing blinks, the window is behind
    /// or Reduce Motion is on).
    pub blink_faded: bool,
    /// The inline images' tiles in view, and the pictures placed under the text.
    pub images: Vec<images::ImagePaint>,
    /// The pictures placed over the text.
    pub overlay: Vec<images::ImagePaint>,
    /// Where the grid must stay inside when it is not the size the pane makes (a text zoom or a live
    /// resize holds the PTY's size, so the grid may be larger than the pane): the pane with its padding.
    pub clip: Option<Bounds<Pixels>>,
}

/// What a frame's preparation reads from the view: copied out of `render`, which has the view and
/// its settings, so the canvas's closures own what they use.
pub(super) struct Prepare {
    pub font_size: f32,
    pub line_height: f32,
    pub ligatures: bool,
    pub base_font: Font,
    pub palette: Rc<Palette>,
    pub focused: bool,
    pub draws_cursor: bool,
    pub reduce_motion: bool,
    /// The window scales as the design fixed (Motion feature): the grid tolerates a pixel's rounding of the
    /// window's size, and the PTY is told a size by the rules of a live resize and a text zoom
    /// (`terminal_view::scaling`).
    pub scaling: bool,
    pub dock: Option<wormhole_core::onboarding::welcome::DockCell>,
    pub layout: Rc<Cell<Layout>>,
    pub state: Rc<RefCell<RenderState>>,
    pub entity: Entity<TerminalView>,
}

impl Prepare {
    pub fn run(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Option<Frame> {
        let font_size = px(self.font_size);
        let text_system = window.text_system().clone();
        let font_id = text_system.resolve_font(&self.base_font);
        let cell_width =
            text_system.advance(font_id, font_size, 'm').map(|size| size.width).unwrap_or(px(8.4));
        let line_height = px((self.font_size * self.line_height).round());
        let area = bounds;
        // A window whose size AppKit rounded to the screen's pixels (its steps are a fractional cell) may be a
        // pixel short of a whole column.
        let slack = if self.scaling { 1.0 / window.scale_factor() } else { 0.0 };
        let wanted = Placement::new(area, cell_width, line_height, slack);
        // What the PTY is told, and the grid drawn: the size the pane makes, unless it is held back.
        let (told_cols, told_rows) = self.entity.update(cx, |this, cx| {
            let size = GridSize {
                cols: wanted.cols as u16,
                rows: wanted.rows as u16,
                cell_width: f32::from(cell_width),
                cell_height: f32::from(line_height),
            };
            this.scale_grid(size, window, cx)
        });
        let held = (told_cols, told_rows) != (wanted.cols, wanted.rows);
        let placed =
            if held { Placement::held(area, cell_width, line_height, told_cols, told_rows) } else { wanted };
        let Placement { cols, rows, bounds, reach } = placed;
        let clip = held.then(|| {
            Bounds::new(
                point(area.origin.x - px(PADDING_X), area.origin.y - px(PADDING_Y)),
                size(area.size.width + px(2.0 * PADDING_X), area.size.height + px(2.0 * PADDING_Y)),
            )
        });
        let viewport = window.viewport_size();
        let frame = (
            (f32::from(viewport.width) - f32::from(bounds.size.width)).max(0.0),
            (f32::from(viewport.height) - f32::from(bounds.size.height)).max(0.0),
        );
        self.layout.set(Layout {
            origin: bounds.origin,
            cell: size(cell_width, line_height),
            cols,
            rows,
            gutter: PADDING_X,
            frame,
            spare: (
                (f32::from(area.size.width) - f32::from(bounds.size.width)).max(0.0),
                (f32::from(area.size.height) - f32::from(bounds.size.height)).max(0.0),
            ),
        });
        if let Some(dock) = self.dock {
            report_dock(cx, bounds.origin, size(cell_width, line_height), dock);
        }
        let geometry = crate::window_ops::geometry(
            window,
            cx,
            crate::window_ops::PaneSize {
                cell: (f32::from(cell_width), f32::from(line_height)),
                cols,
                rows,
                frame,
            },
        );
        // Rows just above the viewport, kept for the soft edge at the top.
        let overscan = (PADDING_Y / f32::from(line_height)).ceil() as usize;

        let mut state = self.state.borrow_mut();
        let RenderState { frame: cache, images: image_cache, words, outlines, .. } = &mut *state;
        let mut found = Vec::new();
        let mut ticks = Vec::new();
        let palette = &self.palette;
        let has_terminal = self.entity.update(cx, |this, cx| {
            this.share_palette(palette);
            this.share_window(geometry);
            let Some(pty) = &this.pty else { return false };
            let span = perf::span();
            pty.snapshot(&mut cache.snapshot, overscan);
            span.end(Part::Snapshot);
            found = this.found_rows(&cache.snapshot);
            ticks = this.found_ticks(&cache.snapshot);
            this.mode = cache.snapshot.mode;
            let blinking = cache.snapshot.cursor.is_some_and(|cursor| cursor.blinking);
            this.blink_wanted(blinking && self.focused && !self.reduce_motion, cx);
            true
        });
        if !has_terminal {
            return None;
        }

        let geometry = Geometry {
            origin: bounds.origin,
            cell_width,
            line_height,
            scale: window.scale_factor(),
            font_size: self.font_size,
        };
        let key = ShapeKey {
            palette: palette.clone(),
            ligatures: self.ligatures,
            font: font_id,
            font_size,
            cell_width,
            colors: colors_key(&cache.snapshot.colors),
        };
        words.sync((font_id, font_size, cell_width));
        words.start_frame(Some(std::time::Instant::now() + words::SHAPE_BUDGET));
        let base_font = &self.base_font;
        let rows_span = perf::span();
        let stats = cache.prepare(
            key,
            geometry,
            palette,
            &found,
            |segment, spare| {
                let glyphs = words.glyphs_into(segment, cell_width, spare.unwrap_or_default(), |segment| {
                    let span = perf::span();
                    let shaped =
                        shape_word(segment, &text_system, base_font, font_size, cell_width, outlines);
                    span.end(Part::Shape);
                    perf::count(Count::WordsShaped, 1);
                    shaped
                });
                (glyphs, !words.take_deferred())
            },
            |text, runs| {
                let span = perf::span();
                let shaped = shape_blurred(text, runs, &text_system, base_font, font_size, cell_width);
                span.end(Part::Blur);
                shaped
            },
        );
        rows_span.end(Part::Rows);
        perf::count(Count::RowsBuilt, stats.rows.built);
        // Words the budget didn't reach are shaped in the next frame, also when nothing
        // else changes by then.
        if stats.provisional > 0 {
            window.request_animation_frame();
        }
        let blinking = blink::wanted(cache.any_blinks(), window.is_window_active(), self.reduce_motion);
        self.entity.update(cx, |this, cx| this.text_blink_wanted(blinking, cx));
        let blink_faded = blinking && blink::faded_now();
        let cursor = self
            .draws_cursor
            .then(|| cursor_frame(&cache.snapshot, palette, self.focused, geometry, &text_system, base_font))
            .flatten();
        let underline_y = gpui_kit::underline_y_offset(
            line_height,
            text_system.ascent(font_id, font_size),
            text_system.descent(font_id, font_size),
        );
        // Where a line of the base font puts its baseline and strikethrough (as
        // `ShapedLine::paint` does), so every row's glyphs sit on the same one.
        let (baseline_y, strike_y) = {
            let run = text_run(1, &Style::of(&Default::default(), palette.foreground), base_font);
            let line = text_system.shape_line("m".into(), font_size, &[run], None);
            let baseline = (line_height - line.ascent - line.descent) / 2.0 + line.ascent;
            (baseline, (line.ascent * 0.5 + baseline) * 0.5)
        };
        // The pictures: under the placeholder cells and placed in the grid. The terminal's
        // lock for what the engine knows of the placeholders, then the store's for the
        // pixels; never both at once.
        let images_span = perf::span();
        let (images, overlay) = self.entity.read(cx).pty.as_ref().map_or_else(Default::default, |pty| {
            let mut tiles: HashMap<(u32, u32), Tiles> = HashMap::new();
            for (_, run) in cache.image_runs() {
                let pair = (run.image, run.placement);
                if let Entry::Vacant(slot) = tiles.entry(pair)
                    && let Some(found) = pty.virtual_placement(pair.0, pair.1)
                {
                    slot.insert(found);
                }
            }
            let store = &mut pty.images();
            let mut paints = images::image_paints(cache.image_runs(), &tiles, store, image_cache, geometry);
            let snapshot = &cache.snapshot;
            let (under, over) = images::placement_paints(
                snapshot.placements(),
                snapshot.display_offset,
                store,
                image_cache,
                bounds,
                geometry,
            );
            paints.extend(under);
            (paints, over)
        });
        images_span.end(Part::Images);
        Some(Frame {
            geometry,
            reach,
            cols,
            rows,
            cursor,
            ticks,
            underline_y,
            baseline_y,
            strike_y,
            blink_faded,
            images,
            overlay,
            clip,
        })
    }
}

/// Where the grid sits in the canvas: whole cells from the top left, and the leftover width and
/// height go to the right and bottom edges (as Terminal.app, iTerm2 and Ghostty do by default).
/// Centring the grid would move the text by up to half a cell whenever a side column opens or the
/// window is resized and the number of columns (or rows) changes; anchored, only what is past the
/// last column or row moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Placement {
    pub cols: usize,
    pub rows: usize,
    /// The grid's own bounds, whole cells.
    pub bounds: Bounds<Pixels>,
    /// How far the edge cells' backgrounds run out: to the pane's edges.
    pub reach: gpui_kit::Edges<Pixels>,
}

impl Placement {
    /// `slack` is how far short of a whole column or row the area may be and still have it, in points:
    /// zero, or a screen pixel for a window whose size is rounded to pixels.
    pub fn new(area: Bounds<Pixels>, cell_width: Pixels, line_height: Pixels, slack: f32) -> Self {
        let cols = ((f32::from(area.size.width) + slack) / f32::from(cell_width)).floor().max(2.0) as usize;
        let rows = ((f32::from(area.size.height) + slack) / f32::from(line_height)).floor().max(1.0) as usize;
        Self::held(area, cell_width, line_height, cols, rows)
    }

    /// The grid of `cols` by `rows` cells at the area's top left, whatever room the area has: smaller than
    /// the area it leaves the rest to the right and bottom edges, larger it runs past them (the paint is
    /// clipped to the area and its padding).
    pub fn held(
        area: Bounds<Pixels>,
        cell_width: Pixels,
        line_height: Pixels,
        cols: usize,
        rows: usize,
    ) -> Self {
        let grid = size(cell_width * cols as f32, line_height * rows as f32);
        let spare_x = (f32::from(area.size.width) - f32::from(grid.width)).max(0.0);
        let spare_y = (f32::from(area.size.height) - f32::from(grid.height)).max(0.0);
        let reach = gpui_kit::Edges {
            left: px(PADDING_X),
            top: px(PADDING_Y),
            right: px(spare_x + PADDING_X),
            bottom: px(spare_y + PADDING_Y),
        };
        Self { cols, rows, bounds: Bounds::new(area.origin, grid), reach }
    }
}

/// Where the start animation's ship docks: the middle of the chevron of `dock` (in cells, from the
/// grid's top left, so a cell's middle is `n + 0.5`) and how tall that chevron is, in window points.
pub(in crate::terminal_view) fn dock_landing(
    origin: Point<Pixels>,
    cell: Size<Pixels>,
    dock: wormhole_core::onboarding::welcome::DockCell,
) -> wormhole_core::onboarding::intro::voyage::Landing {
    let chevron = [f32::from(origin.x + cell.width * dock.col), f32::from(origin.y + cell.height * dock.row)];
    wormhole_core::onboarding::intro::voyage::Landing::on_chevron(chevron, f32::from(cell.height) * dock.rows)
}

/// Tell the start animation where its ship docks (`dock_landing`), every frame the grid is laid out.
fn report_dock(
    cx: &App,
    origin: Point<Pixels>,
    cell: Size<Pixels>,
    dock: wormhole_core::onboarding::welcome::DockCell,
) {
    let landing = dock_landing(origin, cell, dock);
    crate::intro::set_landing(cx, [landing.centre[0], landing.centre[1], landing.size]);
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, point, px, size};

    use super::{PADDING_X, PADDING_Y, Placement};

    fn area(width: f32, height: f32) -> Bounds<gpui_kit::Pixels> {
        Bounds::new(point(px(40.0), px(30.0)), size(px(width), px(height)))
    }

    #[test]
    fn the_grid_keeps_its_origin_when_the_width_changes_the_columns() {
        let (cell, line) = (px(8.4), px(18.0));
        let wide = Placement::new(area(700.0, 400.0), cell, line, 0.0);
        let narrow = Placement::new(area(700.0 - 3.0 * 8.4 - 4.0, 400.0), cell, line, 0.0);
        assert_ne!(wide.cols, narrow.cols, "the width really changed the columns");
        assert_eq!(wide.bounds.origin, narrow.bounds.origin);
        assert_eq!(wide.bounds.origin, point(px(40.0), px(30.0)), "the area's own corner");
    }

    #[test]
    fn the_grid_keeps_its_origin_when_the_height_changes_the_rows() {
        let (cell, line) = (px(8.4), px(18.0));
        let tall = Placement::new(area(700.0, 400.0), cell, line, 0.0);
        let short = Placement::new(area(700.0, 400.0 - 18.0 - 5.0), cell, line, 0.0);
        assert_ne!(tall.rows, short.rows);
        assert_eq!(tall.bounds.origin, short.bounds.origin);
    }

    #[test]
    fn the_leftover_goes_to_the_right_and_bottom_edges() {
        let (cell, line) = (px(10.0), px(20.0));
        let placed = Placement::new(area(105.0, 207.0), cell, line, 0.0);
        assert_eq!((placed.cols, placed.rows), (10, 10));
        assert_eq!(placed.bounds.size, size(px(100.0), px(200.0)));
        assert_eq!(placed.reach.left, px(PADDING_X));
        assert_eq!(placed.reach.top, px(PADDING_Y));
        assert_eq!(placed.reach.right, px(5.0 + PADDING_X));
        assert_eq!(placed.reach.bottom, px(7.0 + PADDING_Y));
    }
}

#[cfg(test)]
mod held_tests {
    use gpui_kit::{Bounds, point, px, size};

    use super::{PADDING_X, Placement};

    fn area(width: f32, height: f32) -> Bounds<gpui_kit::Pixels> {
        Bounds::new(point(px(40.0), px(30.0)), size(px(width), px(height)))
    }

    #[test]
    fn a_pixel_of_slack_keeps_a_column_a_rounded_window_is_short_of() {
        let (cell, line) = (px(8.4), px(18.0));
        // 20 columns of 8.4 are 168.0; a window rounded down to the half point is 167.5.
        assert_eq!(Placement::new(area(167.5, 180.0), cell, line, 0.0).cols, 19);
        assert_eq!(Placement::new(area(167.5, 180.0), cell, line, 0.5).cols, 20);
        assert_eq!(
            Placement::new(area(160.0, 180.0), cell, line, 0.5).cols,
            19,
            "a real shortfall still counts"
        );
    }

    #[test]
    fn a_held_grid_keeps_its_size_whatever_the_area_makes() {
        let (cell, line) = (px(10.0), px(20.0));
        let smaller = Placement::held(area(105.0, 207.0), cell, line, 8, 8);
        assert_eq!((smaller.cols, smaller.rows), (8, 8));
        assert_eq!(smaller.bounds.size, size(px(80.0), px(160.0)));
        assert_eq!(smaller.reach.right, px(25.0 + PADDING_X), "the backgrounds run out to the pane's edge");
        let larger = Placement::held(area(105.0, 207.0), cell, line, 12, 12);
        assert_eq!(larger.bounds.size, size(px(120.0), px(240.0)), "it runs past the area");
        assert_eq!(larger.reach.right, px(PADDING_X), "and no further out than the padding says");
        assert_eq!(larger.bounds.origin, point(px(40.0), px(30.0)), "from the same corner");
    }
}
