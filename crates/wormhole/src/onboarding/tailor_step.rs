//! "How much to show", the Look step's walk through what the window displays: one question at a time,
//! its answers drawn as the window each would give, the one in use ringed, Back and Start over, and a
//! summary at the end (core's `onboarding::tailor` decides it; this draws it and applies it).
//!
//! Every answer is applied as it is given: the settings it changes are written at once, and the one
//! part that is a window's own, the right panel, is asked of the window with
//! [`FirstRunEvent::PanelOpen`] (the first run never reaches into the `Workspace`).

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::presets::{Shown, offered_tools};
use wormhole_core::onboarding::Step;
use wormhole_core::onboarding::tailor::{At, Step as Asked, Walk};
use wormhole_text::keys::{
    TAILOR_ALL_SET, TAILOR_BACK, TAILOR_HEADING, TAILOR_LOOKS_GOOD, TAILOR_OWN, TAILOR_START_OVER,
    TAILOR_SUMMARY, tailor_announce, tailor_counter,
};
use wormhole_text::text;

use super::{FirstRun, FirstRunEvent};
use crate::settings::{SettingsStore, UiColors, update_settings};
use crate::style::{self, Edges, TextRole};

/// The strip under an answer's sketch, holding its number and name: two lines of a label fit. It is a
/// fixed height, so that a card is not measured for its text (that cost more than the drawing).
const STRIP_H: f32 = 48.0;
/// Room for the answer's line under the cards, two lines of caption, so the card below doesn't jump
/// between answers.
const DETAIL_H: f32 = 32.0;

/// Where the walk is, and whether its summary was accepted.
#[derive(Clone)]
pub(super) struct Tailor {
    pub(super) walk: Walk,
    /// "Looks good" was pressed: the walk is a line, with Start over.
    settled: bool,
}

/// What the person does in the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TailorAct {
    /// Answer number `n` (from 0) of the question asked.
    Answer(usize),
    Back,
    StartOver,
    /// "Looks good" on the summary.
    Settle,
}

impl FirstRun {
    /// Whether the Look step is the page, which is where the walk is.
    pub(crate) fn on_look_step(&self) -> bool {
        self.intro.as_ref().is_some_and(|intro| intro.model.step() == Step::Look)
    }

    /// Begin the walk if it hasn't begun, from what the settings show now and the window's panel
    /// ([`super::Facts::panel_open`]). Once, not every frame: the window tells the first run about itself
    /// as the page opens, and the walk keeps what it began with to put back on Start over.
    pub(super) fn begin_tailor(&mut self, cx: &mut Context<Self>) {
        let panel_open = self.facts.panel_open;
        let Some(intro) = &mut self.intro else { return };
        if intro.tailor.is_some() {
            return;
        }
        let shown = Shown::read(cx.global::<SettingsStore>().settings_without_zen(), panel_open);
        intro.tailor = Some(Tailor { walk: Walk::new(shown), settled: false });
        cx.notify();
    }

    /// Do `act` in the walk and write what it changed to the settings and the window. Returns whether it
    /// did anything (an answer past the last, Back on the first question, or any act off the Look step,
    /// does nothing).
    pub(crate) fn tailor_act(&mut self, act: TailorAct, cx: &mut Context<Self>) -> bool {
        if !self.on_look_step() {
            return false;
        }
        self.begin_tailor(cx);
        let Some(tailor) = self.intro.as_mut().and_then(|intro| intro.tailor.as_mut()) else { return false };
        let from = tailor.walk.shown.clone();
        let moved = match act {
            TailorAct::Answer(ix) => tailor.walk.answer(ix).is_some(),
            TailorAct::Back => tailor.walk.back(),
            TailorAct::StartOver => {
                tailor.settled = false;
                tailor.walk.start_over();
                true
            }
            TailorAct::Settle => {
                tailor.settled = tailor.walk.at == At::Done;
                tailor.settled
            }
        };
        let shown = tailor.walk.shown.clone();
        let words = announcement(&tailor.walk, tailor.settled);
        if !moved {
            return false;
        }
        // Only what the answer changed is written: the rest of the settings, which the person may have
        // had their own way, is left as it was.
        if shown != from {
            update_settings(cx, |settings| shown.apply_changes(&from, settings));
            if shown.panel_open != from.panel_open {
                cx.emit(FirstRunEvent::PanelOpen(shown.panel_open));
            }
        }
        crate::platform::a11y::announce(&words, false);
        cx.notify();
        true
    }

    /// A digit key on the Look step: answer number `n` (from 1) of the question asked. Returns whether
    /// the key was used.
    pub(crate) fn tailor_key(&mut self, n: usize, cx: &mut Context<Self>) -> bool {
        let asking = self
            .intro
            .as_ref()
            .and_then(|intro| intro.tailor.as_ref())
            .is_some_and(|tailor| matches!(tailor.walk.at, At::Asking(_)));
        asking && n.checked_sub(1).is_some_and(|ix| self.tailor_act(TailorAct::Answer(ix), cx))
    }

