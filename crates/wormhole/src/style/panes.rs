//! Split panes: a pane's header, which only shows once a tab is split, the divider between
//! two panes, and how much a pane without the keyboard fades.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::panes::{Axis, Zone};

use super::buttons::icon;
use super::scale::{CONTROL_H_SM, DROP_LINE_W, Edges, RESIZE_HANDLE_W, ROW_H, TextRole};
use crate::settings::UiColors;

/// A pane's header is as tall as a one-line row: the status slot, the name and the folder, then
/// the zoom, menu and close buttons at its right end.
pub const PANE_HEADER_H: f32 = ROW_H;
/// The line under the focused pane's header, in the primary.
pub const PANE_FOCUS_LINE: f32 = 2.0;
/// How much of its strength the text of a pane without the keyboard keeps (72 %).
pub const PANE_DIM: f32 = 0.72;
/// The grabber across a divider under the pointer or being dragged: a pill the handle's width.
const GRAB_L: f32 = 32.0;

/// How a [`pane_divider`] looks: a hairline, lit after a short hover, solid while dragged (as
/// the panels' edges).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DividerLook {
    Rest,
    Hot,
    Dragging,
}

/// The header over a pane of a split tab: caption text, semibold and full strength on
/// the focused pane with a [`PANE_FOCUS_LINE`] in the primary under it, medium and muted on the
/// others; a hairline under every header. Add the status slot, the name, the folder and the
/// buttons with `.child(…)`.
#[track_caller]
pub fn pane_header(id: impl Into<ElementId>, ui: UiColors, focused: bool) -> Stateful<Div> {
    div()
        .id(id)
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .h(px(PANE_HEADER_H))
        .pl_3()
        .pr_1()
        .rule_bottom(ui)
        .text_caption()
        .font_weight(if focused { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
        .text_color(if focused { ui.text } else { ui.muted })
        .when(focused, |d| {
            d.child(div().absolute().left_0().right_0().bottom_0().h(px(PANE_FOCUS_LINE)).bg(ui.primary))
        })
}

/// The divider between two panes: a [`RESIZE_HANDLE_W`] handle straddling their shared edge (it
/// sits at the start of the second pane, reaching back over the first) with the hairline along
/// the edge, which takes `resize.handle.hover` under the pointer and `resize.handle.active` while
/// dragged, with a grabber across it. A `Row` split's divider is vertical and drags sideways, a
/// `Column` split's horizontal. Add the hover, press and tooltip handlers.
#[track_caller]
pub fn pane_divider(id: impl Into<ElementId>, ui: UiColors, axis: Axis, look: DividerLook) -> Stateful<Div> {
    let line = match look {
        DividerLook::Rest => ui.border,
        DividerLook::Hot => ui.resize_hover,
        DividerLook::Dragging => ui.resize_active,
    };
    let half = px(RESIZE_HANDLE_W / 2.0);
    let centred = px((RESIZE_HANDLE_W - f32::from(ui.border_width)) / 2.0);
    let handle = div().id(id).absolute();
    let rule = div().absolute().bg(line);
    let grab = div().absolute().rounded(ui.radius_chip).bg(line);
    match axis {
        Axis::Row => handle
            .top_0()
            .bottom_0()
            .left(-half)
            .w(px(RESIZE_HANDLE_W))
            .cursor(CursorStyle::ResizeLeftRight)
            .child(rule.top_0().bottom_0().left(centred).w(ui.border_width))
            .when(look != DividerLook::Rest, |d| {
                d.child(
                    grab.left_0()
                        .w(px(RESIZE_HANDLE_W))
                        .h(px(GRAB_L))
                        .top(relative(0.5))
                        .mt(px(-GRAB_L / 2.0)),
                )
            }),
        Axis::Column => handle
            .left_0()
            .right_0()
            .top(-half)
            .h(px(RESIZE_HANDLE_W))
            .cursor(CursorStyle::ResizeUpDown)
            .child(rule.left_0().right_0().top(centred).h(ui.border_width))
            .when(look != DividerLook::Rest, |d| {
                d.child(
                    grab.top_0()
                        .h(px(RESIZE_HANDLE_W))
                        .w(px(GRAB_L))
                        .left(relative(0.5))
                        .ml(px(-GRAB_L / 2.0)),
                )
            }),
    }
}

/// How a [`pane_drop_zone`] looks while a terminal is dragged over the panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneLook {
    /// A side the pointer isn't on: a dashed outline with a chevron pointing at it.
    Hint,
    /// The side it would land on: the part of the pane the dragged terminal would take, in the
    /// drop tint with the primary's outline.
    Active,
}

/// A place a dragged terminal can land beside a pane: one side of it as a faint dashed
/// outline with a chevron (`Hint`, `border.strong`, `state.hover`), or, under the pointer, the
/// share of the pane the terminal would take, [`DROP_LINE_W`] thick in the primary over the drop
/// tint (`state.highlight`). Place it with `.absolute()` and the rectangle's edges; `zone` is the
/// side of the pane it is.
#[track_caller]
pub fn pane_drop_zone(ui: UiColors, look: ZoneLook, zone: Zone) -> Div {
    let area = div().absolute().flex().items_center().justify_center().rounded(ui.radius_sm);
    match look {
        ZoneLook::Hint => {
            let chevron = match zone {
                Zone::Left => IconName::ChevronLeft,
                Zone::Right => IconName::ChevronRight,
                Zone::Top => IconName::ChevronUp,
                Zone::Bottom => IconName::ChevronDown,
            };
            area.outlined(ui, ui.border_strong)
                .border_dashed()
                .bg(ui.hover)
                .text_color(ui.muted)
                .child(icon(chevron))
        }
        ZoneLook::Active => {
            let mut zone = area.border_color(ui.primary).bg(ui.highlight);
            let line = px(DROP_LINE_W).into();
            let b = &mut zone.style().border_widths;
            (b.top, b.right, b.bottom, b.left) = (Some(line), Some(line), Some(line), Some(line));
            zone
        }
    }
}

/// What a drag over a drop zone says it will do ("Show on the right"): a small primary pill on
/// the zone, labelling each gesture.
#[track_caller]
pub fn drop_hint(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(px(CONTROL_H_SM))
        .px_2()
        .rounded(ui.radius_chip)
        .bg(ui.primary)
        .text_color(ui.on_primary)
        .text_caption()
        .font_weight(FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .child(text.into())
}
