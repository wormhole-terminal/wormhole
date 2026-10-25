//! A group's head in the sidebar, as RF-11 draws it: the name in 13 semibold alone on the line (a
//! dot of the group's colour before it where there is no card to wash), the pin after it, and the
//! chevron at the right in a 20 box. Folded, the head also keeps its terminals' tiles, their count
//! and a dot while one needs input or failed; a pinned group with nothing in it is one line with
//! "No terminals" and a + instead, and any other head shows a + while the pointer is over it. A
//! group with a folder shows it under the name in the caption role (after the name in Dense), when
//! `Metrics::group_folders` is on. The label opens the group's menu, the chevron folds it.

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Selectable, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::Density;

use super::buttons::{icon_button, row_actions};
use super::controls::disclosure;
use super::labels::count_badge;
use super::metrics::Metrics;
use super::scale::{MARK_DOT, TextRole};
use super::section::ClickHandler;
use crate::settings::UiColors;

/// The parts of a [`group_head`].
pub struct GroupHead {
    pub metrics: Metrics,
    pub name: SharedString,
    /// Shown when the pointer rests on the label: the group's folder, if it has one.
    pub tooltip: Option<SharedString>,
    /// The group's folder as its line reads (`~/Projects/app`, cut from the front to fit); shown
    /// under the name when `metrics.group_folders` is on, after it in Dense.
    pub folder: Option<SharedString>,
    /// The group's colour: the dot before the name where the group has no card.
    pub color: Option<Hsla>,
    pub folded: bool,
    pub pinned: bool,
    /// Folded: its terminals' tiles (`style::status_tile` at `metrics.folded_tile`), and what they
    /// number.
    pub tiles: Vec<AnyElement>,
    pub count: usize,
    /// A terminal in it needs input (amber) or failed (danger): the dot after the tiles.
    pub alert: Option<Hsla>,
    /// A pinned group with no terminal: its + (tooltip, what it does), always shown.
    pub empty: Option<(SharedString, ClickHandler)>,
    /// The + of any other group (tooltip, what it does), shown while the pointer is over the head.
    pub add: Option<(SharedString, ClickHandler)>,
    /// What the chevron does (fold or open); the head's own click does the same.
    pub on_toggle: ClickHandler,
    /// What VoiceOver says of the label (it opens the menu) and of the chevron (it folds).
    pub spoken_edit: SharedString,
    pub spoken_fold: SharedString,
    /// What a click on the label does (opens the group's popover); without it the label is the head's.
    pub on_label: Option<ClickHandler>,
    /// Measures the label, which the popover's arrow points at.
    pub label_anchor: Option<crate::platform::Anchor>,
    /// The field that stands where the name is while the group is renamed in place.
    pub name_edit: Option<AnyElement>,
}

/// An element that can be a `Popover`'s trigger: the popover marks it selected while it is open.
#[derive(IntoElement)]
pub struct PopoverTrigger {
    content: Stateful<Div>,
    open: bool,
}

impl PopoverTrigger {
    pub fn new(content: Stateful<Div>) -> Self {
        Self { content, open: false }
    }
}

impl Selectable for PopoverTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for PopoverTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.content
    }
}

/// The chevron's box, and the room round the pin.
const CHEVRON_BOX: f32 = 20.0;
const PIN_SIZE: f32 = 12.0;
/// The colour dot of a group without a card.
const DOT: f32 = 8.0;
/// The head's hover group, for the + that shows under the pointer.
const HEAD_GROUP: &str = "group-head";

