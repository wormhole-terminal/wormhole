//! Dragging a terminal: by its sidebar row or by its pane's header. One payload for both, so
//! a row can land on a pane's edge, a header in the sidebar, and each on the other of its kind. The
//! window keeps what the pointer is over while the drag is in flight (`TerminalDrag`): the row and
//! the part of it the sidebar draws its line or tint for (`sidebar/drag.rs`), and the pane edge the
//! panes' overlay draws its zone for (`pane_drop.rs`). GPUI keeps the payload itself out of reach,
//! so what is read while drawing is this.

use gpui_kit::*;
use wormhole_core::view::panes::{PaneId, Zone};
use wormhole_core::view::sidebar::Part;

use super::motion::Lift;
use crate::settings::ui;
use crate::style;
use crate::style::StatusGlyph;

/// Where a drag was picked up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DragOrigin {
    /// A terminal's row in the sidebar.
    Row,
    /// A pane's header over the terminal.
    PaneHeader,
}

/// The payload and the picture of a dragged terminal: its status glyph and name, beside the pointer.
#[derive(Clone)]
pub(crate) struct DraggedTerminal {
    pub(crate) id: i64,
    pub(crate) origin: DragOrigin,
    pub(crate) name: SharedString,
    pub(crate) glyph: StatusGlyph,
    /// Where in the row or header it was picked up. GPUI draws the picture with its corner where
    /// the source's was, so it moves by this much to come out beside the pointer.
    pub(crate) grabbed: Point<Pixels>,
    /// The row itself as the picture, when the Motion feature is on (`motion::lift`); else the chip.
    pub(crate) lift: Option<Lift>,
    /// The window it was picked up in. GPUI keeps one drag for the whole app and every window draws its
    /// picture at its own idea of where the pointer is, so only this window draws it.
    pub(crate) source: Option<WindowId>,
}

impl DraggedTerminal {
    pub(crate) fn new(id: i64, origin: DragOrigin, name: SharedString, glyph: StatusGlyph) -> Self {
        Self { id, origin, name, glyph, grabbed: Point::default(), lift: None, source: None }
    }
}

impl DraggedTerminal {
    /// Whether window `window` draws the picture: only the one the drag was picked up in does.
    pub(crate) fn drawn_in(&self, window: WindowId) -> bool {
        self.source.is_none_or(|source| source == window)
    }
}

impl Render for DraggedTerminal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.drawn_in(window.window_handle().window_id()) {
            return div().into_any_element();
        }
        let ui = ui(cx);
        let glyph = style::status_glyph(("drag-glyph", self.id as u64), ui, self.glyph, cx);
        let chip = style::drag_ghost(ui, Some(glyph), self.name.clone());
        if let Some(lift) = &self.lift {
            return lift.picture(self.id, self.grabbed, style::drag_lift(chip, ui, 1.0), window, cx);
        }
        let left = f32::from(self.grabbed.x) + style::GHOST_POINTER_GAP;
        let top = (f32::from(self.grabbed.y) - style::GHOST_POINTER_RISE).max(0.0);
        div().pl(px(left)).pt(px(top)).child(chip).into_any_element()
    }
}

/// The side of a pane the pointer is over while a terminal is dragged over the panes, where
/// dropping it would work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PaneHover {
    pub(crate) target: PaneId,
    pub(crate) zone: Zone,
}

/// What the pointer is over in the sidebar while something is dragged there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Over {
    /// A terminal's row, and the part of it.
    Row { id: i64, part: Part },
    /// A group's head, and the part of it.
    Head { group: u32, part: Part },
    /// The gap under a group's card (a terminal in no group has none: the bottom of its row).
    After(u32),
    /// The space under the last line.
    Tail,
}

/// What a drag in flight is over.
pub(crate) struct TerminalDrag {
    pub(crate) id: i64,
    pub(crate) origin: DragOrigin,
    /// The pane edge the pointer is over.
    pub(crate) pane: Option<PaneHover>,
}

impl TerminalDrag {
    pub(crate) fn new(id: i64, origin: DragOrigin) -> Self {
        Self { id, origin, pane: None }
    }
}

/// The payload and the picture of a dragged group: its colour and name, beside the pointer.
#[derive(Clone)]
pub(crate) struct DraggedGroup {
    pub(crate) group: u32,
    pub(crate) name: SharedString,
    pub(crate) color: Option<Hsla>,
    /// Where in the head it was picked up (see [`DraggedTerminal::grabbed`]).
    pub(crate) grabbed: Point<Pixels>,
}

impl Render for DraggedGroup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let dot = style::color_dot(ui, self.color.unwrap_or(ui.faint), style::COLOR_DOT_MENU);
        let left = f32::from(self.grabbed.x) + style::GHOST_POINTER_GAP;
        let top = (f32::from(self.grabbed.y) - style::GHOST_POINTER_RISE).max(0.0);
        div().pl(px(left)).pt(px(top)).child(style::drag_ghost(
            ui,
            Some(dot.into_any_element()),
            self.name.clone(),
        ))
    }
}

/// The status glyph a pane's header says its terminal has: the same states as its slot.
pub(crate) fn pane_glyph(status: crate::terminal_view::Status, told: bool) -> StatusGlyph {
    use crate::terminal_view::Status;
    match status {
        Status::Busy if told => StatusGlyph::Working,
        Status::Busy => StatusGlyph::Running,
        Status::Done => StatusGlyph::Done,
        Status::Failed => StatusGlyph::Failed,
        Status::Attention => StatusGlyph::NeedsYou,
        Status::Idle | Status::Exited => StatusGlyph::Terminal { unread: false },
    }
}
