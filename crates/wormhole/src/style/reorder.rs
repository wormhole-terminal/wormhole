//! Drag-and-drop reordering of the items in a section: the drag payload and its picture,
//! and the overlay that draws where a dragged item would land; and the row of a list a person
//! arranges in Settings (the title row's parts), which moves the same way.

use std::cell::RefCell;

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::icon;
use super::scale::{CONTROL_H, DRAG_LIFT_OPACITY, DROP_LINE_W, Edges, ITEM_LEAD_W, TextRole};
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
    /// `wormhole_core::view::reorder::move_item`).
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
        drag_ghost(crate::settings::ui(cx), None, self.label.clone())
    }
}

/// How far from the pointer a dragged sidebar terminal's picture sits: to its right,
/// with the pointer about at its middle line.
pub const GHOST_POINTER_GAP: f32 = 12.0;
pub const GHOST_POINTER_RISE: f32 = 14.0;

/// What follows the pointer while an item is dragged: its main line on a floating chip, after
/// `lead` (a sidebar terminal's status glyph; a note has none), on the elevated surface (opaque, unlike
/// the panels).
#[track_caller]
pub fn drag_ghost(ui: UiColors, lead: Option<AnyElement>, label: SharedString) -> Div {
    div()
        .max_w(px(240.0))
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded(ui.radius)
        .outlined(ui, ui.border)
        .bg(ui.elevated)
        .text_body()
        .text_color(ui.text)
        .children(lead)
        .child(div().min_w_0().truncate().child(label))
}

/// How a dragged picture's shadow falls: lifted off the list, as a popover floats over a window.
const LIFT_SHADOW_Y: f32 = 8.0;
const LIFT_SHADOW_BLUR: f32 = 20.0;

/// `ghost` (a [`drag_ghost`]) lifted off the list `lift` (0.0 flat, 1.0 all the way) of the way: the
/// shadow of something that floats, growing with it. Only the shadow: a div has no scale in GPUI, so a
/// lift is all the picture can show.
#[track_caller]
pub fn drag_lift(ghost: Div, ui: UiColors, lift: f32) -> Div {
    ghost.shadow(vec![BoxShadow {
        color: ui.shadow.opacity(lift.clamp(0.0, 1.0)),
        offset: point(px(0.), px(LIFT_SHADOW_Y * lift.clamp(0.0, 1.0))),
        blur_radius: px(LIFT_SHADOW_BLUR),
        spread_radius: px(0.),
        inset: false,
    }])
}

/// The mark on the row a dragged terminal would make a split with: two columns, in the primary, at the
/// right end of the row. Lay it over the row with [`row_drop_target`].
#[track_caller]
pub fn split_mark(ui: UiColors) -> Div {
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .right(px(SPLIT_MARK_RIGHT))
        .flex()
        .items_center()
        .text_color(ui.primary)
        .child(super::buttons::icon(gpui_kit::assets::IconName::Columns2))
}

/// How far from the row's right edge the split mark stands: clear of the counts and glyphs at its end.
const SPLIT_MARK_RIGHT: f32 = 44.0;

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
    use wormhole_core::view::reorder::{DropEdge, drop_edge};
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

/// What one row of a list the person arranges in Settings is made of (see [`arrange_row`]).
pub struct ArrangeParts {
    pub icon: IconName,
    pub label: SharedString,
    /// After the label, quieter: what the row means (the flexible space's rule).
    pub note: Option<SharedString>,
    /// Left out (a part the title row doesn't show): its icon and name are faint.
    pub quiet: bool,
    /// At the row's right end: its switch.
    pub control: Option<AnyElement>,
    /// Drag the row to move it within its list; `None` for one that stays where it is.
    pub reorder: Option<Reorder>,
}

/// A row of a list the person arranges in Settings, [`CONTROL_H`] tall: a grip while it can be
/// dragged (the drop line shows where it lands), its icon and name, a note, and its switch at the
/// right end. A row that can't move keeps the grip's room, so the names line up.
#[track_caller]
pub fn arrange_row(id: impl Into<ElementId>, ui: UiColors, parts: ArrangeParts) -> Div {
    let id = id.into();
    let dragged = parts.reorder.as_ref().is_some_and(|reorder| reorder.dragged);
    let grip = div()
        .flex_none()
        .w(px(ITEM_LEAD_W))
        .flex()
        .justify_center()
        .text_color(ui.faint)
        .when(parts.reorder.is_some(), |d| d.child(Icon::new(IconName::GripVertical).xsmall()));
    let line =
        div()
            .id(id.clone())
            .flex()
            .items_center()
            .gap_2()
            .h(px(CONTROL_H))
            .pr_1()
            .rounded(ui.radius)
            .text_body()
            .when(parts.reorder.is_some(), |d| d.cursor_grab())
            .child(grip)
            .child(
                div()
                    .flex_none()
                    .text_color(if parts.quiet { ui.faint } else { ui.muted })
                    .child(icon(parts.icon)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .text_color(if parts.quiet { ui.muted } else { ui.text })
                            .child(parts.label),
                    )
                    .children(parts.note.map(|note| {
                        div().min_w_0().truncate().text_caption().text_color(ui.muted).child(note)
                    })),
            )
            .children(parts.control);
    let (line, drop) = match parts.reorder {
        Some(reorder) => {
            let (line, drop) = draggable(line, id, ui, reorder);
            (line, Some(drop))
        }
        None => (line, None),
    };
    div()
        .relative()
        .w_full()
        .rounded(ui.radius)
        .hover(|d| d.bg(ui.hover))
        .when(dragged, |d| d.opacity(DRAG_LIFT_OPACITY))
        .child(line)
        .children(drop)
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
