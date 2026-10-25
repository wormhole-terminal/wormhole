//! One card of the theme picker: a miniature of the app in a theme's own colors with its name
//! in a strip under it, and for a family of versions a swatch per version.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::{Settings, ThemeMode};
use wormhole_core::theme::catalog::ThemeEntry;
use wormhole_core::theme::tokens::{Appearance, ThemeTokens};

use super::{CardLook, card_height};
use crate::settings::UiColors;
use crate::settings::update_settings;
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge::{hsla, token};

// The preview is a drawing at its own small scale, not a control: its rules and bars do not
// follow the app's metrics (the card shows the previewed theme's colors).
const PREVIEW_RULE: f32 = 1.0;
const PREVIEW_BAR_H: f32 = 3.0;
const PREVIEW_PAD: f32 = 8.0;
const PREVIEW_GAP: f32 = 5.0;
/// The name strip under the miniature.
const STRIP_H: f32 = 28.0;
/// The ring round the card in use, outside its edge.
const RING_WIDTH: f32 = 2.0;
/// One theme's card, or a family's, drawn from the catalog entries it stands for.
pub(crate) struct ThemeCard {
    pub(super) catalog: Rc<Vec<ThemeEntry>>,
    /// Its place among the cards, for its element ids.
    pub(super) card: usize,
    /// Its themes, by index into the catalog: one, or a family's versions.
    pub(super) members: Rc<[usize]>,
    pub(super) look: CardLook,
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
        let entry = &self.catalog[self.look.shown];
        let (ui, look) = (self.ui, self.look);
        let tokens = &entry.tokens;
        let t = |name: &str| token(tokens, name);
        let (value, appearance) = (entry.value.clone(), tokens.appearance);
        let family = self.members.len() > 1;
        // Under Match system a family's card sets both picks: the version of each kind it draws,
        // else the family's first of that kind.
        let of_kind = |kind: Appearance| {
            let fits = |ix: &usize| self.catalog[*ix].tokens.appearance == kind;
            let ix = Some(self.look.shown).filter(fits).or_else(|| self.members.iter().copied().find(fits));
            ix.map(|ix| self.catalog[ix].value.clone())
        };
        let (dark, light) = (of_kind(Appearance::Dark), of_kind(Appearance::Light));
        let ring = UiColors { border_width: px(RING_WIDTH), ..ui };
        let name_color = if look.active { t("primary") } else { t("text.primary") };
        let versions = family.then(|| self.versions(cx));
        div()
            .id(("theme-card", self.card))
            .w_full()
            .h(card_height(px(0.0)))
            .flex()
            .flex_col()
            .rounded(ui.radius_lg)
            .overflow_hidden()
            .cursor_pointer()
            .group("theme-card")
            .map(|d| if look.active { d.outlined(ring, ui.primary) } else { d.outlined(ui, ui.border) })
            .when(!look.active, |d| d.hover(|d| d.border_color(ui.border_strong)))
            .on_click(cx.listener(move |_, _, _, cx| {
                let (value, dark, light) = (value.clone(), dark.clone(), light.clone());
                update_settings(cx, |s| match s.theme_mode {
                    ThemeMode::System if family => {
                        if let Some(dark) = dark {
                            s.theme = dark;
                        }
                        if let Some(light) = light {
                            s.theme_light = light;
                        }
                    }
                    _ => choose(s, value, appearance),
                });
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
                            .font_weight(if look.active { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .text_color(name_color)
                            .child(entry.card_label().to_string()),
                    )
                    // A family's swatches say which of its versions are picked; a theme's marks do.
                    .children(versions)
                    .when(!family && look.dark_pick, |d| d.child(mark(IconName::Moon, t("text.muted"))))
                    .when(!family && look.light_pick, |d| d.child(mark(IconName::Sun, t("text.muted"))))
                    .when(look.active, |d| d.child(mark(IconName::Check, t("primary")))),
            )
    }
}

impl ThemeCard {
    /// A family's versions as swatches of their backgrounds, each named in a tooltip: the ones
    /// in use or picked are ringed, and a click picks one.
    fn versions(&self, cx: &mut Context<Self>) -> AnyElement {
        let shown = &self.catalog[self.look.shown].tokens;
        // Drawn on the card's strip, in the colors of the version it draws.
        let ui = UiColors { text: token(shown, "text.primary"), border: token(shown, "border"), ..self.ui };
        let picked = self.look.picked;
        div()
            .flex()
            .flex_none()
            .items_center()
            .children(self.members.iter().enumerate().map(|(place, &ix)| {
                let entry = &self.catalog[ix];
                let name: SharedString =
                    entry.family.as_ref().map_or_else(|| entry.label.clone(), |(_, v)| v.clone()).into();
                let (value, appearance) = (entry.value.clone(), entry.tokens.appearance);
                let ring = place < 32 && picked & (1 << place) != 0;
                style::swatch_small(("theme-version", ix), ui, hsla(entry.tokens.terminal.background), ring)
                    .tooltip(move |window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(name.clone()).build(window, cx)
                    })
                    .on_click(cx.listener(move |_, _, _, cx| {
                        // This version, not the family the card around it would set.
                        cx.stop_propagation();
                        let value = value.clone();
                        update_settings(cx, |s| choose(s, value, appearance));
                    }))
            }))
            .into_any_element()
    }
}

fn mark(icon: IconName, color: Hsla) -> Icon {
    Icon::new(icon).xsmall().text_color(color).flex_none()
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
