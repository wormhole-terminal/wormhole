//! The settings form: the [`setting`] row with its states, a group's head, and the
//! controls that more than one page needs (a stepper, a slider with its value, a link, the
//! shortcut recorder). The settings window builds every row of its pages from these.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::switch::Switch;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_text::{keys, text};

use std::ops::Range;

use super::buttons::icon_button;
use super::controls::keycap;
use super::info::{marked, setting_info};
use super::labels::{changed_dot, tint_chip};
use super::row_keys::{KeyView, Reset, RowKeys, key_label, with_menu};
use super::scale::{
    CONTROL_H, Edges, HIT_MIN, MARK_DOT, NAME_LINE, RECORDER_W, SETTING_ROW_MIN_H, SLIDER_W, TextRole,
};
use super::section::ClickHandler;
use super::text_field::{FLAT_RING_ALPHA, flat_ring};
use crate::settings::UiColors;

/// A small whole number with a minus and a plus, as System Settings draws a stepper: the value as
/// plain text, and beside it one compact control of two halves ([`STEPPER_HALF_W`] wide, as tall as a
/// select) on the `control.background` fill, a hair of the card between them and no outline
/// (Increase Contrast adds one). Add nothing: `minus` and `plus` are the click handlers. `id` names the
/// two buttons.
#[track_caller]
pub fn stepper(
    ui: UiColors,
    id: &'static str,
    value: String,
    minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (round, flat) = (ui.radius, px(0.0));
    let value = SharedString::from(value);
    div()
        .id((ElementId::from(id), "stepper"))
        .role(Role::Group)
        .aria_value(value.clone())
        .flex()
        .flex_none()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(STEPPER_VALUE_W))
                .flex()
                .justify_end()
                .text_label()
                .font_weight(FontWeight::NORMAL)
                .tabular_nums()
                .text_color(ui.text)
                .child(value),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .gap(px(STEPPER_SPLIT))
                .child(stepper_half(
                    ui,
                    (id, 0),
                    IconName::Minus,
                    text(keys::SETTINGS_WINDOW_DECREASE),
                    [round, flat, flat, round],
                    minus,
                ))
                .child(stepper_half(
                    ui,
                    (id, 1),
                    IconName::Plus,
                    text(keys::SETTINGS_WINDOW_INCREASE),
                    [flat, round, round, flat],
                    plus,
                )),
        )
}

/// One half of a [`stepper`]: a filled box that fills a little more under the pointer, with the Kit's
/// button in it for the focus, the keyboard and VoiceOver. Only its outer corners are round, as
/// `[top left, top right, bottom right, bottom left]` says.
#[track_caller]
fn stepper_half(
    ui: UiColors,
    id: (&'static str, usize),
    icon: IconName,
    spoken: &'static str,
    corners: [Pixels; 4],
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let [tl, tr, br, bl] = corners;
    div()
        .id((id.0, id.1 + 2))
        .flex_none()
        .w(px(STEPPER_HALF_W))
        .h(px(CONTROL_H))
        .corners(tl, tr, br, bl)
        .bg(ui.control)
        .hover(move |d| d.bg(ui.control_hover))
        .when(ui.increase_contrast, |d| d.outlined_always(ui, ui.border_strong))
        .child(
            Button::new(id)
                .ghost()
                .small()
                .icon(icon)
                .accessibility_label(spoken)
                .w_full()
                .h_full()
                .corners(tl, tr, br, bl)
                .on_click(on_click),
        )
}

/// Width of the value before a [`stepper`]'s halves, so "10000" and "80 %" don't move them.
const STEPPER_VALUE_W: f32 = 56.0;
/// A [`stepper`] half's width, and the gap between the two halves that shows the card through.
const STEPPER_HALF_W: f32 = 28.0;
const STEPPER_SPLIT: f32 = 1.0;

/// A switch. `on_change` gets the new state.
pub fn toggle(
    id: impl Into<ElementId>,
    checked: bool,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Switch {
    Switch::new(id)
        .checked(checked)
        .on_click(move |checked: &bool, window, cx| on_change(*checked, window, cx))
}

/// What a [`setting`] row says about its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingState {
    /// At its default.
    Default,
    /// Changed from its default: a dot before the label; under the pointer the config key
    /// after it and a reset arrow before the control.
    Changed,
    /// A project sets it while one of its terminals is in front: the project's chip after
    /// the label, and the control (showing the project's value) muted and out of reach.
    Project(SharedString),
    /// Not available here (no layout.json in this project): faint, and the control off.
    Disabled,
}

/// When a changed [`setting`] row shows its reset arrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetArrow {
    /// Under the pointer, before the control: on a setting's own page, where most rows are at their
    /// defaults and an arrow on each would only be noise.
    OnHover,
    /// Always, at the right end of the row, in a column of its own so the controls line up: on the
    /// Changed page, where every row differs and putting it back is what the page is for.
    Always,
}

