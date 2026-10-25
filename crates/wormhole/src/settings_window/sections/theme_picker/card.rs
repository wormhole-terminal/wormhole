//! One card of the theme picker: a miniature of the app in a theme's own colors with its name
//! in a strip under it. A card stands for one theme; a click makes it the pick of its kind.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::{Settings, ThemeMode};
use wormhole_core::theme::catalog::ThemeEntry;
use wormhole_core::theme::tokens::{Appearance, ThemeTokens};

use super::card_height;
use crate::settings::UiColors;
use crate::settings::update_settings;
use crate::style::{Edges, TextRole};
use crate::theme_bridge::{hsla, token};

// The preview is a drawing at its own small scale, not a control: its rules and bars do not
// follow the app's metrics (the card shows the previewed theme's colors).
const PREVIEW_RULE: f32 = 1.0;
const PREVIEW_BAR_H: f32 = 3.0;
const PREVIEW_PAD: f32 = 8.0;
const PREVIEW_GAP: f32 = 5.0;
/// The name strip under the miniature.
const STRIP_H: f32 = 28.0;
/// The ring round the card that is its shelf's pick, outside its edge.
const RING_WIDTH: f32 = 2.0;

/// One theme's card.
pub(crate) struct ThemeCard {
    pub(super) catalog: Rc<Vec<ThemeEntry>>,
    /// The theme, by index into the catalog; it is also the card's element id.
    pub(super) entry: usize,
    /// What the strip says: the family's name when the shelf holds one version of it, else
    /// the version's own.
    pub(super) label: SharedString,
    /// It is the pick of its kind.
    pub(super) picked: bool,
    pub(super) ui: UiColors,
}

// How many times cards were drawn, per thread (tests run in parallel).
#[cfg(test)]
thread_local! {
    pub(super) static CARD_RENDERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Set `value`, a theme of the `appearance` kind, as the pick the mode says a card sets: under
/// Match system, the pick of the theme's own kind.
fn choose(s: &mut Settings, value: String, appearance: Appearance) {
    let light = match s.theme_mode {
        ThemeMode::Dark => false,
        ThemeMode::Light => true,
        ThemeMode::System => appearance == Appearance::Light,
    };
    if light { s.theme_light = value } else { s.theme = value }
}

impl Render for ThemeCard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        CARD_RENDERS.with(|n| n.set(n.get() + 1));
        let entry = &self.catalog[self.entry];
        let (ui, picked) = (self.ui, self.picked);
        let tokens = &entry.tokens;
        let t = |name: &str| token(tokens, name);
        let (value, appearance) = (entry.value.clone(), tokens.appearance);
        let ring = UiColors { border_width: px(RING_WIDTH), ..ui };
        let name_color = if picked { t("primary") } else { t("text.primary") };
        div()
            .id(("theme-card", self.entry))
            .w_full()
            .h(card_height(px(0.0)))
            .flex()
            .flex_col()
            .rounded(ui.radius_lg)
            .overflow_hidden()
            .cursor_pointer()
            .map(|d| if picked { d.outlined(ring, ui.primary) } else { d.outlined(ui, ui.border) })
            .when(!picked, |d| d.hover(|d| d.border_color(ui.border_strong)))
            .on_click(cx.listener(move |_, _, _, cx| {
                let value = value.clone();
                update_settings(cx, |s| choose(s, value, appearance));
            }))
            .child(preview(tokens))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .h(px(STRIP_H))
                    .flex_none()
                    .px_3()
                    .bg(solid(tokens, "surface.panel"))
                    .rule_top(UiColors { border: t("border"), ..ui })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_label()
                            .font_weight(if picked { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .text_color(name_color)
                            .child(self.label.clone()),
                    )
                    .when(picked, |d| {
                        d.child(Icon::new(IconName::Check).xsmall().text_color(t("primary")).flex_none())
                    }),
            )
    }
}

/// A token of the card's theme on the theme's own window color (surfaces may carry alpha for
/// a translucent window; a card is opaque).
fn solid(tokens: &ThemeTokens, name: &str) -> Hsla {
    match (tokens.get(name), tokens.get("surface.window").map(|c| c.with_alpha(1.0))) {
        (Some(color), Some(window)) => hsla(color.over(window)),
        _ => token(tokens, name),
    }
}

/// A miniature of the app in `tokens`: the sidebar with three rows, the first one selected, and
/// the terminal with lines in the theme's colors.
fn preview(tokens: &ThemeTokens) -> AnyElement {
    let t = |name: &str| token(tokens, name);
    let ansi = |i: usize| hsla(tokens.terminal.ansi[i]);
    let bar =
        |color: Hsla, width: f32| div().h(px(PREVIEW_BAR_H)).w(relative(width)).rounded_full().bg(color);
    let sidebar = div()
        .w(relative(0.28))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(PREVIEW_GAP))
        .p(px(PREVIEW_PAD))
        .bg(solid(tokens, "sidebar.background"))
        .border_r(px(PREVIEW_RULE))
        .border_color(t("border"))
        .child(div().h(px(PREVIEW_BAR_H * 2.0)).w(relative(0.9)).rounded_full().bg(t("sidebar.row.selected")))
        .child(bar(t("text.muted"), 0.6))
        .child(bar(t("text.muted"), 0.75));
    let terminal = div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(PREVIEW_GAP))
        .p(px(PREVIEW_PAD))
        .bg(hsla(tokens.terminal.background))
        .child(bar(t("primary"), 0.55))
        .child(bar(ansi(2), 0.8))
        .child(bar(ansi(3), 0.45))
        .child(bar(hsla(tokens.terminal.foreground), 0.7))
        .child(bar(ansi(1), 0.35));
    div().flex_1().min_h_0().w_full().flex().child(sidebar).child(terminal).into_any_element()
}
