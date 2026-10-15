//! Painting the terminal grid (and the blurred lines under the title bar).
//!
//! A frame takes one short lock to bring a `ScreenSnapshot` up to date (only
//! rows the engine reports changed are copied), then builds only rows whose
//! content or selection changed (`RowCache`; scrolled rows are found again by
//! content), shaping only words it hasn't seen before (`WordCache`), and paints
//! glyph by glyph from that cache (`FrameCache`). The view is
//! embedded with `.cached(..)`, so window frames the terminal didn't cause
//! (sidebar, tick, hover) reuse its last paint instead of running any of this.

mod blink;
mod blur;
mod cache;
mod cursor;
mod frame;
mod glyphs;
mod images;
mod minimap;
mod rows;
mod scrollbar;
mod underline;
mod words;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod guards;

use self::frame::{FrameCache, Geometry};
use self::rows::{Segment, Style};
pub(super) use self::scrollbar::ScrollbarState;
use self::underline::{UnderlinePaint, underline_paint};
use self::words::{Glyph, Shaped, WordCache};
use super::blocks;
use super::input::TerminalCommand;
use super::*;
use crate::palette::Palette;
use crate::scroller::ScrollerPrefs;
use crate::style::{Edges, TextRole};
use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::input as text_input;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use wormhole_core::keymap;
use wormhole_core::terminal::ScreenSnapshot;
use wormhole_core::terminal::images::Tiles;
use wormhole_core::terminal::types::{COLOR_COUNT, Color, Colors, NamedColor};

/// What every cached row was shaped with; any change rebuilds them all.
#[derive(Clone, PartialEq)]
struct ShapeKey {
    /// Compared by value: a settings change that leaves the colors alone (or
    /// rebuilds an equal palette) keeps the rows.
    palette: Rc<Palette>,
    ligatures: bool,
    font: FontId,
    font_size: Pixels,
    cell_width: Pixels,
    /// The colors programs set (OSC 4/10/11): a cell keeps its content when its
    /// palette entry changes, so a row found again by content would be stale.
    colors: u64,
}

/// What a word's glyphs were shaped with: the font (its features carry the
/// ligatures), its size and the cell they are pinned to.
type WordsKey = (FontId, Pixels, Pixels);

/// A fingerprint of the runtime color overrides.
fn colors_key(colors: &Colors) -> u64 {
    (0..COLOR_COUNT).fold(0u64, |hash, index| {
        let value =
            colors[index].map_or(u64::MAX, |c| u64::from(c.r) << 16 | u64::from(c.g) << 8 | u64::from(c.b));
        (hash.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95)
    })
}

/// Terminal grids painted since launch, for devctl's `paints` (frames actually drawn, which no
/// program in the terminal can see).
#[cfg(feature = "devctl")]
pub(crate) static PAINTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// What the canvas keeps between frames.
#[derive(Default)]
pub(super) struct RenderState {
    frame: FrameCache<ShapeKey, Vec<Glyph>, ShapedLine>,
    words: WordCache<WordsKey>,
    minimap: wormhole_core::terminal::minimap::MinimapSnapshot,
    /// Where the minimap was last drawn, for clicks on it.
    minimap_bounds: Bounds<Pixels>,
    /// Where the scrollbar was last drawn, if it was, for the pointer.
    scrollbar: Option<scrollbar::ScrollbarLayout>,
    /// The inline images as GPUI frames, one conversion per image.
    images: images::ImageCache,
}

impl RenderState {
    /// The last frame's copy of the screen.
    pub(super) fn snapshot(&self) -> &ScreenSnapshot {
        &self.frame.snapshot
    }

    /// How far the last frame was scrolled into the history.
    pub(super) fn display_offset(&self) -> usize {
        self.frame.snapshot.display_offset
    }

    /// How long the history was in the last frame.
    pub(super) fn history_size(&self) -> usize {
        self.frame.snapshot.history_size
    }
}

