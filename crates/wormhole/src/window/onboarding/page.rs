//! The intro window's page: a layer over the whole window, opaque, holding the welcome page or a
//! step's card, with the replay button in the corner. Each part comes in as the animation allows
//! (`wormhole_core::onboarding::intro::page`): fading in and rising into place.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::onboarding::Step;
use wormhole_core::onboarding::intro::page::Entrance;

use super::FirstRun;
use crate::settings::{UiColors, ui};
use crate::style;
use crate::window::aux_window;

/// How far a part rises as it comes in, points.
const RISE: f32 = 16.0;
/// The glow along the page's top: how far down it reaches (a share of the window) and how strong it
/// starts.
const GLOW_HEIGHT: f32 = 0.6;
const GLOW_ALPHA: f32 = 0.06;

/// A part of the page coming in: `progress` of the way from faint and low to whole and in place.
pub(super) trait Entering: Styled + Sized {
    fn entering(self, progress: f32) -> Self {
        self.relative().top(px(RISE * (1.0 - progress))).opacity(progress)
    }
}

impl<T: Styled> Entering for T {}

impl Render for FirstRun {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_intro(window, cx).unwrap_or_else(|| div().into_any_element())
    }
}

impl FirstRun {
    /// The intro window over the window: the page, its drag strip along the top and the replay
    /// button in the corner. `None` when it isn't open.
    fn render_intro(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let intro = self.intro.as_ref()?;
        let ui = ui(cx);
        let entrance = self.intro_entrance(cx);
        // While the animation plays the page is drawn every frame, to come in with it.
        if crate::intro::progress(cx).is_some() {
            window.request_animation_frame();
        }
        let body = match intro.model.step() {
            Step::Welcome => self.render_welcome(intro, ui, entrance, cx),
            _ => self.render_sheet(intro, window, cx).into_any_element(),
        };
        // The steps' card comes in as one piece; the welcome page by its parts.
        let body = if intro.model.step() == Step::Welcome {
            body
        } else {
            div().child(body).entering(entrance.body).into_any_element()
        };
        Some(
            div()
                .id("intro-window")
                .key_context(super::keys::CONTEXT)
                .on_action(cx.listener(|this, _: &super::keys::NextButton, window, cx| {
                    this.tab_within_intro(false, window, cx)
                }))
                .on_action(cx.listener(|this, _: &super::keys::PreviousButton, window, cx| {
                    this.tab_within_intro(true, window, cx)
                }))
                // A dialog VoiceOver names, holding the keyboard while it is up.
                .role(Role::Dialog)
                .aria_label("Welcome to Wormhole")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .flex_col()
                .bg(aux_window::surfaces(cx, false).background)
                .text_color(ui.text)
                .occlude()
                .track_focus(&intro.focus)
                // A click while the page is still coming in cuts the animation short.
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        if this.intro_is_entering(cx) {
                            crate::intro::skip(cx);
                            cx.stop_propagation();
                        }
                    }),
                )
                .on_key_down(cx.listener(Self::intro_escape))
                // First, so it is behind what is drawn over it (`chrome::TitleDrag`).
                .child(aux_window::drag_strip())
                // The light of the wormhole, faint, falling from the top: it comes in with the logo.
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w_full()
                        .h(relative(GLOW_HEIGHT))
                        .bg(linear_gradient(
                            180.0,
                            linear_color_stop(ui.primary.opacity(GLOW_ALPHA), 0.0),
                            linear_color_stop(ui.primary.opacity(0.0), 1.0),
                        ))
                        .opacity(entrance.logo),
                )
                .child(
                    div()
                        .id("intro-body")
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .items_center()
                        .pt(px(style::TITLE_ROW_H))
                        .pb_6()
                        .overflow_y_scroll()
                        // Centered by its margins: where it doesn't fit it starts at the top and
                        // scrolls, which centering by `justify_center` would clip on both ends.
                        .child(div().my_auto().child(body)),
                )
                // Last in the tree, so ⇥ reaches the page's buttons first and this one after them.
                .child(self.render_replay_button(ui, entrance, cx))
                .into_any_element(),
        )
    }

    /// The replay button in the top right corner: the animation again, and the welcome page with it.
    fn render_replay_button(&self, ui: UiColors, entrance: Entrance, cx: &mut Context<Self>) -> AnyElement {
        let keys = "R";
        div()
            .absolute()
            .top(px((style::TITLE_ROW_H - style::HIT_MIN) / 2.0))
            .right(px(style::HEADER_PAD_END))
            .child(
                style::icon_button("intro-replay", ui, IconName::RotateCcw, format!("Replay intro ({keys})"))
                    .in_title_row()
                    .on_click(cx.listener(|this, _, window, cx| this.replay_intro(window, cx))),
            )
            .opacity(entrance.corner)
            .into_any_element()
    }
}
