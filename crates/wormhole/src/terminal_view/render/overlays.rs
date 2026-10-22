//! What is drawn on or beside the grid that isn't its cells: the command blocks' tint and gutter
//! marks, find's ticks along the pane's edge, the visual bell's glow, and the listener that keeps a
//! selection drag following the pointer outside the pane.

use gpui_kit::{
    Bounds, DispatchPhase, Div, Hsla, MouseMoveEvent, Pixels, Point, Styled, WeakEntity, Window, div, fill,
    point, px, size,
};
use wormhole_term::ScreenSnapshot;

use crate::terminal_view::{PADDING_X, TerminalView, blocks};

/// Find's match ticks: along the pane's right edge, in its padding, beside the grid.
const TICK_W: f32 = 6.0;
const TICK_H: f32 = 2.0;
const TICK_INSET: f32 = 2.0;

/// The command blocks: the hovered block's tint under its rows, and each block's gutter
/// mark in the pane's left padding, from the block's first row to its last one in view.
pub(super) fn paint_blocks(
    snapshot: &ScreenSnapshot,
    hovered: Option<u64>,
    ui: crate::settings::UiColors,
    pane: Bounds<Pixels>,
    origin: Point<Pixels>,
    line_height: Pixels,
    window: &mut Window,
) {
    let left = pane.left() - px(PADDING_X);
    let rows = snapshot.rows().len();
    for span in snapshot.block_spans() {
        let top = origin.y + line_height * span.rows.start as f32;
        let height = line_height * span.rows.len() as f32;
        if hovered == Some(span.id) {
            let tint = Bounds::new(point(left, top), size(pane.right() - left, height));
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

/// Find's ticks: one per match (one per pixel row, however many matches share it), along the
/// pane's right edge, the grid's height standing for the whole scrollback.
pub(super) fn paint_ticks(ticks: &[f32], grid: Bounds<Pixels>, color: Hsla, window: &mut Window) {
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

/// The visual bell's glow along the pane's edge.
const BELL_EDGE_W: f32 = 2.0;

pub(super) fn bell_edge(ui: crate::settings::UiColors, color: Hsla) -> Div {
    let mut edge = div().absolute().inset_0().rounded(ui.radius_lg).border_color(color);
    let width = Some(px(BELL_EDGE_W).into());
    let b = &mut edge.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (width, width, width, width);
    edge
}

/// While a selection is dragged, follow the pointer outside the pane too (the
/// pane's own move handler only runs while it is hovered).
pub(super) fn drag_outside_listener(entity: WeakEntity<TerminalView>, window: &mut Window) {
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
        if phase != DispatchPhase::Bubble {
            return;
        }
        let _ = entity.update(cx, |this, cx| this.drag_selection_outside(event, cx));
    });
}