/// The parts of a [`setting`] row.
pub struct SettingParts {
    pub label: SharedString,
    /// What the setting does. Searched either way; see [`visible_hint`](Self::visible_hint) for where
    /// it is read.
    pub description: Option<SharedString>,
    /// Show the description under the label, for a warning. Any other description is behind the (i)
    /// after the label, so the row stays one line.
    pub visible_hint: bool,
    /// A tag after the label, in the primary's tint ("Time Sensitive").
    pub chip: Option<SharedString>,
    /// The `config.json` keys the row writes: the first is shown under the pointer, all are in its menu
    /// ([`RowKeys::NONE`]: none, and no menu).
    pub keys: RowKeys,
    pub state: SettingState,
    /// What a search found in the label, marked with `state.highlight`.
    pub found: Option<Range<usize>>,
    /// The same in the description.
    pub found_description: Option<Range<usize>>,
    pub control: AnyElement,
    /// The control runs over more than one line (a few fields, a text box): the label sits
    /// beside its first line rather than in the middle of the row, and the description under it.
    pub tall: bool,
    /// Before the control: a link such as "Edit in project".
    pub extra: Option<AnyElement>,
    /// Puts the default back; the reset arrow shows, and the row's menu offers it, while the value is
    /// changed.
    pub on_reset: Option<Reset>,
    /// Where the reset arrow shows while the value is changed.
    pub reset_arrow: ResetArrow,
}

/// How faint a control is while a project sets its value.
const SET_ELSEWHERE_OPACITY: f32 = 0.55;

/// One setting in the settings window: the label (14 / 500) at the left with, after it, an (i) that
/// opens the description in a card (VoiceOver reads the description from the row), or, for a
/// warning that must not be missed ([`SettingParts::visible_hint`]), the description under the label
/// (12, muted); the control at the right edge of the row, at least
/// [`SETTING_ROW_MIN_H`] tall. It has no line of its own: a [`setting_group`](super::setting_group)
/// puts its rows on one card and draws the dividers between them. A row has no hover fill of its own
/// (the card's rounded edge would show around a square one; the control in it answers the pointer). See [`SettingState`] for what a changed, project-set or disabled
/// value adds.
#[track_caller]
pub fn setting(id: impl Into<ElementId>, ui: UiColors, parts: SettingParts, cx: &App) -> Stateful<Div> {
    let SettingParts {
        label,
        description,
        visible_hint,
        chip,
        keys,
        state,
        found,
        found_description,
        control,
        tall,
        extra,
        on_reset,
        reset_arrow,
    } = parts;
    let id = id.into();
    let group = SharedString::from(format!("setting-{id}"));
    let disabled = state == SettingState::Disabled;
    let changed = state == SettingState::Changed;
    let elsewhere = matches!(state, SettingState::Project(_)) || disabled;
    let (label_color, description_color) = if disabled { (ui.faint, ui.faint) } else { (ui.text, ui.muted) };
    let (code_font, _) = crate::fonts::code_font(cx);
    let info = description.clone().filter(|_| !visible_hint);
    let view = KeyView::read(&id, cx);
    let reset = on_reset.filter(|_| changed);
    let menu_reset = reset.clone();
    let menu_id = id.clone();
    let row = div()
        .id(id.clone())
        .role(Role::Group)
        .aria_label(label.clone())
        .when_some(description.clone(), |d, description| d.aria_description(description))
        .group(group.clone())
        .flex()
        .map(|d| if tall { d.items_start() } else { d.items_center() })
        .gap_4()
        .w_full()
        .min_h(px(SETTING_ROW_MIN_H))
        .py_1()
        .relative()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .min_h(rems(NAME_LINE))
                        // As tall as the control's first line, so the two share a middle.
                        .when(tall, |d| d.min_h(px(CONTROL_H)))
                        .when(changed, |d| d.child(changed_dot(ui)))
                        .child(
                            div()
                                .text_body()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(label_color)
                                .child(marked(ui, label, found)),
                        )
                        .when_some(info, |d, description| {
                            d.child(setting_info(id.clone(), ui, description, found_description.clone()))
                        })
                        .when_some(
                            match &state {
                                SettingState::Project(project) => Some(project.clone()),
                                _ => None,
                            },
                            |d, project| d.child(tint_chip(ui, Some(IconName::Folder), project)),
                        )
                        .children(chip.map(|chip| tint_chip(ui, None, chip)))
                        .children(key_label(&id, ui, view, keys, &group, code_font).filter(|_| !elsewhere)),
                )
                .children(description.filter(|_| visible_hint).map(|text| {
                    div().text_caption().text_color(description_color).child(marked(
                        ui,
                        text,
                        found_description,
                    ))
                })),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap_2()
                .when_some(reset.clone().filter(|_| reset_arrow == ResetArrow::OnHover), |d, reset| {
                    d.child(
                        div().invisible().group_hover(group.clone(), |s| s.visible()).child(
                            icon_button(
                                (id.clone(), "reset"),
                                ui,
                                IconName::RotateCcw,
                                text(keys::SETTINGS_ROW_RESET),
                            )
                            .still()
                            .on_click(move |_, window, cx| reset(window, cx)),
                        ),
                    )
                })
                .children(extra)
                .child(if disabled { out_of_reach((id.clone(), "control"), control) } else { control })
                // The column is there for every row, so a row without an arrow (a project sets its value)
                // keeps its control where the others have theirs.
                .when(reset_arrow == ResetArrow::Always, |d| {
                    d.child(div().flex_none().w(px(HIT_MIN)).children(reset.map(|reset| {
                        icon_button(
                            (id.clone(), "reset"),
                            ui,
                            IconName::RotateCcw,
                            text(keys::SETTINGS_ROW_RESET),
                        )
                        .still()
                        .on_click(move |_, window, cx| reset(window, cx))
                    })))
                }),
        );
    // What a project sets or a layout lacks keeps its row as it was: no key, no menu.
    if elsewhere { row } else { with_menu(row, menu_id, keys, menu_reset) }
}

