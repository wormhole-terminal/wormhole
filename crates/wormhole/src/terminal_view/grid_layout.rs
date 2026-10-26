//! The geometry of a terminal's last painted frame: where the grid sits and how big its cells are, which the
//! mouse handlers read, and the part of it the window reads through a probe it shares (never the terminal).

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{Pixels, Point, Size};

/// The size of a terminal's grid as of the last frame it painted, which a terminal sets while
/// painting (a change of it is told by `TerminalEvent::Resized`, which the window listens to).
#[derive(Clone)]
pub struct GridProbe(pub(super) Rc<Cell<Layout>>);

impl GridProbe {
    /// Columns and rows.
    pub fn size(&self) -> (usize, usize) {
        let layout = self.0.get();
        (layout.cols, layout.rows)
    }

    /// What the pane has beyond the grid's whole cells, across and down, in points.
    pub fn spare(&self) -> (f32, f32) {
        self.0.get().spare
    }

    /// A cell's width and height in points: what a window that resizes in whole cells steps by.
    pub fn cell(&self) -> (f32, f32) {
        let layout = self.0.get();
        (f32::from(layout.cell.width), f32::from(layout.cell.height))
    }
}
/// Geometry of the last painted frame, shared with mouse handlers.
#[derive(Clone, Copy, Default)]
pub(super) struct Layout {
    pub(super) origin: Point<Pixels>,
    pub(super) cell: Size<Pixels>,
    pub(super) cols: usize,
    pub(super) rows: usize,
    /// How far the grid's left edge is from the pane's: the padding.
    pub(super) gutter: f32,
    /// What the window has around the grid (the content less the grid's area), for window operations.
    pub(super) frame: (f32, f32),
    /// What the pane has beyond the grid's whole cells, across and down: the strip a window that is off the
    /// cell grid leaves at its right and bottom edges.
    pub(super) spare: (f32, f32),
}
