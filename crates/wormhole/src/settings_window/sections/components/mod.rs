//! Settings → Components: every control and block the Settings pages are built from, drawn in one place
//! with sample state, so how they look can be tuned without hunting through the pages. A development tool
//! behind the `components` flag; it is a page of specimens and holds no setting.
//!
//! Each specimen is the real component, called the way the pages call it, under a caption that names the
//! function. Its sample state (a switch that flips, a stepper that counts, a field that takes text) lives in
//! this page's [`Samples`] and nowhere else: nothing here is written to `config.json`. The state lasts while
//! the page is in front, and starts again when it is opened. What the Theme Studio's gallery already draws
//! with nothing of the Studio in it (the nav, the banners, the shortcut recorder, the profile marks) is
//! called from there, not drawn a second time.
//!
//! By file: this one is the page, its state and what the specimens share; `rows` the setting row in each of
//! its states, with the controls a row holds; `fields` the text fields; `buttons` buttons and links;
//! `marks` badges, chips, tiles and logos; `surfaces` banners, cards, the nav and the preview's frame.

use std::sync::Arc;

use gpui_kit::component::input::{InputState, TextareaState};
use gpui_kit::component::slider::{SliderEvent, SliderState, SliderValue};
use gpui_kit::*;
use wormhole_core::theme::tokens::ThemeTokens;

use crate::settings::{SettingsStore, UiColors};
use crate::settings_window::ui;
use crate::style::{ResetArrow, RowKeys, SettingParts, SettingState, TextRole};

// The specimens carry the marker that says their words are samples on the line of the word (`// text:`), which
// rustfmt would move when it breaks a call across lines; their layout is kept as written.
#[rustfmt::skip]
mod buttons;
#[rustfmt::skip]
mod fields;
#[rustfmt::skip]
mod marks;
#[rustfmt::skip]
mod rows;
#[rustfmt::skip]
mod surfaces;

/// The page, for `cx`'s window: its specimens in order, grouped by what they are.
pub(in crate::settings_window) fn draw(cx: &App) -> AnyElement {
    Page { ui: ui(cx), tokens: cx.global::<SettingsStore>().tokens.clone() }.into_any_element()
}

#[derive(IntoElement)]
struct Page {
    ui: UiColors,
    tokens: Arc<ThemeTokens>,
}

impl RenderOnce for Page {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Kept for as long as the page is drawn in consecutive frames; a click on a specimen notifies it,
        // which draws the window again (`use_keyed_state` observes it for the view it is drawn in).
        let state = window.use_keyed_state("components-samples", cx, Samples::new);
        let sheet = Sheet { ui: self.ui, tokens: self.tokens, now: state.read(cx).now.clone(), state };
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(rows::draw(&sheet, cx))
            .child(fields::draw(&sheet, cx))
            .child(buttons::draw(&sheet, cx))
            .child(marks::draw(&sheet, cx))
            .child(surfaces::draw(&sheet, cx))
    }
}

/// What a specimen draws with: the colours, the tokens, the sample state as of this frame and the entity
/// that changes it.
struct Sheet {
    ui: UiColors,
    tokens: Arc<ThemeTokens>,
    now: Now,
    state: Entity<Samples>,
}

/// The values the specimens show. Cheap to copy: numbers, and handles to the fields' and the slider's state.
#[derive(Clone)]
struct Now {
    switch_on: bool,
    switch_off: bool,
    checks: [bool; 3],
    segment: usize,
    segment_icons: usize,
    segment_many: usize,
    panel_segment: usize,
    size: u32,
    slider_value: f32,
    select_open: bool,
    select_value: String,
    swatch: usize,
    card: usize,
    folded: bool,
    choice: usize,
    menu_search: Entity<InputState>,
    hint: Entity<InputState>,
    filled: Entity<InputState>,
    focused: Entity<InputState>,
    search: Entity<InputState>,
    prefixed: Entity<InputState>,
    letter: Entity<InputState>,
    lines: Entity<TextareaState>,
    slider: Entity<SliderState>,
}

/// The stepper's default: its row shows the changed dot and a reset arrow while the value differs.
const SIZE_DEFAULT: u32 = 14;

/// The sample state of the page and what keeps it up to date.
struct Samples {
    now: Now,
    _subscriptions: Vec<Subscription>,
}

impl Samples {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let slider = cx.new(|_| SliderState::new().min(30.0).max(100.0).step(1.0).default_value(80.0));
        let empty = |window: &mut Window, cx: &mut Context<Self>| cx.new(|cx| InputState::new(window, cx));
        let hint = cx.new(|cx| InputState::new(window, cx).placeholder("Search themes")); // text: sample (a field's hint)
        let filled = cx.new(|cx| InputState::new(window, cx).default_value("Tokyo Night")); // text: sample (a field's text)
        let prefixed = cx.new(|cx| InputState::new(window, cx).placeholder("Filter fonts")); // text: sample (a field's hint)
        let search = cx.new(|cx| InputState::new(window, cx).default_value("line")); // text: sample (a search's text)
        let letter = cx.new(|cx| InputState::new(window, cx).default_value("W")); // text: sample (a field's text)
        let now = Now {
            switch_on: true,
            switch_off: false,
            checks: [false, true, false],
            segment: 0,
            segment_icons: 2,
            segment_many: 1,
            panel_segment: 0,
            size: SIZE_DEFAULT,
            slider_value: 80.0,
            select_open: false,
            select_value: "Tokyo Night".into(), // text: sample (a select's value)
            swatch: 0,
            card: 1,
            folded: false,
            choice: 0,
            menu_search: empty(window, cx),
            focused: empty(window, cx),
            lines: cx.new(|cx| TextareaState::new(window, cx)),
            hint,
            filled,
            search,
            prefixed,
            letter,
            slider: slider.clone(),
        };
        let subscriptions = vec![cx.subscribe(&slider, |this, _, event: &SliderEvent, cx| {
            let SliderEvent::Change(value) = event else { return };
            this.now.slider_value = match value {
                SliderValue::Single(value) => *value,
                SliderValue::Range(_, end) => *end,
            }
            .round();
            cx.notify();
        })];
        Self { now, _subscriptions: subscriptions }
    }
}

/// A specimen under the name of the function that draws it, in the page's quiet type.
#[track_caller]
fn specimen(ui: UiColors, name: &'static str, content: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_caption().text_color(ui.faint).child(name))
        .child(content)
}

/// A category: its head (the settings pages' own group head, with what the category holds at its right) and
/// its specimens.
#[track_caller]
fn category(
    ui: UiColors,
    title: &'static str,
    aside: &'static str,
    specimens: impl IntoIterator<Item = AnyElement>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .items_end()
                .justify_between()
                .gap_4()
                .child(div().text_label().text_color(ui.muted).child(title))
                .child(div().min_w_0().truncate().text_caption().text_color(ui.faint).child(aside)),
        )
        .children(specimens)
}

/// A row's parts with everything at its plainest: a label, the (i)'s description and one control.
fn parts(
    label: &'static str,
    description: &'static str,
    state: SettingState,
    control: AnyElement,
) -> SettingParts {
    SettingParts {
        label: label.into(),
        description: Some(description.into()),
        visible_hint: false,
        chip: None,
        keys: RowKeys::NONE,
        state,
        found: None,
        found_description: None,
        control,
        tall: false,
        extra: None,
        on_reset: None,
        reset_arrow: ResetArrow::OnHover,
    }
}
