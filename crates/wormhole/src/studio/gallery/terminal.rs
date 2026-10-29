//! The 16 ANSI colors, text, selection and cursor.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens::ThemeTokens;

use super::Env;
use super::view::GalleryView;
use crate::settings::settings;
use crate::studio::draft::ANSI_NAMES;
use crate::studio::ui;
use crate::style::{Edges, TextRole};
use crate::theme_bridge::hsla;

/// A palette slot's square in the strip above the sample.
const SLOT: f32 = 16.0;

pub(super) fn render(
    studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let tokens = env.tokens;
    let mut ansi = div().flex().flex_wrap().gap_1();
    for name in ANSI_NAMES {
        ansi = ansi.child(studio.slot(name, tokens, cx));
    }
    let term = &tokens.terminal;
    let sample = div()
        .p_3()
        .rounded(ui.radius_lg)
        .bg(hsla(term.background))
        .text_color(hsla(term.foreground))
        .font_family(SharedString::from(settings(cx).font_family.clone()))
        .text_body()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .gap_1()
                .child(div().text_color(hsla(term.ansi[2])).child("➜"))
                .child(div().text_color(hsla(term.ansi[6])).child("wormhole"))
                .child(div().text_color(hsla(term.ansi[4])).child("git:(main)"))
                .child("cargo test"), // text: sample (the gallery's specimen)
        )
        .child(div().text_color(hsla(term.ansi[2])).child("test result: ok. 57 passed")) // text: sample (the gallery's specimen)
        .child(div().text_color(hsla(term.ansi[3])).child("warning: unused variable")) // text: sample (the gallery's specimen)
        .child(div().text_color(hsla(term.ansi[1])).child("error[E0425]: cannot find value")) // text: sample (the gallery's specimen)
        .child(
            div()
                .flex()
                .child(div().bg(hsla(term.selection_background)).child("selected text")) // text: sample (the gallery's specimen)
                .child(" and a cursor ") // text: sample (the gallery's specimen)
                .child(div().w(px(9.0)).h(px(18.0)).bg(hsla(term.cursor))),
        );
    div().flex().flex_col().gap_2().child(ansi).child(env.mark("terminal", sample)).into_any_element()
}

impl GalleryView {
    /// A palette slot: its color in a small square, ringed in the primary while it is the
    /// token being edited; a click selects it.
    fn slot(&self, name: &'static str, tokens: &ThemeTokens, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let selected = self.selected.as_deref() == Some(name);
        let color = tokens.get(name).unwrap_or(crate::theme_bridge::MISSING_TOKEN);
        div()
            .id(SharedString::from(format!("swatch-{name}")))
            .size(px(SLOT))
            .rounded(ui.radius_sm)
            .cursor_pointer()
            .map(|d| {
                if selected {
                    d.ringed(ui, ui.primary, ui.border_width * 2.0)
                } else {
                    d.outlined(ui, ui.border)
                }
            })
            .bg(hsla(color))
            .on_click(cx.listener(move |this, _, window, cx| this.select_token(name, window, cx)))
    }
}
