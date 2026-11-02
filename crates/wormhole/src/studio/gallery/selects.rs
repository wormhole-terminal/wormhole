//! The select: a trigger with chevrons, and the menu it opens (a search row over roomy
//! options on a lifted surface).

use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::select::{self, ColorChoice, Picked, Swatch, Value};
use crate::style::TextRole;
use crate::theme_bridge;
use std::rc::Rc;
use wormhole_core::theme::folder_color::FolderColor;

pub(super) fn render(
    studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let surface = select::menu_bg(ui, cx);
    let menu = div()
        .w(px(260.0))
        .rounded(ui.radius_lg)
        .bg(surface)
        .shadow(select::menu_shadow(ui))
        .child(select::search_row(ui, false, div().text_body().text_color(ui.faint).child("Search themes"))) // text: sample (the gallery's specimen)
        .child(
            div()
                .flex()
                .flex_col()
                .p_1()
                .child(select::option("g-opt-a", ui, true, false).child("Tokyo Night")) // text: sample (the gallery's specimen)
                .child(select::option("g-opt-b", ui, false, false).child("Nord")) // text: sample (the gallery's specimen)
                .child(select::option("g-opt-c", ui, false, false).child("Dracula")), // text: sample (the gallery's specimen)
        );
    div()
        .flex()
        .flex_wrap()
        .gap_4()
        .items_start()
        .child(select::SelectTrigger::new("g-select", ui, "Tokyo Night").width(px(220.0))) // text: sample (the gallery's specimen)
        .child(select::SelectTrigger::new("g-select-compact", ui, "main").compact())
        // The settings rows' control: the same face, its menu built only while open (click it to see it).
        .child(sample_menu_select(ui))
        // The colour control of the primary and branch rows: its card (the theme's colour, eight presets, Custom…
        // with its editor) is built only while open, so click it to see it.
        .child(sample_color_select(ui, env, studio))
        .child(env.mark("menu", menu))
        .into_any_element()
}

/// A settings row's menu control with sample words.
fn sample_menu_select(ui: crate::settings::UiColors) -> impl IntoElement {
    const NAME: &str = "Mode"; // text: sample (the gallery's specimen)
    const OPTIONS: [&str; 3] = ["Match system", "Dark", "Light"]; // text: sample (the gallery's specimen)
    select::menu_select("g-menu-select", ui, NAME, OPTIONS[0], |_| {
        select::MenuOptions::words(OPTIONS.map(Into::into), Some(0), |_, _, _| {})
    })
}

/// The specimen's colour.
pub(super) const SAMPLE_COLOR: &str = "#a35cf2"; // text: sample (the gallery's specimen)

/// The colour control with the theme's own colours, the folder colours as the presets, and a colour of its
/// own as the value, so the face shows a code.
fn sample_color_select(ui: crate::settings::UiColors, env: &Env, studio: &GalleryView) -> impl IntoElement {
    const NAME: &str = "Primary colour"; // text: sample (the gallery's specimen)
    let own = wormhole_core::theme::tokens::Color::from_hex(SAMPLE_COLOR).map(theme_bridge::hsla).ok();
    let theme = env.tokens.get("primary").map_or(ui.primary, theme_bridge::hsla);
    let presets = FolderColor::ALL
        .map(|name| Swatch { name: name.label().into(), color: theme_bridge::hsla(name.color(env.tokens)) });
    let choice = ColorChoice {
        theme,
        extras: Vec::new(),
        presets: presets.to_vec(),
        value: Value::Custom { text: SAMPLE_COLOR.into(), color: own },
        pick: Rc::new(|_: Picked, _| {}),
    };
    select::color_select("g-color-select", ui, NAME, &studio.sample_color, choice)
}
