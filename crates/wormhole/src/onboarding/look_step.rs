//! The Look step: theme cards, the primary, the terminal font and ligatures, a sample.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::import::{Found, Proposed};
use wormhole_core::onboarding::{self, Step};
use wormhole_core::theme::fonts::{FontSlot, display_name};
use wormhole_core::theme::tokens::{Appearance, ThemeTokens, builtins};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::whole;

use super::{FirstRun, Intro};
use crate::settings::import::{self as shared, Brought, terminal_name};
use crate::settings::update_settings;
use crate::settings::{SettingsStore, UiColors};
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge::{hsla, token};

/// The terminal font's face, so the size stepper beside it fits the row.
const FONT_FACE_W: f32 = 132.0;
/// A theme card: its miniature and the name strip under it.
const CARD_H: f32 = 72.0;
const CARD_STRIP_H: f32 = 24.0;
const PREVIEW_BAR: f32 = 3.0;
const PREVIEW_PAD: f32 = 6.0;
const PREVIEW_GAP: f32 = 4.0;
const PREVIEW_RULE: f32 = 1.0;
/// The terminal colours that are offered as the primary, by their place in the 16: green, magenta,
/// red, yellow and cyan.
const PRIMARY_ANSI: [usize; 5] = [2, 5, 1, 3, 6];
/// The terminal sample under the font.
const SAMPLE: [&str; 4] =
    ["~/Projects/app on main", "$ git status --short", " M src/main.rs", "?? docs/notes.md"];

