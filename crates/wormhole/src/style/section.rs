//! The items of a right-panel section: the flush row that tasks and notes share under
//! their `style::band`, its label, editor and hover actions, and whether a section has
//! anything to fold.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::{icon_button, row_actions};
use super::reorder::{self, Reorder};
use super::rows::lead;
use super::scale::{DRAG_LIFT_OPACITY, ITEM_LEAD_W, PANEL_INSET, ROW_H, TextRole};
use crate::settings::UiColors;

/// A click handler that can be stored in a component's optional slot.
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Whether a collapsible section has anything to fold.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// Nothing inside: no chevron and nothing to click, so it can't be left folded and hidden.
    Empty,
    Collapsed,
    Open,
}

impl Section {
    /// `folded` is the saved choice; a section with no `items` is [`Section::Empty`]
    /// whatever it says, so emptying a folded section shows it open again.
    pub fn of(items: usize, folded: bool) -> Self {
        match (items, folded) {
            (0, _) => Self::Empty,
            (_, true) => Self::Collapsed,
            (_, false) => Self::Open,
        }
    }

    /// Whether the section's rows are hidden.
    pub fn is_collapsed(self) -> bool {
        self == Self::Collapsed
    }
}

/// How far GPUI Kit's `small` input and textarea inset their text. An item's editor is
/// pulled out by this much ([`item_editor`]), so text doesn't move when editing starts.
const EDITOR_INSET: f32 = 8.0;
/// Height of an item's main line: that of GPUI Kit's `small` input, so a row keeps its
/// height when its label turns into an editor.
const ITEM_LINE_H: f32 = 24.0;
/// Group of an [`item_row`], for its hover-only actions and grip.
const ITEM_GROUP: &str = "section-item";
/// Where a flush item's body starts so its text lines up with the main line at x 40: the
/// inset, the lead column and its gap, less the editor's own inset.
const FLUSH_BODY_INDENT: f32 = PANEL_INSET + ITEM_LEAD_W + 8.0 - EDITOR_INSET;
/// The drag grip of a flush item, in its left inset while the item is hovered.
const GRIP_W: f32 = 12.0;
const GRIP_INSET: f32 = 2.0;

/// What one item of a panel section is made of. Every kind of item (tasks, notes) is
/// the same row: only what goes in the slots differs.
pub struct ItemParts {
    /// The leading slot ([`ITEM_LEAD_W`]): a task's checkbox, a note's chevron.
    pub lead: AnyElement,
    /// The main line: an [`item_label`], or an [`item_editor`] while it is being edited.
    pub main: AnyElement,
    /// Shown while the item is hovered or edited, at the right ([`item_action`]). Every kind
    /// lists the same actions in the same order.
    pub actions: Vec<AnyElement>,
    /// Below the main line, lined up with it: a note's text.
    pub body: Option<AnyElement>,
    /// Being edited: the hover fill and the actions stay on, so it's clear where typing goes.
    pub active: bool,
    /// Drag the main line to move the item within its list.
    pub reorder: Option<Reorder>,
}

/// One task or note of a right-panel section: a square row
/// that runs edge to edge, [`ROW_H`] tall, with the lead (a checkbox, a chevron) in the
/// lead column at x 16 and the main line at x 40. The actions show at the right while the
/// item is hovered (the main line then truncates before them), and a draggable item shows
/// its grip in the left inset. It sits under a `style::band`.
#[track_caller]
pub fn item_row(id: impl Into<ElementId>, ui: UiColors, parts: ItemParts) -> Div {
    let id = id.into();
    let draggable = parts.reorder.is_some();
    let dragged = parts.reorder.as_ref().is_some_and(|reorder| reorder.dragged);
    let active = parts.active;
    let line = div()
        .id(id.clone())
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .min_h(px(ROW_H))
        .text_body()
        .child(lead(parts.lead))
        .child(
            // One line that truncates, as every row of a list does; the note's text
            // goes in the body below.
            div()
                .flex_1()
                .min_w_0()
                .min_h(px(ITEM_LINE_H))
                .flex()
                .flex_col()
                .justify_center()
                .truncate()
                .child(parts.main),
        )
        .when(!parts.actions.is_empty(), |d| {
            d.child(row_actions((id.clone(), "actions"), ITEM_GROUP, active, parts.actions))
        });
    let (line, drop) = match parts.reorder {
        Some(reorder) => {
            let (line, drop) = reorder::draggable(line, id, ui, reorder);
            (line, Some(drop))
        }
        None => (line, None),
    };
    div()
        .relative()
        .group(ITEM_GROUP)
        .flex()
        .flex_col()
        .w_full()
        .hover(|d| d.bg(ui.hover))
        .when(active, |d| d.bg(ui.hover))
        .when(dragged, |d| d.opacity(DRAG_LIFT_OPACITY))
        .when(draggable, |d| {
            d.child(
                div()
                    .absolute()
                    .left(px(GRIP_INSET))
                    .top_0()
                    .h(px(ROW_H))
                    .w(px(GRIP_W))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(ui.faint)
                    .invisible()
                    .group_hover(ITEM_GROUP, |d| d.visible())
                    .child(Icon::new(IconName::GripVertical).xsmall()),
            )
        })
        .child(line)
        .children(parts.body.map(|body| div().pl(px(FLUSH_BODY_INDENT)).pr_4().pb_2().child(body)))
        .children(drop)
}

/// The main line of an [`item_row`] when it isn't being edited: body text, `quiet` (muted)
/// for an item that is done or whose content shows below it anyway.
#[track_caller]
pub fn item_label(ui: UiColors, text: impl Into<SharedString>, quiet: bool) -> Div {
    div().text_body().text_color(if quiet { ui.muted } else { ui.text }).child(text.into())
}

/// An item's editor (GPUI Kit `small` input, no appearance) in place of its label, pulled
/// out by the editor's inset so the text stays where the label drew it.
#[track_caller]
pub fn item_editor(editor: impl IntoElement) -> Div {
    div().ml(px(-EDITOR_INSET)).child(editor)
}

/// A hover action of an [`item_row`]: a row-sized icon button (`IconButton::in_row`) with a
/// tooltip. The click doesn't reach the row (which starts editing or opens the item).
pub fn item_action(
    id: impl Into<ElementId>,
    ui: UiColors,
    icon: IconName,
    tooltip: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    icon_button(id, ui, icon, tooltip)
        .in_row()
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            on_click(event, window, cx);
        })
        .into_any_element()
}
