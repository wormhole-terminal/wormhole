//! The settings form: the [`setting`] row with its states, a group's head, and the
//! controls that more than one page needs (a stepper, a slider with its value, a link, the
//! shortcut recorder). The settings window builds every row of its pages from these.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::switch::Switch;
use gpui_kit::prelude::*;
use gpui_kit::*;

use std::ops::Range;

use super::buttons::icon_button;
use super::controls::keycap;
use super::labels::{changed_dot, tint_chip};
use super::scale::{
    CONTROL_H, Edges, MARK_DOT, NAME_LINE, RECORDER_W, SETTING_ROW_MIN_H, SLIDER_W, TextRole,
};
use super::section::ClickHandler;
use crate::settings::UiColors;

/// A small whole number with a minus and a plus: one outlined control as tall as a
/// select, the value between hairlines in the middle. Add nothing: `minus` and `plus` are
/// the click handlers. `id` names the two buttons.
#[track_caller]
pub fn stepper(
    ui: UiColors,
    id: &'static str,
    value: String,
    minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .h(px(CONTROL_H))
        .rounded(ui.radius)
        .outlined(ui, ui.border_strong)
        .bg(ui.input)
        .child(Button::new((id, 0usize)).ghost().small().icon(IconName::Minus).on_click(minus))
        .child(
            div()
                .w(px(STEPPER_VALUE_W))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .rule_left(ui)
                .rule_right(ui)
                .text_label()
                .font_weight(FontWeight::NORMAL)
                .tabular_nums()
                .text_color(ui.text)
                .child(value),
        )
        .child(Button::new((id, 1usize)).ghost().small().icon(IconName::Plus).on_click(plus))
}

/// Width of the value between a [`stepper`]'s buttons, so "10000" and "80 %" don't
/// move the buttons.
const STEPPER_VALUE_W: f32 = 72.0;

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

/// The parts of a [`setting`] row.
pub struct SettingParts {
    pub label: SharedString,
    pub description: Option<SharedString>,
    /// A tag after the label, in the primary's tint ("Time Sensitive").
    pub chip: Option<SharedString>,
    /// The `config.json` key, shown under the pointer (empty: none).
    pub key: &'static str,
    pub state: SettingState,
    /// What a search found in the label, marked with `state.highlight`.
    pub found: Option<Range<usize>>,
    /// The same in the description.
    pub found_description: Option<Range<usize>>,
    pub control: AnyElement,
    /// Before the control: a link such as "Edit in project".
    pub extra: Option<AnyElement>,
    /// Puts the default back; the reset arrow shows while the value is changed.
    pub on_reset: Option<ClickHandler>,
}

/// How faint a control is while a project sets its value.
const SET_ELSEWHERE_OPACITY: f32 = 0.55;