/// See [`GroupHead`]. `id` names the head; add the drag, the drop and the context menu to it.
#[track_caller]
pub fn group_head(id: impl Into<ElementId>, ui: UiColors, parts: GroupHead) -> Stateful<Div> {
    let GroupHead {
        metrics,
        name,
        tooltip,
        folder,
        color,
        folded,
        pinned,
        tiles,
        count,
        alert,
        empty,
        add,
        on_toggle,
        spoken_edit,
        spoken_fold,
        on_label,
        label_anchor,
        name_edit,
    } = parts;
    let id: ElementId = id.into();
    let is_empty = empty.is_some();
    let editing = name_edit.is_some();
    let folder = folder.filter(|_| metrics.group_folders);
    let dense = metrics.density == Density::Dense;
    let two_lines = folder.is_some() && !dense;
    // Secondary text, as a row's second line; it gives way before the name does.
    let folder_line =
        |line: SharedString| div().min_w_0().truncate().text_caption().text_color(ui.muted).child(line);
    let name_line = div()
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        // The field takes the line, as a terminal's does in its row.
        .when(editing, |d| d.flex_1())
        .map(|d| match name_edit {
            Some(edit) => d.child(div().min_w_0().flex_1().child(edit)),
            None => d.child(div().min_w_0().truncate().text_label().text_color(ui.text).child(name)),
        })
        .when_some(folder.clone().filter(|_| dense), |d, line| d.child(folder_line(line)))
        .when(pinned, |d| {
            d.child(
                div()
                    .flex_none()
                    .text_color(ui.faint)
                    .child(Icon::new(IconName::Pin).with_size(px(PIN_SIZE))),
            )
        });
    let label = div()
        .id((id.clone(), "label"))
        .debug_selector(|| "group-label".to_string())
        .role(Role::Button)
        .aria_label(spoken_edit)
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        .h_full()
        // The field takes the line, as a terminal's does in its row.
        .when(editing, |d| d.flex_1())
        .when_some(label_anchor, |d, anchor| d.relative().child(anchor.probe()))
        .cursor_pointer()
        .when_some(on_label.filter(|_| !editing), |d, on_label| {
            d.on_click(move |event, window, cx| {
                // The label's click is the menu's; the head's own (fold) is for the rest of it.
                cx.stop_propagation();
                on_label(event, window, cx);
            })
        })
        .child(if two_lines {
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .when(editing, |d| d.flex_1())
                .child(name_line)
                .children(folder.map(folder_line))
                .into_any_element()
        } else {
            name_line.into_any_element()
        })
        .when_some(tooltip, |d, tooltip| {
            d.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        });
    let tiles = folded.then(|| {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(5.0))
            .children(tiles)
            .child(count_badge(ui, super::scale::count_label(count)))
            .children(alert.map(|color| div().flex_none().size(px(MARK_DOT + 1.0)).rounded_full().bg(color)))
    });
    let trailing = match empty {
        Some((tooltip, on_add)) => div()
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .child(div().text_caption().text_color(ui.faint).child("No terminals"))
            .child(icon_button((id.clone(), "add"), ui, IconName::Plus, tooltip).in_row().on_click(
                move |event, window, cx| {
                    cx.stop_propagation();
                    on_add(event, window, cx);
                },
            ))
            .into_any_element(),
        None => div()
            .id((id.clone(), "chevron"))
            .role(Role::Button)
            .aria_label(spoken_fold)
            .aria_expanded(!folded)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(CHEVRON_BOX))
            .rounded(ui.radius_sm)
            .cursor_pointer()
            .hover(move |d| d.bg(ui.hover))
            .child(disclosure(ui, folded))
            .into_any_element(),
    };
    // At the end of the line, collapsed to no width at rest (`row_actions`), so the name keeps its
    // place and only gives way, as a terminal's does for its ×, when the + shows.
    let hover_add = add.filter(|_| !is_empty).map(|(tooltip, on_add)| {
        row_actions(
            (id.clone(), "hover-actions"),
            HEAD_GROUP,
            false,
            vec![
                icon_button((id.clone(), "hover-add"), ui, IconName::Plus, tooltip)
                    .in_row()
                    .on_click(move |event, window, cx| {
                        cx.stop_propagation();
                        on_add(event, window, cx);
                    })
                    .into_any_element(),
            ],
        )
    });
    // A folder's line makes the head taller, an empty pinned group's too.
    let height = match (two_lines, is_empty) {
        (true, _) => metrics.head_two_line_h,
        (false, true) => metrics.empty_h,
        (false, false) => metrics.head_h,
    };
    let head = div()
        .id(id)
        .group(HEAD_GROUP)
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(height))
        .px(px(metrics.row_px))
        .rounded(px(metrics.row_radius))
        .cursor_pointer()
        .when(!metrics.cards, |d| {
            d.when_some(color, |d, color| d.child(div().flex_none().size(px(DOT)).rounded_full().bg(color)))
        })
        .child(label)
        .child(div().flex_1())
        .children(tiles)
        .children(hover_add)
        .child(trailing);
    if is_empty { head } else { head.on_click(move |event, window, cx| on_toggle(event, window, cx)) }
}