/// A control shown but not to be used (its value is set elsewhere): faint, and a cover over
/// it takes the clicks.
#[track_caller]
fn out_of_reach(id: impl Into<ElementId>, control: AnyElement) -> AnyElement {
    div()
        .relative()
        .opacity(SET_ELSEWHERE_OPACITY)
        .child(control)
        .child(div().id(id).absolute().top_0().left_0().size_full().occlude())
        .into_any_element()
}

/// A text button in the link color ("Edit in Fonts", "Open Notifications"), with an arrow
/// after it when it takes you to another page. Its text lines up with the edge it is placed
/// at, like any other text: the hover fill reaches [`LINK_PAD`] past it on both sides. Add
/// `.on_click(…)`.
#[track_caller]
pub fn link_button(
    id: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    arrow: bool,
) -> Stateful<Div> {
    let label = label.into();
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label.clone())
        .focusable()
        .tab_stop(true)
        .focus_visible(move |d| d.focus_outline(ui))
        .rounded(ui.radius)
        .group(LINK_GROUP)
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .h(px(CONTROL_H))
        .text_label()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ui.link)
        .cursor_pointer()
        // The fill is laid over nothing: out of the layout, so the text keeps its place.
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(-LINK_PAD))
                .right(px(-LINK_PAD))
                .rounded(ui.radius)
                .group_hover(LINK_GROUP, move |d| d.bg(ui.hover)),
        )
        .child(label)
        .when(arrow, |d| d.child(super::buttons::icon(IconName::ArrowRight)))
}

/// The group a [`link_button`]'s hover fill answers to.
const LINK_GROUP: &str = "link-button";

/// The room between a [`link_button`]'s text and the edge of its hover fill.
const LINK_PAD: f32 = 4.0;

