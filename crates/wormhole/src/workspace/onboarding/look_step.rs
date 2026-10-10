//! The Look step: theme cards, the primary, the terminal font and ligatures, a sample.

use crate::workspace::settings_page::update_settings;
use wormhole_core::fonts::{FontSlot, display_name};
use wormhole_core::tokens::{Appearance, ThemeTokens, builtins};

use super::*;
use crate::theme_bridge::{hsla, token};

/// The terminal font's face, so the size stepper beside it fits the row.
const FONT_FACE_W: f32 = 132.0;
/// A theme card: its miniature and the name strip under it.
const CARD_H: f32 = 72.0;
const CARD_STRIP_H: f32 = 24.0;
const CARD_RING: f32 = 2.0;
const PREVIEW_BAR: f32 = 3.0;
const PREVIEW_PAD: f32 = 6.0;
const PREVIEW_GAP: f32 = 4.0;
const PREVIEW_RULE: f32 = 1.0;
/// The themes the step offers, if the build has them (more are in Settings).
const THEMES: [&str; 6] =
    ["Tokyo Night", "Catppuccin Mocha", "Nord", "Rose Pine", "Gruvbox Dark", "GitHub Light"];
/// The terminal sample under the font.
const SAMPLE: [&str; 4] =
    ["~/Projects/app on main", "$ git status --short", " M src/main.rs", "?? docs/notes.md"];

impl Workspace {
    /// Step 2: a few themes, the primary, the terminal font's size and ligatures, and a sample.
    pub(super) fn render_look_step(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let store = cx.global::<SettingsStore>();
        let s = &store.settings;
        let current = s.theme.clone();
        let mut themes: Vec<&ThemeTokens> =
            THEMES.iter().filter_map(|name| builtins().iter().find(|t| t.name == *name)).collect();
        // A build without some of them fills up with the others.
        for theme in builtins() {
            if themes.len() >= THEMES.len() {
                break;
            }
            if !themes.iter().any(|t| t.name == theme.name) {
                themes.push(theme);
            }
        }
        // The theme in use is always one of the cards.
        if !themes.iter().any(|t| t.name == current)
            && let Some(theme) = builtins().iter().find(|t| t.name == current)
        {
            themes.pop();
            themes.push(theme);
        }
        let cards =
            div().grid().grid_cols(3).gap_2().children(themes.into_iter().enumerate().map(|(ix, tokens)| {
                let name = tokens.name.clone();
                theme_card(ix, tokens, current == tokens.name, ui).on_click(move |_, _, cx| {
                    let name = name.clone();
                    update_settings(cx, |s| s.theme = name);
                })
            }));
        let base = store.base_tokens();
        let theme_primary = base.get("primary").map(|c| c.with_alpha(1.0));
        let chosen = s.primary();
        let mut colors = Vec::new();
        for ix in [2, 5, 1, 3, 6] {
            let color = base.terminal.ansi[ix].with_alpha(1.0);
            if Some(color.to_hex()) != theme_primary.map(|c| c.to_hex())
                && !colors.iter().any(|c: &wormhole_core::tokens::Color| c.to_hex() == color.to_hex())
            {
                colors.push(color);
            }
        }
        let swatches = div()
            .flex()
            .items_center()
            .gap_1()
            .children(theme_primary.map(|primary| {
                style::swatch("onboarding-primary-theme", ui, hsla(primary), chosen.is_none())
                    .on_click(|_, _, cx| update_settings(cx, |s| s.primary_color.clear()))
            }))
            .children(colors.into_iter().enumerate().map(|(ix, color)| {
                let hex = color.to_hex();
                let picked = chosen.is_some_and(|c| c.to_hex() == hex);
                style::swatch(("onboarding-primary", ix), ui, hsla(color), picked).on_click(
                    move |_, _, cx| {
                        let hex = hex.clone();
                        update_settings(cx, |s| s.primary_color = hex)
                    },
                )
            }));
        let family = store.fonts.family(FontSlot::Terminal).to_string();
        let size = store.fonts.size(FontSlot::Terminal);
        let bundled =
            cx.try_global::<crate::fonts::LoadedFonts>().is_some_and(|l| l.bundled.contains(&family));
        let font = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .max_w(px(FONT_FACE_W))
                    .h(px(style::CONTROL_H))
                    .px_2()
                    .rounded(ui.radius)
                    .outlined(ui, ui.border)
                    .bg(ui.input)
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_body()
                            .font_family(SharedString::from(family.clone()))
                            .child(display_name(&family).to_string()),
                    )
                    .when(bundled, |d| d.child(style::tint_chip(ui, None, "Bundled"))),
            )
            .child(style::stepper(
                ui,
                "onboarding-font-size",
                format!("{size:.0}"),
                |_, _, cx| update_settings(cx, |s| FontSlot::Terminal.step_size(s, -1.0)),
                |_, _, cx| update_settings(cx, |s| FontSlot::Terminal.step_size(s, 1.0)),
            ));
        let ligatures = s.font_ligatures;
        let term = &store.tokens.terminal;
        let fg = hsla(term.foreground);
        let ansi = |i: usize| hsla(term.ansi[i]);
        let sample = div()
            .flex()
            .flex_col()
            .px_3()
            .py_2()
            .rounded(ui.radius)
            .outlined(ui, ui.border)
            .bg(hsla(term.background))
            .font_family(SharedString::from(family.clone()))
            .font_features(crate::fonts::ligature_features(ligatures))
            .text_caption()
            .child(div().text_color(ui.muted).child(SAMPLE[0]))
            .child(div().text_color(fg).child(SAMPLE[1]))
            .child(div().text_color(ansi(3)).child(SAMPLE[2]))
            .child(div().text_color(ansi(1)).child(SAMPLE[3]));
        let row = |label: &'static str, control: AnyElement| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .min_h(px(style::CONTROL_H))
                .child(div().text_body().child(label))
                .child(control)
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .px_6()
            .pb_4()
            .child(cards)
            .child(
                style::link_button(
                    "onboarding-import",
                    ui,
                    "Import from Ghostty, iTerm2 or Alacritty…",
                    false,
                )
                .child(style::icon(IconName::ArrowDown))
                .on_click(cx.listener(|_, _, _, cx| import_theme(cx))),
            )
            .child(row("Primary colour", swatches.into_any_element()))
            .child(row("Terminal font", font.into_any_element()))
            .child(row(
                "Ligatures",
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_body()
                            .text_color(ui.muted)
                            .font_family(SharedString::from(family))
                            .font_features(crate::fonts::ligature_features(ligatures))
                            .child("=> != >="),
                    )
                    .child(style::toggle("onboarding-ligatures", ligatures, |on, _, cx| {
                        update_settings(cx, |s| s.font_ligatures = on)
                    }))
                    .into_any_element(),
            ))
            .child(sample)
            .into_any_element()
    }
}

