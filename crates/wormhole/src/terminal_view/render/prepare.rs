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
        let cols = (f32::from(bounds.size.width) / f32::from(cell_width)).floor().max(2.0) as usize;
        let rows = (f32::from(bounds.size.height) / f32::from(line_height)).floor().max(1.0) as usize;
        // Whole cells only: what is left over is split evenly, so the grid is centred in
        // the padded area (whole points, so the glyphs stay where the pixels are).
        let spare_x = (f32::from(bounds.size.width) - f32::from(cell_width) * cols as f32).max(0.0);
        let spare_y = (f32::from(bounds.size.height) - f32::from(line_height) * rows as f32).max(0.0);
        let area = bounds;
        let (left, top) = ((spare_x / 2.0).floor(), (spare_y / 2.0).floor());
        let bounds = Bounds::new(
            point(area.origin.x + px(left), area.origin.y + px(top)),
            size(cell_width * cols as f32, line_height * rows as f32),
        );
        // How far the edge cells' backgrounds run out: to the pane's edges.
        let reach = gpui_kit::Edges {
            left: px(left + PADDING_X),
            top: px(top + PADDING_Y),
            right: px(spare_x - left + PADDING_X),
            bottom: px(spare_y - top + PADDING_Y),
        };
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
            gutter: left + PADDING_X,
            frame,
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
            this.resize(
                GridSize {
                    cols: cols as u16,
                    rows: rows as u16,
                    cell_width: f32::from(cell_width),
                    cell_height: f32::from(line_height),
                },
                cx,
            );
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
        })
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