/// One setting in the settings window: the label (14 / 500) and its
/// description (12, muted) on the left, the control at the right edge of the column, a
/// hairline above and at least [`SETTING_ROW_MIN_H`] tall, no card. Hovering fills it
/// with `state.hover`. See [`SettingState`] for what a changed, project-set or disabled
/// value adds.
#[track_caller]
pub fn setting(id: impl Into<ElementId>, ui: UiColors, parts: SettingParts, cx: &App) -> Stateful<Div> {
    let SettingParts {
        label,
        description,
        chip,
        key,
        state,
        found,
        found_description,
        control,
        extra,
        on_reset,
    } = parts;
    let id = id.into();
    let group = SharedString::from(format!("setting-{id}"));
    let disabled = state == SettingState::Disabled;
    let changed = state == SettingState::Changed;
    let elsewhere = matches!(state, SettingState::Project(_)) || disabled;
    let (label_color, description_color) = if disabled { (ui.faint, ui.faint) } else { (ui.text, ui.muted) };
    let mark = HighlightStyle { background_color: Some(ui.highlight), ..Default::default() };
    let marked = |text: SharedString, found: Option<Range<usize>>| match found {
        Some(found)
            if found.end <= text.len()
                && text.is_char_boundary(found.start)
                && text.is_char_boundary(found.end) =>
        {
            StyledText::new(text).with_highlights([(found, mark)]).into_any_element()
        }
        _ => text.into_any_element(),
    };
    let (code_font, _) = crate::fonts::code_font(cx);
    div()
        .id(id.clone())
        .group(group.clone())
        .flex()
        .items_center()
        .gap_4()
        .w_full()
        .min_h(px(SETTING_ROW_MIN_H))
        .py_2()
        .rule_top(ui)
        .when(!disabled, |d| d.hover(|d| d.bg(ui.hover)))
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
                        .when(changed, |d| d.child(changed_dot(ui)))
                        .child(
                            div()
                                .text_body()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(label_color)
                                .child(marked(label, found)),
                        )
                        .when_some(
                            match &state {
                                SettingState::Project(project) => Some(project.clone()),
                                _ => None,
                            },
                            |d, project| d.child(tint_chip(ui, Some(IconName::Folder), project)),
                        )
                        .children(chip.map(|chip| tint_chip(ui, None, chip)))
                        .when(!key.is_empty() && !elsewhere, |d| {
                            d.child(
                                div()
                                    .text_caption()
                                    .font_family(code_font)
                                    .text_color(ui.faint)
                                    .invisible()
                                    .group_hover(group.clone(), |s| s.visible())
                                    .child(key),
                            )
                        }),
                )
                .children(description.map(|text| {
                    div().text_caption().text_color(description_color).child(marked(text, found_description))
                })),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap_2()
                .when_some(on_reset.filter(|_| changed), |d, on_reset| {
                    d.child(
                        div().invisible().group_hover(group.clone(), |s| s.visible()).child(
                            icon_button((id.clone(), "reset"), ui, IconName::RotateCcw, "Reset to default")
                                .on_click(on_reset),
                        ),
                    )
                })
                .children(extra)
                .child(if elsewhere { out_of_reach((id, "control"), control) } else { control }),
        )
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

/// The head of a group of [`setting`] rows: the group's name as a heading (16 / 600) with a
/// step of space above it, and a quiet note at the right ("Same values as the funnel"). The
/// rows follow with their hairlines; end the group with `rule_bottom`.
#[track_caller]
pub fn setting_group_head(ui: UiColors, title: &str, aside: Option<&str>) -> Div {
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap_4()
        .h(px(GROUP_HEAD_H))
        .pb_2()
        .child(
            div()
                .text_heading()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ui.text)
                .child(title.to_string()),
        )
        .children(aside.map(|aside| {
            div().min_w_0().truncate().text_caption().text_color(ui.faint).child(aside.to_string())
        }))
}

/// A group head's height: its name sits at the bottom, a clear step below the group above and a
/// small one above the first row's hairline.
const GROUP_HEAD_H: f32 = 40.0;

/// A text button in the link color ("Edit in Fonts", "Open Notifications"), with an arrow
/// after it when it takes you to another page. Add `.on_click(…)`.
#[track_caller]
pub fn link_button(
    id: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    arrow: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .h(px(CONTROL_H))
        .px_1()
        .rounded(ui.radius)
        .text_label()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ui.link)
        .cursor_pointer()
        .hover(move |d| d.bg(ui.hover))
        .child(label.into())
        .when(arrow, |d| d.child(super::buttons::icon(IconName::ArrowRight)))
}

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
    /// Waiting for keys: the focus ring, a red dot, the modifiers held so far, "esc".
    Recording(Vec<SharedString>),
    /// The keys are taken by something else: a warning outline (say what takes them under it).
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
    let (outline, ring) = match &state {
        RecorderState::Recording(_) => (ui.focus, true),
        RecorderState::Conflict(_) => (ui.warning, true),
        RecorderState::Empty | RecorderState::Set(_) => (ui.border_strong, false),
    };
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
        .bg(ui.input)
        .cursor_pointer()
        .outlined(if ring { UiColors { border_width: ui.focus_ring_width, ..ui } } else { ui }, outline)
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
