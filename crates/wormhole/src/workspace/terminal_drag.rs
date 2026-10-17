//! Dragging a terminal: by its sidebar row or by its pane's header. One payload for both, so
//! a row can land on a pane's edge, a header in the sidebar, and each on the other of its kind. The
//! window keeps what the pointer is over while the drag is in flight (`TerminalDrag`): the row and
//! the part of it the sidebar draws its line or tint for (`sidebar/drag.rs`), and the pane edge the
//! panes' overlay draws its zone for (`pane_drop.rs`). GPUI keeps the payload itself out of reach,
//! so what is read while drawing is this.

use gpui_kit::component::ActiveTheme as _;
use wormhole_core::panes::{PaneId, Zone};
use wormhole_core::sidebar::Part;

use super::*;
use crate::style::StatusGlyph;

/// Where a drag was picked up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DragOrigin {
    /// A terminal's row in the sidebar.
    Row,
    /// A pane's header over the terminal.
    PaneHeader,
}

/// The payload and the picture of a dragged terminal: its status glyph and name, beside the pointer.
#[derive(Clone)]
pub(super) struct DraggedTerminal {
    pub(super) id: i64,
    pub(super) origin: DragOrigin,
    pub(super) name: SharedString,
    pub(super) glyph: StatusGlyph,
    /// Where in the row or header it was picked up. GPUI draws the picture with its corner where
    /// the source's was, so it moves by this much to come out beside the pointer.
    pub(super) grabbed: Point<Pixels>,
}

impl DraggedTerminal {
    pub(super) fn new(id: i64, origin: DragOrigin, name: SharedString, glyph: StatusGlyph) -> Self {
        Self { id, origin, name, glyph, grabbed: Point::default() }
    }
}

impl Render for DraggedTerminal {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let glyph = style::status_glyph(("drag-glyph", self.id as u64), ui, self.glyph, cx);
        let left = f32::from(self.grabbed.x) + style::GHOST_POINTER_GAP;
        let top = (f32::from(self.grabbed.y) - style::GHOST_POINTER_RISE).max(0.0);
        div().pl(px(left)).pt(px(top)).child(style::drag_ghost(
            ui,
            cx.theme().popover,
            Some(glyph),
            self.name.clone(),
        ))
    }
}

/// The side of a pane the pointer is over while a terminal is dragged over the panes, where
/// dropping it would work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PaneHover {
    pub(super) target: PaneId,
    pub(super) zone: Zone,
}

/// What the pointer is over in the sidebar while something is dragged there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Over {
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
pub(super) struct TerminalDrag {
    pub(super) id: i64,
    pub(super) origin: DragOrigin,
    /// The pane edge the pointer is over.
    pub(super) pane: Option<PaneHover>,
}

impl TerminalDrag {
    pub(super) fn new(id: i64, origin: DragOrigin) -> Self {
        Self { id, origin, pane: None }
    }
}

/// The payload and the picture of a dragged group: its colour and name, beside the pointer.
#[derive(Clone)]
pub(super) struct DraggedGroup {
    pub(super) group: u32,
    pub(super) name: SharedString,
    pub(super) color: Option<Hsla>,
    /// Where in the head it was picked up (see [`DraggedTerminal::grabbed`]).
    pub(super) grabbed: Point<Pixels>,
}

impl Render for DraggedGroup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let dot = style::color_dot(ui, self.color.unwrap_or(ui.faint), style::COLOR_DOT_MENU);
        let left = f32::from(self.grabbed.x) + style::GHOST_POINTER_GAP;
        let top = (f32::from(self.grabbed.y) - style::GHOST_POINTER_RISE).max(0.0);
        div().pl(px(left)).pt(px(top)).child(style::drag_ghost(
            ui,
            cx.theme().popover,
            Some(dot.into_any_element()),
            self.name.clone(),
        ))
    }
}

/// The status glyph a pane's header says its terminal has: the same states as its slot.
pub(super) fn pane_glyph(status: crate::terminal_view::Status, told: bool) -> StatusGlyph {
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