impl FirstRun {
    /// Step 2: a few themes, the primary, the terminal font's size and ligatures, and a sample.
    pub(super) fn render_look_step(&self, sheet: &Intro, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let store = cx.global::<SettingsStore>();
        let s = &store.settings;
        let current = s.theme.clone();
        let themes = offered_themes(&current);
        let cards =
            div().grid().grid_cols(3).gap_2().children(themes.into_iter().enumerate().map(|(ix, tokens)| {
                let name = tokens.name.clone();
                theme_card(ix, tokens, current == tokens.name, ui).on_click(move |_, _, cx| {
                    let name = name.clone();
                    update_settings(cx, |s| s.theme = name);
                })
            }));
        let base = store.base_tokens();
        let (theme_primary, colors) = offered_primaries(base);
        let chosen = s.primary();
        let swatches = div()
            .flex()
            .items_center()
            .gap_1()
            .children(theme_primary.map(|primary| {
                style::swatch("onboarding-primary-theme", ui, hsla(primary), chosen.is_none())
                    .on_click(|_, _, cx| update_settings(cx, |s| s.choose_primary(String::new())))
            }))
            .children(colors.into_iter().enumerate().map(|(ix, (_, color))| {
                let hex = color.to_hex();
                let picked = chosen.is_some_and(|c| c.to_hex() == hex);
                style::swatch(("onboarding-primary", ix), ui, hsla(color), picked).on_click(
                    move |_, _, cx| {
                        let hex = hex.clone();
                        update_settings(cx, |s| s.choose_primary(hex))
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
                |_, _, cx| update_settings(cx, |s| s.step_slot_size(FontSlot::Terminal, -1.0)),
                |_, _, cx| update_settings(cx, |s| s.step_slot_size(FontSlot::Terminal, 1.0)),
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
            .child(self.render_import(sheet, ui, cx))
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
            .children(self.render_tailor(ui, cx))
            .into_any_element()
    }
}

/// What the step knows of the terminals on this Mac: the newest one's settings, once looked for.
#[derive(Default)]
pub(crate) struct LookImport {
    started: bool,
    task: Option<Task<()>>,
    /// The terminal whose settings were changed last that has something to bring over by default.
    found: Option<Found>,
    /// What bringing them over did.
    done: Option<Brought>,
    /// Why it didn't work, in the words of the Import page.
    error: Option<SharedString>,
}

impl FirstRun {
    /// The first draw of the look step starts the scan (off the UI thread; it reads another terminal's
    /// files, so it waits until the step is shown).
    pub(super) fn look_step_shown(&mut self, cx: &mut Context<Self>) {
        let Some(intro) = self.intro.as_mut() else { return };
        if intro.model.step() != Step::Look || intro.import.started {
            return;
        }
        intro.import.started = true;
        intro.import.task = Some(cx.spawn(async move |this, cx| {
            let found = cx.background_executor().spawn(async { shared::scan_here() }).await;
            this.update(cx, |this, cx| {
                if let Some(intro) = &mut this.intro {
                    // The newest terminal with something ticked by default: one with nothing to "use" leaves
                    // the theme file import where it is.
                    intro.import.found =
                        found.into_iter().find(|found| found.proposal.selected().next().is_some());
                    intro.import.task = None;
                }
                cx.notify();
            })
            .ok();
        }));
    }

    /// "Use my <terminal> settings" when one was found, else the theme file import. After it, the line
    /// that says what came over.
    fn render_import(&self, sheet: &Intro, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let Some(found) = sheet.import.found.as_ref() else {
            return style::link_button(
                "onboarding-import",
                ui,
                "Import from Ghostty, iTerm2 or Alacritty…",
                false,
            )
            .child(style::icon(IconName::ArrowDown))
            .on_click(cx.listener(|_, _, _, cx| import_theme(cx)))
            .into_any_element();
        };
        let name = terminal_name(found.source);
        let button = style::link_button("onboarding-import", ui, settings_import_use_mine(name), false)
            .child(style::icon(IconName::ArrowDown))
            .on_click(cx.listener(|this, _, _, cx| this.use_found(cx)));
        let failed =
            sheet.import.error.clone().map(|why| div().text_caption().text_color(ui.danger_text).child(why));
        let said = sheet.import.done.map(|done| {
            let text = if done.left == 0 {
                settings_import_onboarding_all(whole(done.brought))
            } else {
                settings_import_onboarding_summary(whole(done.brought), &done.left.to_string())
            };
            div().text_caption().text_color(ui.success_text).child(text)
        });
        // The button, and under it what came over: the sentence is longer than the step is wide.
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap_1()
            .child(button)
            .children(said.map(|said| said.w_full()))
            .children(failed.map(|failed| failed.w_full()))
            .into_any_element()
    }

    /// Bring over what the found terminal's settings tick by default, in one settings update. Read when
    /// the button is pressed, not each frame.
    fn use_found(&mut self, cx: &mut Context<Self>) {
        let Some(found) = self.intro.as_ref().and_then(|intro| intro.import.found.clone()) else { return };
        let chosen: Vec<Proposed> = found.proposal.selected().cloned().collect();
        match shared::apply(cx, &found, &chosen) {
            Ok(done) => {
                if let Some(intro) = &mut self.intro {
                    intro.import.done = Some(done);
                    intro.import.error = None;
                }
            }
            Err(why) => {
                tracing::warn!("importing from {}: {why}", found.source.key());
                if let Some(intro) = &mut self.intro {
                    intro.import.error = Some(why.into());
                }
            }
        }
        cx.notify();
    }
}

/// The themes the step offers, if the build has them (more are in Settings): core's list
/// (`onboarding::LOOK_THEMES`), filled up with others where a build lacks some, and always with the
/// theme in use (`current`) among them. The terminal welcome offers the same.
pub(crate) fn offered_themes(current: &str) -> Vec<&'static ThemeTokens> {
    let wanted = onboarding::LOOK_THEMES;
    let mut themes: Vec<&ThemeTokens> =
        wanted.iter().filter_map(|name| builtins().iter().find(|t| t.name == *name)).collect();
    for theme in builtins() {
        if themes.len() >= wanted.len() {
            break;
        }
        if !themes.iter().any(|t| t.name == theme.name) {
            themes.push(theme);
        }
    }
    if !themes.iter().any(|t| t.name == current)
        && let Some(theme) = builtins().iter().find(|t| t.name == current)
    {
        themes.pop();
        themes.push(theme);
    }
    themes
}

/// The primary colours the step offers for the theme `base`: the theme's own, and those of its
/// terminal colours that differ from it, each with its place in the terminal's 16.
pub(crate) fn offered_primaries(
    base: &ThemeTokens,
) -> (Option<wormhole_core::theme::tokens::Color>, Vec<(usize, wormhole_core::theme::tokens::Color)>) {
    let theme_primary = base.get("primary").map(|c| c.with_alpha(1.0));
    let mut colors: Vec<(usize, wormhole_core::theme::tokens::Color)> = Vec::new();
    for ix in PRIMARY_ANSI {
        let color = base.terminal.ansi[ix].with_alpha(1.0);
        if Some(color.to_hex()) != theme_primary.map(|c| c.to_hex())
            && !colors.iter().any(|(_, c)| c.to_hex() == color.to_hex())
        {
            colors.push((ix, color));
        }
    }
    (theme_primary, colors)
}

/// A theme on the Look step: a miniature in its own colors and its name in a strip.
fn theme_card(ix: usize, tokens: &ThemeTokens, current: bool, ui: UiColors) -> Stateful<Div> {
    let t = |name: &str| token(tokens, name);
    let ansi = |i: usize| hsla(tokens.terminal.ansi[i]);
    let bar = |color: Hsla, width: f32| div().h(px(PREVIEW_BAR)).w(relative(width)).rounded_full().bg(color);
    let solid = |name: &str| match (tokens.get(name), tokens.get("surface.window").map(|c| c.with_alpha(1.0)))
    {
        (Some(color), Some(window)) => hsla(color.over(window)),
        _ => t(name),
    };
    let light = tokens.appearance == Appearance::Light;
    style::choice_card(("onboarding-theme", ix), ui, current)
        .aria_label(tokens.name.clone())
        .h(px(CARD_H))
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