/// The per-frame part, handed from prepaint to paint.
struct Frame {
    geometry: Geometry,
    cursor: Option<CursorPaint>,
    /// Find's matches along the scrollback, 0 at its top and 1 at the screen's bottom.
    ticks: Vec<f32>,
    /// How far below a row's top the font's underline sits.
    underline_y: Pixels,
    /// How far below a row's top its baseline sits, and its strikethrough.
    baseline_y: Pixels,
    strike_y: Pixels,
    /// Blinking text is in the clock's off half (never while nothing blinks, the window is behind
    /// or Reduce Motion is on).
    blink_faded: bool,
    /// The inline images' tiles in view, and the pictures placed under the text.
    images: Vec<images::ImagePaint>,
    /// The pictures placed over the text.
    overlay: Vec<images::ImagePaint>,
}

/// Find's match ticks: along the pane's right edge, in its padding, beside the grid.
const TICK_W: f32 = 6.0;
const TICK_H: f32 = 2.0;
const TICK_INSET: f32 = 2.0;

struct CursorPaint {
    rect: Bounds<Pixels>,
    color: Hsla,
    /// The character inside a block cursor, redrawn on top of it.
    text: Option<(Point<Pixels>, ShapedLine)>,
}

/// GPUI's text run for a style.
fn text_run(len: usize, style: &Style, base_font: &Font) -> TextRun {
    let mut font = base_font.clone();
    if style.bold {
        font.weight = FontWeight::BOLD;
    }
    if style.italic {
        font.style = FontStyle::Italic;
    }
    TextRun {
        len,
        font,
        color: style.fg,
        background_color: None,
        underline: None,
        strikethrough: style
            .strike
            .then_some(StrikethroughStyle { thickness: px(1.0), color: Some(style.fg) }),
    }
}

/// Shape one word of a row, each glyph pinned to its cells.
fn shape_word(
    segment: &Segment,
    text_system: &WindowTextSystem,
    base_font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
) -> Vec<Shaped> {
    let runs: Vec<TextRun> =
        segment.runs.iter().map(|(len, style)| text_run(*len, style, base_font)).collect();
    let advance = if segment.wide { cell_width * 2.0 } else { cell_width };
    let line = text_system.shape_line(segment.text.clone().into(), font_size, &runs, Some(advance));
    line.runs
        .iter()
        .flat_map(|run| {
            run.glyphs.iter().map(|glyph| Shaped {
                position: glyph.position,
                index: glyph.index,
                font_id: run.font_id,
                id: glyph.id,
                emoji: glyph.is_emoji,
            })
        })
        .collect()
}