/// A value on a scale: GPUI Kit's slider [`SLIDER_W`] wide (`compact`:
/// [`super::scale::SLIDER_W_COMPACT`]), the value after it in a column of its own, and under
/// the track the marks it runs between ("30  65  100"). `slider` is the Kit's `Slider`.
#[track_caller]
pub fn slider_control(
    ui: UiColors,
    slider: impl IntoElement,
    value: impl Into<SharedString>,
    marks: &[&'static str],
    compact: bool,
) -> Div {
    let width = if compact { super::scale::SLIDER_W_COMPACT } else { SLIDER_W };
    div()
        .flex()
        .flex_none()
        .items_start()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .w(px(width))
                .child(div().h(px(CONTROL_H)).flex().items_center().child(slider))
                .when(!marks.is_empty(), |d| {
                    d.child(
                        div()
                            .flex()
                            .justify_between()
                            .text_caption()
                            .tabular_nums()
                            .text_color(ui.faint)
                            .children(marks.iter().map(|mark| div().child(*mark))),
                    )
                }),
        )
        .child(
            div()
                .w(px(SLIDER_VALUE_W))
                .h(px(CONTROL_H))
                .flex()
                .items_center()
                .justify_end()
                .text_body()
                .tabular_nums()
                .text_color(ui.text)
                .child(value.into()),
        )
}

/// The value after a [`slider_control`]: wide enough for "100 %" and "1.30".
const SLIDER_VALUE_W: f32 = 48.0;

/// What a [`shortcut_recorder`] shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecorderState {
    /// No shortcut: the placeholder and a keyboard glyph.
    Empty,
    /// The shortcut's keys as key caps, and a × that clears it.
    Set(Vec<SharedString>),
    /// Waiting for keys: the soft focus ring, a red dot, the modifiers held so far, "esc".
    Recording(Vec<SharedString>),
    /// The keys are taken by something else: a warning outline and ring (say what takes them under it).
    Conflict(Vec<SharedString>),
}

/// The field that records a global shortcut: [`RECORDER_W`] wide, as tall as a
/// select, the keys drawn as the one key cap. Add `.on_click(…)` to start recording;
/// `on_clear` is the × of a set shortcut.
#[track_caller]
pub fn shortcut_recorder(
    id: impl Into<ElementId>,
    ui: UiColors,
    state: RecorderState,
    on_clear: Option<ClickHandler>,
) -> Stateful<Div> {
    let id = id.into();
    let caps = |keys: Vec<SharedString>| keys.into_iter().map(|key| keycap(ui, key));
    let frame = div()
        .id(id.clone())
        .flex()
        .flex_none()
        .items_center()
        .gap_1()
        .w(px(RECORDER_W))
        .h(px(CONTROL_H))
        .px_2()
        .rounded(ui.radius)
        .bg(ui.control)
        .cursor_pointer()
        // Flat like the text fields: a fill, no outline (Increase Contrast adds one), the soft
        // focus ring while it waits for keys, and a warning outline inside a ring for a conflict.
        .map(|d| match &state {
            RecorderState::Recording(_) if ui.increase_contrast => {
                d.relative().child(flat_ring(ui, ui.focus, 1.0))
            }
            RecorderState::Recording(_) => d.relative().child(flat_ring(ui, ui.focus, FLAT_RING_ALPHA)),
            RecorderState::Conflict(_) if ui.increase_contrast => {
                d.relative().child(flat_ring(ui, ui.warning, 1.0))
            }
            RecorderState::Conflict(_) => {
                d.outlined_always(ui, ui.warning).relative().child(flat_ring(ui, ui.warning, FLAT_RING_ALPHA))
            }
            RecorderState::Empty | RecorderState::Set(_) if ui.increase_contrast => {
                d.outlined_always(ui, ui.border_strong)
            }
            RecorderState::Empty | RecorderState::Set(_) => d,
        })
        .when(!matches!(state, RecorderState::Recording(_)), |d| d.hover(move |d| d.bg(ui.control_hover)))
        .text_label()
        .font_weight(FontWeight::NORMAL);
    match state {
        RecorderState::Empty => frame
            .text_color(ui.faint)
            .child(div().flex_1().child("Record shortcut"))
            .child(super::buttons::icon(IconName::Keyboard)),
        RecorderState::Set(keys) | RecorderState::Conflict(keys) => {
            frame.children(caps(keys)).child(div().flex_1()).child(
                super::buttons::close_x((id, "clear"), ui, super::buttons::SelectedFill::Plain).when_some(
                    on_clear,
                    |x, on_clear| {
                        x.on_click(move |event, window, cx| {
                            // The field underneath would start recording too.
                            cx.stop_propagation();
                            on_clear(event, window, cx);
                        })
                    },
                ),
            )
        }
        RecorderState::Recording(held) => frame
            .text_color(ui.muted)
            .child(div().flex_none().size(px(MARK_DOT)).rounded_full().bg(ui.danger))
            .child(div().flex_1().min_w_0().truncate().child("Press keys…"))
            .children(caps(held))
            .child(div().text_caption().child("esc")),
    }
}
