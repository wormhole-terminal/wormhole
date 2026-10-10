//! Drag-and-drop reordering of the items in a section: the drag payload and its picture,
//! and the overlay that draws where a dragged item would land.

use std::cell::RefCell;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{DROP_LINE_W, Edges, TextRole};
use crate::settings::UiColors;

/// Moves an item from one index of its list to another (see [`Reorder`]).
pub type MoveHandler = std::rc::Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

/// Drag-and-drop reordering of an `item_row` within its list.
pub struct Reorder {
    /// Names the list: an item only drops among items with the same one (so a task
    /// never lands among notes, or in another window's list).
    pub list: SharedString,
    /// The item's place in the list.
    pub ix: usize,
    /// Drawn on the item that follows the pointer.
    pub label: SharedString,
    /// Called on drop with the dragged item's index and the one it moves to (see
    /// `wormhole_core::reorder::move_item`).
    pub on_move: MoveHandler,
    /// This item is the one being dragged ([`Reorder::dragging`]): it stays in its place,
    /// dimmed to `DRAG_LIFT_OPACITY`.
    pub dragged: bool,
}

thread_local! {
    /// The item a drag started from: GPUI keeps the payload of the drag in flight to itself.
    static DRAGGING: RefCell<Option<(SharedString, usize)>> = const { RefCell::new(None) };
}

impl Reorder {
    /// Whether item `ix` of `list` is being dragged right now.
    pub fn dragging(list: &SharedString, ix: usize, cx: &App) -> bool {
        cx.has_active_drag()
            && DRAGGING
                .with(|dragging| dragging.borrow().as_ref().is_some_and(|(l, i)| l == list && *i == ix))
    }
}

/// The payload and the picture of a dragged `item_row`.
#[derive(Clone)]
pub struct DraggedItem {
    list: SharedString,
    ix: usize,
    label: SharedString,
}

impl Render for DraggedItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        drag_ghost(crate::settings::ui(cx), cx.theme().popover, None, self.label.clone())
    }
}

/// How far from the pointer a dragged sidebar terminal's picture sits: to its right,
/// with the pointer about at its middle line.
pub const GHOST_POINTER_GAP: f32 = 12.0;
pub const GHOST_POINTER_RISE: f32 = 14.0;

/// What follows the pointer while an item is dragged: its main line on a floating chip, after
/// `lead` (a sidebar terminal's status glyph; a note has none). `surface` is the
/// popover color (opaque, unlike the panels).
#[track_caller]
pub fn drag_ghost(ui: UiColors, surface: Hsla, lead: Option<AnyElement>, label: SharedString) -> Div {
    div()
        .max_w(px(240.0))
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded(ui.radius)
        .outlined(ui, ui.border)
        .bg(surface)
        .text_body()
        .text_color(ui.text)
        .children(lead)
        .child(div().min_w_0().truncate().child(label))
}

/// Makes `line` (an item's main line) draggable, and returns it with the overlay that
/// draws where a dragged sibling would land; the caller lays the overlay over the item.
#[track_caller]
pub(super) fn draggable(
    line: Stateful<Div>,
    id: ElementId,
    ui: UiColors,
    reorder: Reorder,
) -> (Stateful<Div>, Stateful<Div>) {
    let dragged = DraggedItem { list: reorder.list.clone(), ix: reorder.ix, label: reorder.label.clone() };
    let line = line.on_drag(dragged, |item, _, _, cx| {
        DRAGGING.with(|dragging| *dragging.borrow_mut() = Some((item.list.clone(), item.ix)));
        cx.new(|_| item.clone())
    });
    (line, drop_target(id, ui, reorder))
}

/// Covers an item without taking its clicks, and draws the drop indicator on the edge
/// where a dragged sibling would land.
#[track_caller]
fn drop_target(id: ElementId, ui: UiColors, reorder: Reorder) -> Stateful<Div> {
    use wormhole_core::reorder::{DropEdge, drop_edge};
    let Reorder { list, ix, on_move, .. } = reorder;
    let over = list.clone();
    div()
        .id((id, "drop"))
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .drag_over::<DraggedItem>(move |mut style, dragged, _, _| {
            let line = Some(px(DROP_LINE_W).into());
            match (dragged.list == over).then(|| drop_edge(dragged.ix, ix)).flatten() {
                Some(DropEdge::Above) => style.border_widths.top = line,
                Some(DropEdge::Below) => style.border_widths.bottom = line,
                None => return style,
            }
            style.border_color = Some(ui.primary);
            style
        })
        .on_drop(move |dragged: &DraggedItem, window, cx| {
            if dragged.list == list {
                on_move(dragged.ix, ix, window, cx);
            }
        })
}

/// The row a dragged terminal would make a split with, or a dragged pane would join:
/// the drop tint with a primary outline all round. Lay it over the row.
#[track_caller]
pub fn row_drop_target(ui: UiColors) -> Div {
    div().absolute().top_0().left_0().size_full().bg(ui.highlight).outlined(ui, ui.primary)
}

/// How far a [`drop_line`] that stays inside a split's bracket starts from the row's left edge: at
/// the lead column, so it reads as "among these" and the full-width one as "out of them".
pub const DROP_LINE_INSET: f32 = 16.0;

/// The line that shows where a dragged terminal lands between two rows: [`DROP_LINE_W`] thick in
/// the primary across the whole row, which takes it out of any bracket it is dropped beside, or
/// `inside` a split's bracket inset from the left, which only reorders its rows. It
/// straddles the top edge of the row it is laid over (its bottom edge when `below`).
#[track_caller]
pub fn drop_line(ui: UiColors, inside: bool, below: bool) -> Div {
    let half = px(DROP_LINE_W / 2.0);
    div()
        .absolute()
        .left(px(if inside { DROP_LINE_INSET } else { 0.0 }))
        .right_0()
        .h(px(DROP_LINE_W))
        .bg(ui.primary)
        .map(|d| if below { d.bottom(-half) } else { d.top(-half) })
}