    /// The walk, under its heading; nothing until it has begun (the first frame of the page).
    pub(super) fn render_tailor(&self, ui: UiColors, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tailor = self.intro.as_ref()?.tailor.as_ref()?;
        let body = if tailor.settled {
            self.render_tailor_settled(ui, cx)
        } else {
            match tailor.walk.step() {
                Some(step) => self.render_tailor_question(&tailor.walk, step, ui, cx),
                None => self.render_tailor_summary(&tailor.walk, ui, cx),
            }
        };
        Some(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(style::section_label(ui, text(TAILOR_HEADING)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .p_3()
                        .rounded(ui.radius)
                        .outlined(ui, ui.border)
                        .child(body),
                )
                .into_any_element(),
        )
    }

    /// A question: its place, the answers as the window each gives (a click or its number picks it),
    /// and a line about the one in use.
    fn render_tailor_question(
        &self,
        walk: &Walk,
        step: Asked,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let question = step.question;
        let picked = question.picked(&walk.shown);
        let answers = question.answers();
        let offered = offered_tools(&cx.global::<SettingsStore>().settings).len();
        let counter = tailor_counter(&step.number.to_string(), &step.of.to_string());
        let cards = div()
            .id("onboarding-tailor-answers")
            .flex()
            .gap_2()
            .role(Role::Group)
            .aria_label(text(question.title()))
            .children(answers.iter().enumerate().map(|(ix, answer)| {
                let on = picked == Some(ix);
                let label = text(answer.label);
                let sketch = style::window_sketch(ui, &question.preview(ix, &walk.shown), offered);
                style::choice_card(("onboarding-tailor-answer", ix), ui, on)
                    .flex_1()
                    .aria_label(label)
                    .aria_description(text(answer.detail))
                    .child(div().p_1().child(sketch))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_start()
                            .gap_1()
                            .h(px(STRIP_H))
                            .overflow_hidden()
                            .p_1()
                            .rule_top(ui)
                            .text_caption()
                            .font_weight(if on { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .text_color(if on { ui.primary } else { ui.text })
                            .child(style::keycap(ui, (ix + 1).to_string()))
                            .child(div().flex_1().min_w_0().child(label)),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.tailor_act(TailorAct::Answer(ix), cx);
                        this.focus_page(window, cx);
                    }))
            }));
        let detail = picked.map(|ix| text(answers[ix].detail)).unwrap_or_default();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .text_body()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(text(question.title())),
                    )
                    .child(div().flex_none().text_caption().text_color(ui.muted).child(counter)),
            )
            .child(cards)
            .child(div().min_h(px(DETAIL_H)).text_caption().text_color(ui.muted).child(detail))
            .child(self.render_tailor_links(step.number > 1, ui, cx))
            .into_any_element()
    }

    /// The summary: each question with the answer it has now, and the way to accept it.
    fn render_tailor_summary(&self, walk: &Walk, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let rows = walk.summary().into_iter().map(|(question, answer)| {
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_caption()
                        .text_color(ui.muted)
                        .child(text(question.title())),
                )
                .child(match answer {
                    Some(answer) => {
                        div().text_body().font_weight(FontWeight::MEDIUM).child(text(answer.label))
                    }
                    None => div().text_body().text_color(ui.muted).child(text(TAILOR_OWN)),
                })
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child(text(TAILOR_SUMMARY)))
            .child(div().flex().flex_col().gap_1().children(rows))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(self.render_tailor_links(true, ui, cx))
                    .child(
                        Button::new("onboarding-tailor-settle")
                            .primary()
                            .small()
                            .label(text(TAILOR_LOOKS_GOOD))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.tailor_act(TailorAct::Settle, cx);
                                this.focus_page(window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    /// What the walk is once accepted: one line, and the way to begin again.
    fn render_tailor_settled(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_body()
                    .child(style::icon(IconName::Check).text_color(ui.primary))
                    .child(text(TAILOR_ALL_SET)),
            )
            .child(self.start_over_link(ui, cx))
            .into_any_element()
    }

    /// Back (past the first question) and Start over.
    fn render_tailor_links(&self, back: bool, ui: UiColors, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .gap_3()
            .when(back, |d| {
                d.child(style::link_button("onboarding-tailor-back", ui, text(TAILOR_BACK), false).on_click(
                    cx.listener(|this, _, window, cx| {
                        this.tailor_act(TailorAct::Back, cx);
                        this.focus_page(window, cx);
                    }),
                ))
            })
            .child(self.start_over_link(ui, cx))
    }

    fn start_over_link(&self, ui: UiColors, cx: &mut Context<Self>) -> Stateful<Div> {
        style::link_button("onboarding-tailor-start-over", ui, text(TAILOR_START_OVER), false).on_click(
            cx.listener(|this, _, window, cx| {
                this.tailor_act(TailorAct::StartOver, cx);
                this.focus_page(window, cx);
            }),
        )
    }

    /// The keyboard back on the page after a click on one of the walk's parts, so its digits and the
    /// sheet's arrows keep working.
    fn focus_page(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(intro) = &self.intro {
            window.focus(&intro.focus, cx);
        }
    }
}

/// What a screen reader is told as the walk moves: the question and its place, or where the walk ended.
fn announcement(walk: &Walk, settled: bool) -> String {
    match walk.step() {
        Some(step) => {
            tailor_announce(text(step.question.title()), &step.number.to_string(), &step.of.to_string())
        }
        None if settled => text(TAILOR_ALL_SET).to_string(),
        None => text(TAILOR_SUMMARY).to_string(),
    }
}

#[cfg(test)]
impl super::Intro {
    /// The walk, once it has begun.
    pub(crate) fn tailor_walk(&self) -> Option<&Walk> {
        self.tailor.as_ref().map(|tailor| &tailor.walk)
    }
}

#[cfg(test)]
impl FirstRun {
    pub(crate) fn tailor_settled(&self) -> bool {
        self.intro.as_ref().and_then(|intro| intro.tailor.as_ref()).is_some_and(|tailor| tailor.settled)
    }
}