/// Shape a faint row above the viewport.
fn shape_blurred(
    text: String,
    runs: Vec<(usize, Hsla)>,
    text_system: &WindowTextSystem,
    base_font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
) -> ShapedLine {
    let runs: Vec<TextRun> = runs
        .into_iter()
        .map(|(len, color)| TextRun {
            len,
            font: base_font.clone(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        })
        .collect();
    text_system.shape_line(text.into(), font_size, &runs, Some(cell_width))
}

/// The cursor's rectangle and, inside a block, its character redrawn.
fn cursor_frame(
    snapshot: &ScreenSnapshot,
    palette: &Palette,
    focused: bool,
    geometry: Geometry,
    text_system: &WindowTextSystem,
    base_font: &Font,
) -> Option<CursorPaint> {
    let at = snapshot.cursor?;
    let row = snapshot.rows().get(at.row)?;
    let cell = row.cells().get(at.col)?;
    let Geometry { origin, cell_width, line_height, font_size, .. } = geometry;
    let corner = point(origin.x + cell_width * at.col as f32, origin.y + line_height * at.row as f32);
    let width = cell_width * if at.wide { 2.0 } else { 1.0 };
    let rect = cursor::cursor_rect(at.shape, focused, corner, width, line_height);
    let shown = cell.c != ' ' && cell.c != '\0' && cell.c != '\t' && !rows::is_shape(cell.c);
    let text = (cursor::redraws_text(rect, focused) && shown).then(|| {
        let selected = snapshot.selected_columns(at.row).is_some_and(|(a, b)| (a..=b).contains(&at.col));
        let (_, background) = palette.cell_colors(cell, selected, &snapshot.colors);
        let style = Style::of(cell, cursor::text_color(palette, background));
        let mut text = String::from(cell.c);
        text.extend(row.extras().zerowidth(at.col));
        let run = text_run(text.len(), &style, base_font);
        (corner, text_system.shape_line(text.into(), px(font_size), &[run], None))
    });
    // OSC 12 recolors the cursor.
    let color = palette.resolve(Color::Named(NamedColor::Cursor), &snapshot.colors);
    Some(CursorPaint { rect, color, text })
}

/// Where the start animation's ship docks: the middle of the chevron of `dock` (in cells, from the
/// grid's top left, so a cell's middle is `n + 0.5`) and how tall that chevron is, in window points.
pub(super) fn dock_landing(
    origin: Point<Pixels>,
    cell: Size<Pixels>,
    dock: wormhole_core::welcome::DockCell,
) -> wormhole_core::intro::voyage::Landing {
    let chevron = [f32::from(origin.x + cell.width * dock.col), f32::from(origin.y + cell.height * dock.row)];
    wormhole_core::intro::voyage::Landing::on_chevron(chevron, f32::from(cell.height) * dock.rows)
}

/// Tell the start animation where its ship docks (`dock_landing`), every frame the grid is laid out.
fn report_dock(cx: &App, origin: Point<Pixels>, cell: Size<Pixels>, dock: wormhole_core::welcome::DockCell) {
    let landing = dock_landing(origin, cell, dock);
    crate::intro::set_landing(cx, [landing.centre[0], landing.centre[1], landing.size]);
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.redraw.rendered();
        let entity = cx.entity();
        let store = cx.global::<SettingsStore>();
        let palette = store.palette.clone();
        // A zoom kept while Settings changed the size still stays within what Settings allows.
        let font_size_setting =
            (store.settings.font_size + self.font_zoom).clamp(*FONT_SIZES.start(), *FONT_SIZES.end());
        let line_height_setting = store.settings.line_height;
        let ligatures = store.settings.font_ligatures;
        let layout_cell = self.layout.clone();
        let mut base_font = font(store.settings.font_family.clone());
        base_font.features = crate::fonts::ligature_features(ligatures);
        let focused = self.focus_handle.is_focused(window);
        let exited = self.status == Status::Exited;
        let draws_cursor = cursor::drawn(self.status) && !self.cursor_off;
        let reduce_motion = cx.reduce_motion();
        let blink_allowed = !reduce_motion;
        let bell_glow = self.bell_glow().then_some(store.ui.agent_attention);
        let ui = store.ui;
        let drop_tint = token(&store.tokens, "state.highlight");
        let find_tick = palette.find_current;
        let selecting = self.selecting;
        let prepaint_state = self.render_state.clone();
        let top_inset = self.top_inset;
        let dock = self.dock;
        let minimap = store.settings.minimap;
        let paint_state = self.render_state.clone();
        let weak = entity.downgrade();
        let link_rows = self.hovered_link_rows(self.render_state.borrow().display_offset());
        let link_color = palette.foreground;
        let hovered_block = self.hovered_block();
        let background_override =
            self.render_state.borrow().snapshot().colors[NamedColor::Background].map(crate::palette::to_hsla);
        let has_blocks = self.pty.as_ref().is_some_and(Pty::has_blocks);

        let grid = canvas(
            move |bounds, window, cx| {
                let font_size = px(font_size_setting);
                let text_system = window.text_system().clone();
                let font_id = text_system.resolve_font(&base_font);
                let cell_width =
                    text_system.advance(font_id, font_size, 'm').map(|size| size.width).unwrap_or(px(8.4));
                let line_height = px((font_size_setting * line_height_setting).round());
                let cols = (f32::from(bounds.size.width) / f32::from(cell_width)).floor().max(2.0) as usize;
                let rows = (f32::from(bounds.size.height) / f32::from(line_height)).floor().max(1.0) as usize;
                let viewport = window.viewport_size();
                let frame = (
                    (f32::from(viewport.width) - f32::from(bounds.size.width)).max(0.0),
                    (f32::from(viewport.height) - f32::from(bounds.size.height)).max(0.0),
                );
                layout_cell.set(Layout {
                    origin: bounds.origin,
                    cell: size(cell_width, line_height),
                    cols,
                    rows,
                    frame,
                });
                if let Some(dock) = dock {
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
                // Rows just above the viewport, shown blurred under the title bar.
                let overscan = ((top_inset + PADDING_Y) / f32::from(line_height)).ceil() as usize;

                let mut state = prepaint_state.borrow_mut();
                let RenderState { frame: cache, images: image_cache, words, .. } = &mut *state;
                let mut found = Vec::new();
                let mut ticks = Vec::new();
                let (has_terminal, above_changed) = entity.update(cx, |this, cx| {
                    this.resize(GridSize {
                        cols: cols as u16,
                        rows: rows as u16,
                        cell_width: f32::from(cell_width),
                        cell_height: f32::from(line_height),
                    });
                    this.share_palette(&palette);
                    this.share_window(geometry);
                    let Some(pty) = &this.pty else { return (false, false) };
                    pty.snapshot(&mut cache.snapshot, overscan);
                    found = this.found_rows(&cache.snapshot);
                    ticks = this.found_ticks(&cache.snapshot);
                    this.mode = cache.snapshot.mode;
                    let blinking = cache.snapshot.cursor.is_some_and(|cursor| cursor.blinking);
                    this.blink_wanted(blinking && focused && blink_allowed, cx);
                    let above = cache.snapshot.has_content_above();
                    let changed = above != this.content_above;
                    this.content_above = above;
                    (true, changed)
                });
                if above_changed {
                    // The workspace read the old value for this frame's title bar.
                    let entity = entity.downgrade();
                    cx.defer(move |cx| {
                        let _ = entity.update(cx, |_, cx| cx.notify());
                    });
                }
                if !has_terminal {
                    return None;
                }

                let geometry = Geometry {
                    origin: bounds.origin,
                    cell_width,
                    line_height,
                    scale: window.scale_factor(),
                    font_size: font_size_setting,
                };
                let key = ShapeKey {
                    palette: palette.clone(),
                    ligatures,
                    font: font_id,
                    font_size,
                    cell_width,
                    colors: colors_key(&cache.snapshot.colors),
                };
                words.sync((font_id, font_size, cell_width));
                cache.prepare(
                    key,
                    geometry,
                    &palette,
                    &found,
                    |segment| {
                        words.glyphs(&segment, cell_width, |segment| {
                            shape_word(segment, &text_system, &base_font, font_size, cell_width)
                        })
                    },
                    |text, runs| shape_blurred(text, runs, &text_system, &base_font, font_size, cell_width),
                );
                let blinking = blink::wanted(cache.any_blinks(), window.is_window_active(), reduce_motion);
                entity.update(cx, |this, cx| this.text_blink_wanted(blinking, cx));
                let blink_faded = blinking && blink::faded_now();
                let cursor = draws_cursor
                    .then(|| {
                        cursor_frame(&cache.snapshot, &palette, focused, geometry, &text_system, &base_font)
                    })
                    .flatten();
                let underline_y = underline_y_offset(
                    line_height,
                    text_system.ascent(font_id, font_size),
                    text_system.descent(font_id, font_size),
                );
                // Where a line of the base font puts its baseline and strikethrough (as
                // `ShapedLine::paint` does), so every row's glyphs sit on the same one.
                let (baseline_y, strike_y) = {
                    let run = text_run(1, &Style::of(&Default::default(), palette.foreground), &base_font);
                    let line = text_system.shape_line("m".into(), font_size, &[run], None);
                    let baseline = (line_height - line.ascent - line.descent) / 2.0 + line.ascent;
                    (baseline, (line.ascent * 0.5 + baseline) * 0.5)
                };
                // The pictures: under the placeholder cells and placed in the grid. The terminal's
                // lock for what the engine knows of the placeholders, then the store's for the
                // pixels; never both at once.
                let (images, overlay) = entity.read(cx).pty.as_ref().map_or_else(Default::default, |pty| {
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
                    let mut paints =
                        images::image_paints(cache.image_runs(), &tiles, store, image_cache, geometry);
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
                Some(Frame {
                    geometry,
                    cursor,
                    ticks,
                    underline_y,
                    baseline_y,
                    strike_y,
                    blink_faded,
                    images,
                    overlay,
                })
            },
            move |bounds, frame, window, cx| {
                let Some(frame) = frame else { return };
                #[cfg(feature = "devctl")]
                PAINTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let state = paint_state.borrow();
                let cache = &state.frame;
                let Geometry { origin, cell_width, line_height, .. } = frame.geometry;
                let font_size = px(frame.geometry.font_size);
                let at = |col: usize, row: usize| {
                    point(origin.x + cell_width * col as f32, origin.y + line_height * row as f32)
                };
                for (k, line) in cache.blurred() {
                    let row_origin = point(origin.x, origin.y - line_height * k as f32);
                    for (dx, dy) in blur::BLUR_TAPS {
                        let tap = point(row_origin.x + px(dx), row_origin.y + px(dy));
                        let _ = line.paint(tap, line_height, TextAlign::Left, None, window, cx);
                    }
                }
                paint_blocks(&cache.snapshot, hovered_block, ui, bounds, line_height, window);
                // Each quad painted outside a layer goes into the scene's bounds tree to find its
                // z-order, and a screen of colored cells is thousands of them: in a full-screen
                // truecolor animation that tree took nine tenths of the main thread, and the window
                // stopped answering. A layer gives everything in it one order. The cells' quads
                // never overlap one another, so nothing about the stacking changes; the images
                // stay between the two layers, where they are now.
                let grid = window.content_mask().bounds;
                window.paint_layer(grid, |window| {
                    for (row, painted) in cache.rows() {
                        for (start, end, color) in &painted.backgrounds {
                            let width = cell_width * (end - start) as f32;
                            window.paint_quad(fill(
                                Bounds::new(at(*start, row), size(width, line_height)),
                                *color,
                            ));
                        }
                    }
                });
                for paint in &frame.images {
                    let painted = window.paint_image(
                        paint.bounds,
                        paint.image_bounds,
                        Corners::default(),
                        paint.image.clone(),
                        0,
                        false,
                    );
                    if let Err(err) = painted {
                        tracing::debug!("an inline image didn't paint: {err}");
                    }
                }
                // Block elements and box drawing, drawn as quads and strokes: one layer too.
                window.paint_layer(grid, |window| {
                    for (_, painted) in cache.rows() {
                        for (rect, color) in &painted.quads {
                            window.paint_quad(fill(*rect, *color));
                        }
                        for (path, color) in &painted.strokes {
                            window.paint_path(path.clone(), *color);
                        }
                    }
                });
                // Unfocused cursor: hollow block.
                if let Some(cursor) = &frame.cursor {
                    if focused {
                        window.paint_quad(fill(cursor.rect, cursor.color));
                    } else {
                        window.paint_quad(outline(cursor.rect, cursor.color, BorderStyle::Solid));
                    }
                }
                // The text in a layer of its own, for the same reason as the backgrounds: a glyph
                // painted on its own goes into the bounds tree, and a full screen of text is
                // thousands of them. The lines through and under it go in the next layer, so they
                // still draw over the glyphs.
                window.paint_layer(grid, |window| {
                    for (row, painted) in cache.rows() {
                        for (col, glyphs) in &painted.lines {
                            let word = at(*col, row);
                            for glyph in glyphs {
                                let origin = point(
                                    word.x + glyph.position.x,
                                    word.y + frame.baseline_y + glyph.position.y,
                                );
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
                for paint in &frame.overlay {
                    let painted = window.paint_image(
                        paint.bounds,
                        paint.image_bounds,
                        Corners::default(),
                        paint.image.clone(),
                        0,
                        false,
                    );
                    if let Err(err) = painted {
                        tracing::debug!("a placed image didn't paint: {err}");
                    }
                }
                let mut underlines = Vec::new();
                window.paint_layer(grid, |window| {
                    for (row, painted) in cache.rows() {
                        for (first, end, color) in &painted.struck {
                            let width = cell_width * (end - first) as f32;
                            let start = at(*first, row);
                            let style = StrikethroughStyle { thickness: px(1.0), color: Some(*color) };
                            window.paint_strikethrough(
                                point(start.x, start.y + frame.strike_y),
                                width,
                                &style,
                            );
                        }
                        for (first, end, color) in &painted.overlined {
                            // The same weight as an underline, along the top of the cell.
                            let width = cell_width * (end - first) as f32;
                            let size = size(width, px(underline::THICKNESS));
                            window.paint_quad(fill(Bounds::new(at(*first, row), size), *color));
                        }
                        for span in &painted.underlines {
                            underlines.clear();
                            let cells = span.end - span.first;
                            let start = at(span.first, row);
                            let y = frame.underline_y;
                            underline_paint(
                                span.kind,
                                start,
                                cells,
                                cell_width,
                                y,
                                line_height,
                                &mut underlines,
                            );
                            paint_underlines(&underlines, span.color, window);
                        }
                    }
                });
                for (row, columns) in &link_rows {
                    underlines.clear();
                    let (start, cells, y) = (at(columns.start, *row), columns.len(), frame.underline_y);
                    let kind = rows::UnderlineKind::Single;
                    underline_paint(kind, start, cells, cell_width, y, line_height, &mut underlines);
                    paint_underlines(&underlines, link_color, window);
                }
                if let Some((corner, line)) = frame.cursor.as_ref().and_then(|cursor| cursor.text.as_ref()) {
                    let _ = line.paint(*corner, line_height, TextAlign::Left, None, window, cx);
                }
                paint_ticks(&frame.ticks, bounds, find_tick, window);
                if selecting {
                    drag_outside_listener(weak.clone(), window);
                }
            },
        )
        .size_full();

        div()
            .id("terminal")
            .track_focus(&self.focus_handle)
            .key_context("Terminal")
            // The Edit menu sends the text fields' own actions, so the terminal answers them too
            // (the keys come through `TerminalKeys` first); without a selection they do nothing.
            .on_action(cx.listener(|this, _: &text_input::Copy, _, cx| {
                this.run_command(TerminalCommand::Copy, cx);
            }))
            .on_action(cx.listener(|this, _: &text_input::Paste, _, cx| {
                this.run_command(TerminalCommand::Paste, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &text_input::SelectAll, _, cx| {
                this.run_command(TerminalCommand::SelectAll, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ClearBuffer, _, cx| {
                this.run_command(TerminalCommand::Clear, cx);
                cx.notify();
            }))
            // The command blocks' actions, only once the shell has shown it has integration:
            // without blocks their menu items are greyed out.
            .when(has_blocks, |this| {
                this.on_action(cx.listener(|this, _: &PreviousPrompt, _, cx| {
                    this.jump_prompt(wormhole_core::terminal::Direction::Up, cx)
                }))
                .on_action(cx.listener(|this, _: &NextPrompt, _, cx| {
                    this.jump_prompt(wormhole_core::terminal::Direction::Down, cx)
                }))
                .on_action(cx.listener(|this, _: &SelectOutput, _, cx| {
                    this.block_action(blocks::BlockAction::SelectOutput, cx)
                }))
                .on_action(cx.listener(|this, _: &CopyOutput, _, cx| {
                    this.block_action(blocks::BlockAction::CopyOutput, cx)
                }))
                .on_action(cx.listener(|this, _: &CopyCommand, _, cx| {
                    this.block_action(blocks::BlockAction::CopyCommand, cx)
                }))
                .on_action(cx.listener(|this, _: &RerunCommand, _, cx| {
                    this.block_action(blocks::BlockAction::Rerun, cx)
                }))
            })
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !hovered {
                    this.leave_blocks(cx);
                }
            }))
            .size_full()
            // A program that set the background (OSC 11) has it behind the padding too, as the
            // grid's cells have it; the theme's own is the window's and shows through.
            .when_some(background_override, |this, color| this.bg(color))
            .min_w(px(MIN_WIDTH))
            .pl(px(PADDING_X))
            .pr(px(Self::right_inset(cx)))
            .pb(px(PADDING_Y))
            .pt(px(self.top_inset + PADDING_Y))
            // A program that takes the mouse gets clicks, not a text selection (⇧ still selects).
            .cursor(if self.hovered_link.is_some() {
                CursorStyle::PointingHand
            } else if self.mode.intersects(TermMode::MOUSE_MODE) {
                CursorStyle::Arrow
            } else {
                CursorStyle::IBeam
            })
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, window, cx| {
                this.hover_link(window.mouse_position(), event.modifiers.platform, cx);
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            // A release over the title bar, the sidebar or outside the window
            // still ends the drag (and is reported to apps that track the mouse).
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            // A tint (not a border) so the grid keeps its size while files hover over it.
            .drag_over::<ExternalPaths>(move |style, _, _, _| style.bg(drop_tint))
            .on_drop(cx.listener(Self::drop_files))
            .child(grid)
            .when(minimap, |this| this.child(self.render_minimap(cx)))
            // The minimap is the scroller when it is shown.
            .when(!minimap, |this| this.child(self.render_scrollbar(cx)))
            .children(self.render_block_readout(cx))
            .children(bell_glow.map(|color| bell_edge(ui, color)))
            .when(exited, |this| this.child(self.render_exited(cx)))
    }
}

/// The command blocks: the hovered block's tint under its rows, and each block's gutter
/// mark in the pane's left padding, from the block's first row to its last one in view.
fn paint_blocks(
    snapshot: &ScreenSnapshot,
    hovered: Option<u64>,
    ui: crate::settings::UiColors,
    grid: Bounds<Pixels>,
    line_height: Pixels,
    window: &mut Window,
) {
    let left = grid.left() - px(PADDING_X);
    let rows = snapshot.rows().len();
    for span in snapshot.block_spans() {
        let top = grid.top() + line_height * span.rows.start as f32;
        let height = line_height * span.rows.len() as f32;
        if hovered == Some(span.id) {
            let tint = Bounds::new(point(left, top), size(grid.right() - left, height));
            window.paint_quad(fill(tint, ui.hover));
        }
        let Some(color) = blocks::mark_color(span.state, ui) else { continue };
        // Inset where the block starts or ends in view, so neighbours don't run into each other.
        let inset_top = if span.starts { px(blocks::MARK_INSET) } else { Pixels::ZERO };
        let inset_bottom = if span.rows.end < rows { px(blocks::MARK_INSET) } else { Pixels::ZERO };
        let mark = Bounds::new(
            point(left + px(blocks::MARK_X), top + inset_top),
            size(px(blocks::MARK_W), height - inset_top - inset_bottom),
        );
        window.paint_quad(fill(mark, color).corner_radii(px(blocks::MARK_W / 2.0)));
    }
}

impl TerminalView {
    /// How far the grid stays from the pane's right edge: the padding, and the minimap when it is
    /// shown or the scrollbar's gutter (the legacy style; the overlay one takes no room). The
    /// find bar lines up with it.
    pub fn right_inset(cx: &App) -> f32 {
        PADDING_X + if settings(cx).minimap { minimap::MINIMAP_W } else { ScrollerPrefs::gutter(cx) }
    }

    /// The bar along the bottom of a pane whose shell has ended: what ended and how, a
    /// new shell in the same tab, or closing the tab.
    fn render_exited(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = cx.global::<SettingsStore>().ui;
        let what = match self.exit_code {
            Some(code) => format!("{} exited with code {code}", self.shell),
            None => format!("{} exited", self.shell),
        };
        div()
            .id("exited-bar")
            .absolute()
            .left_0()
            .right_0()
            .bottom_0()
            .h(px(crate::style::TITLE_ROW_H))
            .flex()
            .items_center()
            .gap_2()
            .pl_4()
            .pr_2()
            .bg(ui.panel)
            .rule_top(ui)
            .cursor(CursorStyle::Arrow)
            // The pane below it would start a selection.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().text_color(ui.muted).child(crate::style::icon(IconName::Terminal)))
            .child(div().flex_1().min_w_0().truncate().text_caption().text_color(ui.muted).child(what))
            .child(
                Button::new("exited-restart")
                    .small()
                    .outline()
                    .icon(IconName::RefreshCw)
                    .label("Restart")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(TerminalEvent::Restart))),
            )
            .child(
                div()
                    .id("exited-close")
                    .flex()
                    .items_center()
                    .gap_1()
                    .h(px(crate::style::CONTROL_H))
                    .px_2()
                    .rounded(ui.radius)
                    .text_caption()
                    .text_color(ui.muted)
                    .cursor_pointer()
                    .hover(move |d| d.bg(ui.hover).text_color(ui.text))
                    .child("Close terminal")
                    .child(crate::style::keycap(ui, keymap::glyphs(keymap::ActionId::CloseTerminal)))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(TerminalEvent::Close))),
            )
            .into_any_element()
    }
}

/// Find's ticks: one per match (one per pixel row, however many matches share it), along the
/// pane's right edge, the grid's height standing for the whole scrollback.
fn paint_ticks(ticks: &[f32], grid: Bounds<Pixels>, color: Hsla, window: &mut Window) {
    let x = grid.right() + px(PADDING_X - TICK_INSET - TICK_W);
    let height = f32::from(grid.size.height) - TICK_H;
    let mut last = None;
    for at in ticks {
        let y = (at * height).round();
        if last == Some(y) {
            continue;
        }
        last = Some(y);
        let tick = Bounds::new(point(x, grid.top() + px(y)), size(px(TICK_W), px(TICK_H)));
        window.paint_quad(fill(tick, color).corner_radii(px(TICK_H / 2.0)));
    }
}

fn paint_underlines(paints: &[UnderlinePaint], color: Hsla, window: &mut Window) {
    for paint in paints {
        match *paint {
            UnderlinePaint::Line { origin, width, wavy } => {
                let style = UnderlineStyle { thickness: px(underline::THICKNESS), color: Some(color), wavy };
                window.paint_underline(origin, width, &style);
            }
            UnderlinePaint::Quad(bounds) => window.paint_quad(fill(bounds, color)),
        }
    }
}

/// The visual bell's glow along the pane's edge.
const BELL_EDGE_W: f32 = 2.0;

fn bell_edge(ui: crate::settings::UiColors, color: Hsla) -> Div {
    let mut edge = div().absolute().inset_0().rounded(ui.radius_lg).border_color(color);
    let width = Some(px(BELL_EDGE_W).into());
    let b = &mut edge.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (width, width, width, width);
    edge
}

/// While a selection is dragged, follow the pointer outside the pane too (the
/// pane's own move handler only runs while it is hovered).
fn drag_outside_listener(entity: WeakEntity<TerminalView>, window: &mut Window) {
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
        if phase != DispatchPhase::Bubble {
            return;
        }
        let _ = entity.update(cx, |this, cx| this.drag_selection_outside(event, cx));
    });
}
