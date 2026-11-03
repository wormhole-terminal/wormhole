//! The profile chip, its menu and avatars, the stepper and switch, the swatches and chips, and the inline banners:
//! the shared pieces of the interface that no window of their own shows.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use style::{BannerKind, TextRole};

use super::Env;
use super::view::GalleryView;
use crate::settings::UiColors;
use crate::style::{self, Edges};
use crate::theme_bridge;

/// The letters of the sample row's tiles.
const LETTERS: &str = "DWPCSRABX"; // text: sample (the gallery's specimen)

/// The column the samples are as wide as.
const COLUMN_W: f32 = 560.0;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    match part {
        0 => profile_marks(env).into_any_element(),
        1 => controls(env).into_any_element(),
        _ => banners(env.ui).into_any_element(),
    }
}

/// The small controls that are ours rather than GPUI Kit's.
fn controls(env: &Env) -> Div {
    let ui = env.ui;
    let palette = [ui.primary, ui.success, ui.branch, ui.warning];
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_4()
        .child(style::stepper(ui, "g-form-size", "14 pt".into(), |_, _, _| {}, |_, _, _| {})) // text: sample (the gallery's specimen)
        .child(style::toggle("g-toggle-on", true, |_, _, _| {}))
        .child(style::toggle("g-toggle-off", false, |_, _, _| {}))
        .children(
            palette
                .into_iter()
                .enumerate()
                .map(|(ix, color)| style::swatch(("g-swatch", ix), ui, color, ix == 0)),
        )
        .child(env.mark("tint.chip", style::tint_chip(ui, Some(IconName::Folder), "api-server"))) // text: sample (the gallery's specimen)
        .child(env.mark("tint.chip", style::tint_chip(ui, None, "Bundled"))) // text: sample (the gallery's specimen)
        .child(
            env.mark("link", div().text_body().text_color(env.tk("text.link")).underline().child("A link")),
        ) // text: sample (the gallery's specimen)
        .child(style::changed_dot(ui))
}

/// A profile's avatar at the sizes the app draws it, and the edge a window in another profile wears.
fn profile_marks(env: &Env) -> Div {
    let ui = env.ui;
    let colors = wormhole_core::view::sidebar::GroupColor::ALL
        .map(|color| (color, theme_bridge::hsla(color.color(env.tokens))));
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(div().flex().items_center().gap_2().children(colors.iter().zip(LETTERS.chars()).map(
            |((_, color), letter)| {
                // text: sample (the gallery's specimen)
                style::profile_avatar(ui, *color, letter, style::TITLE_GLYPH)
            },
        )))
        .child(div().flex().items_center().gap_2().children(colors.iter().zip(LETTERS.chars()).take(3).map(
            |((_, color), letter)| {
                // text: sample (the gallery's specimen)
                style::profile_avatar(ui, *color, letter, PROFILE_AVATAR_LARGE)
            },
        )))
        .child(div().flex().items_center().gap_3().child(style::profile_chip(
            "g-profile-chip",
            ui,
            colors[1].1,
            'W',
            "Work",                         // text: sample (the gallery's specimen)
            "Profile Work, switch profile", // text: sample (the gallery's specimen)
        )))
        .child(
            div()
                .w(px(PROFILE_MENU_W))
                .p_1()
                .rounded(ui.radius_lg)
                .bg(ui.elevated)
                .outlined(ui, ui.border_strong)
                .child(style::popover_heading(ui, "Switch Wormhole to")) // text: sample (the gallery's specimen)
                .child(style::popover_row(
                    "g-menu-default",
                    ui,
                    style::PopoverRow {
                        lead: Some(
                            style::profile_avatar(ui, colors[0].1, 'D', style::TITLE_GLYPH)
                                .into_any_element(),
                        ),
                        note: Some("every window switches".into()), // text: sample (the gallery's specimen)
                        ..style::PopoverRow::new("Default")         // text: sample (the gallery's specimen)
                    },
                ))
                .child(style::popover_row(
                    "g-menu-work",
                    ui,
                    style::PopoverRow {
                        lead: Some(
                            style::profile_avatar(ui, colors[1].1, 'W', style::TITLE_GLYPH)
                                .into_any_element(),
                        ),
                        note: Some("current".into()),
                        current: true,
                        ..style::PopoverRow::new("Work") // text: sample (the gallery's specimen)
                    },
                ))
                .child(style::popover_rule(ui))
                .child(style::popover_row(
                    "g-menu-settings",
                    ui,
                    style::PopoverRow { key: Some("⌘,".into()), ..style::PopoverRow::new("Settings…") }, // text: sample (the gallery's specimen)
                )),
        )
        .child(
            div()
                .w(px(COLUMN_W))
                .rounded(ui.radius)
                .overflow_hidden()
                .outlined(ui, ui.border)
                .child(style::profile_edge(colors[4].1))
                .child(div().h(px(PROFILE_EDGE_SAMPLE_H)).bg(ui.background)),
        )
}

/// The larger avatar (the settings window's top) and the height of the window sample under the edge.
const PROFILE_AVATAR_LARGE: f32 = 24.0;
/// The profile chip's menu sample.
const PROFILE_MENU_W: f32 = 280.0;
const PROFILE_EDGE_SAMPLE_H: f32 = 24.0;

/// Inline banners of each kind, and the flush one of a panel.
fn banners(ui: UiColors) -> Div {
    let small = |id: &'static str, label: &'static str, outline: bool| {
        Button::new(id).small().when(outline, |b| b.outline()).when(!outline, |b| b.ghost()).label(label)
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .max_w(px(COLUMN_W))
        .child(
            style::banner(
                ui,
                BannerKind::Info,
                false,
                "This terminal shows its project's theme. Your own is kept.", // text: sample (the gallery's specimen)
            )
            .child(small("g-banner-info", "Project settings", false)), // text: sample (the gallery's specimen)
        )
        .child(
            style::banner(
                ui,
                BannerKind::Warning,
                false,
                "config.json line 12: expected “,”. Using the last good settings.", // text: sample (the gallery's specimen)
            )
            .child(small("g-banner-warning", "Open", true)), // text: sample (the gallery's specimen)
        )
        .child(style::banner(
            ui,
            BannerKind::Danger,
            false,
            "Couldn't write ~/.gemini/settings.json: permission denied.", // text: sample (the gallery's specimen)
        ))
        .child(style::banner(ui, BannerKind::Success, false, "Status hooks installed for Claude Code.")) // text: sample (the gallery's specimen)
        .child(style::banner(
            ui,
            BannerKind::Warning,
            true,
            "The settings file changed on disk. Reload to use it.", // text: sample (the gallery's specimen)
        ))
}