/// A theme on the Look step: a miniature in its own colors and its name in a strip.
fn theme_card(ix: usize, tokens: &ThemeTokens, current: bool, ui: UiColors) -> Stateful<Div> {
    let t = |name: &str| token(tokens, name);
    let ansi = |i: usize| hsla(tokens.terminal.ansi[i]);
    let bar = |color: Hsla, width: f32| div().h(px(PREVIEW_BAR)).w(relative(width)).rounded_full().bg(color);
    let ring = UiColors { border_width: px(CARD_RING), ..ui };
    let solid = |name: &str| match (tokens.get(name), tokens.get("surface.window").map(|c| c.with_alpha(1.0)))
    {
        (Some(color), Some(window)) => hsla(color.over(window)),
        _ => t(name),
    };
    let light = tokens.appearance == Appearance::Light;
    div()
        .id(("onboarding-theme", ix))
        .h(px(CARD_H))
        .flex()
        .flex_col()
        .rounded(ui.radius)
        .overflow_hidden()
        .cursor_pointer()
        .map(|d| if current { d.outlined(ring, ui.primary) } else { d.outlined(ui, ui.border) })
        .when(!current, |d| d.hover(|d| d.border_color(ui.border_strong)))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .child(
                    div()
                        .w(relative(0.3))
                        .h_full()
                        .flex()
                        .flex_col()
                        .gap(px(PREVIEW_GAP))
                        .p(px(PREVIEW_PAD))
                        .bg(solid("sidebar.background"))
                        .border_r(px(PREVIEW_RULE))
                        .border_color(t("border"))
                        .child(bar(t("text.muted"), 0.8))
                        .child(bar(t("text.muted"), 0.6)),
                )
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .flex()
                        .flex_col()
                        .gap(px(PREVIEW_GAP))
                        .p(px(PREVIEW_PAD))
                        .bg(hsla(tokens.terminal.background))
                        .child(bar(t("primary"), 0.7))
                        .child(bar(ansi(2), 0.85))
                        .child(bar(ansi(3), 0.5)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .h(px(CARD_STRIP_H))
                .flex_none()
                .px_2()
                .bg(solid("surface.panel"))
                .text_caption()
                .font_weight(if current { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                .text_color(if current { t("primary") } else { t("text.primary") })
                .child(div().flex_1().min_w_0().truncate().child(tokens.name.clone()))
                .when(light && !current, |d| d.child(style::icon(IconName::Sun).text_color(t("text.muted"))))
                .when(current, |d| d.child(style::icon(IconName::Check).text_color(t("primary")))),
        )
}

/// Import a theme file the person picks; it becomes the theme shown.
fn import_theme(cx: &mut App) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Import theme".into()),
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = paths.await else { return };
        let Some(path) = paths.into_iter().next() else { return };
        cx.update(|cx| {
            if let Err(err) = SettingsStore::import_theme(cx, &path) {
                tracing::warn!("importing {}: {err}", path.display());
            }
            cx.refresh_windows();
        });
    })
    .detach();
}
