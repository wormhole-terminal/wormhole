//! The settings form: the [`setting`] row with its states, a group's head, and the
//! controls that more than one page needs (a stepper, a slider with its value, a link, the
//! shortcut recorder). The settings window builds every row of its pages from these.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_text::{keys, text};

use std::ops::Range;
use std::rc::Rc;

use super::controls::keycap;
use super::info::{marked, setting_info};
use super::row_keys::{KeyView, Reset, RowKeys, key_face, with_menu};
use super::scale::{CONTROL_H, Edges, MARK_DOT, RECORDER_W, SLIDER_W, TextRole};
use super::section::ClickHandler;
use crate::settings::UiColors;
use crate::ui::{Handler, Look, SettingsRowProps, StepperProps, SwitchControlProps};

/// A small whole number with a minus and a plus: one outlined control as tall as a
/// select, the value between hairlines in the middle. Add nothing: `minus` and `plus` are
/// the click handlers. `id` names the two buttons.
#[track_caller]
pub fn stepper(
    _: UiColors,
    id: &'static str,
    value: String,
    minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> StepperView {
    StepperView { id, value, minus: Rc::new(minus), plus: Rc::new(plus) }
}

/// A [`stepper`], drawn by the design's `Stepper` component.
#[derive(IntoElement)]
pub struct StepperView {
    id: &'static str,
    value: String,
    minus: Handler,
    plus: Handler,
}

impl RenderOnce for StepperView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        StepperProps {
            id: self.id.into(),
            label: wormhole_text::text(wormhole_text::keys::SETTINGS_WINDOW_STEPPER_LABEL).into(),
            smaller: wormhole_text::text(wormhole_text::keys::SETTINGS_WINDOW_STEPPER_SMALLER).into(),
            larger: wormhole_text::text(wormhole_text::keys::SETTINGS_WINDOW_STEPPER_LARGER).into(),
            value: self.value.into(),
            on_smaller: Some(self.minus),
            on_larger: Some(self.plus),
        }
        .render(&Look::of(cx))
    }
}

/// What a switch does with the state it is flipped to.
type OnToggle = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A switch. `on_change` gets the new state. Drawn by the `SwitchControl` component (the design's 34 by 20
/// switch); a click flips it, and so do Space and Return, since GPUI sends a focused element those as clicks.
#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    checked: bool,
    on_change: OnToggle,
    label: Option<SharedString>,
    disabled: bool,
}

pub fn toggle(
    id: impl Into<ElementId>,
    checked: bool,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Toggle {
    Toggle { id: id.into(), checked, on_change: Rc::new(on_change), label: None, disabled: false }
}

impl Toggle {
    /// What VoiceOver calls the switch when its row does not name it.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl gpui_kit::component::Disableable for Toggle {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Toggle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { id, checked, on_change, label, disabled } = self;
        let click: Handler = Rc::new(move |_, window, cx| on_change(!checked, window, cx));
        SwitchControlProps {
            id,
            label: label.unwrap_or_default(),
            on: checked,
            disabled,
            on_click: (!disabled).then_some(click),
        }
        .render(&Look::of(cx))
    }
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
    /// What a search found in the label (each word of the query), marked with `settings.search_mark`.
    pub found: Vec<Range<usize>>,
    /// The same in the description.
    pub found_description: Vec<Range<usize>>,
    pub control: AnyElement,
    /// The control runs over more than one line (a few fields, a text box): the label sits
    /// beside its first line rather than in the middle of the row, and the description under it.
    pub tall: bool,
    /// Before the control: a link such as "Edit in project".
    pub extra: Option<AnyElement>,
    /// Inside the row under the label and hint.
    pub under: Option<AnyElement>,
    /// Puts the default back; the reset arrow shows, and the row's menu offers it, while the value is
    /// changed.
    pub on_reset: Option<Reset>,
}

/// How faint a control is while a project sets its value.
const SET_ELSEWHERE_OPACITY: f32 = 0.55;

/// One setting in the settings window: drawn by the `SettingsRow` component (`ui/settings_row.dc.html`,
/// which is also the design), with this function giving it the row's words, state and behaviour: the
/// label (marked where a search found it) with, after it, an (i) that opens the description in a card
/// (VoiceOver reads the description from the row), or, for a warning that must not be missed
/// ([`SettingParts::visible_hint`]), the description under the label; the control at the right edge. It
/// stands in a [`group_card`](super::group_card), which keeps the first hairline from showing. See
/// [`SettingState`] for what a changed, project-set or disabled value adds.
#[track_caller]
pub fn setting(id: impl Into<ElementId>, ui: UiColors, parts: SettingParts, cx: &App) -> Stateful<Div> {
    setting_in(id, ui, parts, &Look::of(cx), cx)
}

/// [`setting`] in the colours of `look` rather than the app's theme: the Theme Studio's gallery draws the
/// theme being edited.
#[track_caller]
pub fn setting_in(
    id: impl Into<ElementId>,
    ui: UiColors,
    parts: SettingParts,
    look: &Look,
    cx: &App,
) -> Stateful<Div> {
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
        under,
        on_reset,
    } = parts;
    let id = id.into();
    let disabled = state == SettingState::Disabled;
    let changed = state == SettingState::Changed;
    let elsewhere = matches!(state, SettingState::Project(_)) || disabled;
    let info = description.clone().filter(|_| !visible_hint);
    let key = key_face(&id, KeyView::read(&id, cx), keys).filter(|_| !elsewhere);
    let reset = on_reset.filter(|_| changed);
    let menu_reset = reset.clone();
    let menu_id = id.clone();
    let project = match &state {
        SettingState::Project(project) => Some(project.clone()),
        _ => None,
    };
    let row = SettingsRowProps {
        id: id.clone(),
        name: label.clone(),
        description: description.clone(),
        label: marked(ui, label, found),
        tall,
        disabled,
        changed,
        project,
        chip,
        info: info.map(|description| {
            setting_info(id.clone(), ui, description, found_description.clone()).into_any_element()
        }),
        key_text: key.as_ref().map(|key| key.shown.clone()),
        key_name: SharedString::from(key.as_ref().map_or("", |key| key.key)),
        copy_label: text(keys::SETTINGS_ROW_COPY_KEY).into(),
        on_copy: key.as_ref().map(|key| key.copy.clone()),
        dormant: key.as_ref().is_none_or(|key| key.dormant),
        copied: key.as_ref().is_some_and(|key| key.copied),
        hint: description.filter(|_| visible_hint),
        under,
        on_reset: reset.map(|reset| -> Handler { Rc::new(move |_, window, cx| reset(window, cx)) }),
        reset_label: text(keys::SETTINGS_ROW_RESET).into(),
        extra,
        control: if elsewhere { out_of_reach((id, "control"), control) } else { control },
    }
    .render(look);
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
    div()
        .id(id)
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
        .child(label.into())
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
        .map(|d| if ring { d.ringed(ui, outline, ui.focus_ring_width) } else { d.outlined(ui, outline) })
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
