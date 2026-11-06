//! Fonts: one specimen for the three fonts, then the terminal font with its line height
//! and ligatures, the code and interface fonts, and the user's own fonts.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::TextSize;
use wormhole_core::theme::fonts::{FontSlot, display_name};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::open_button;
use crate::fonts::LoadedFonts;
use crate::select::Choice;
use crate::settings::{SettingsStore, settings};
use crate::settings_window::registry::{Setting, toggle};
use crate::settings_window::{SettingsWindow, open_folder, ui};
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge;

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("font-specimen", draw_specimen),
    Setting::custom("font-terminal", |w, cx| w.font_control(FontSlot::Terminal, cx)),
    Setting::custom("line-height", |w, cx| {
        let ui = ui(cx);
        let value = format!("{:.2}", settings(cx).line_height);
        style::slider_control(
            ui,
            style::slider(ui, &w.line_height_slider),
            value,
            &["1.0", "1.5", "2.0"],
            false,
        )
        .into_any_element()
    }),
    toggle!("font-ligatures", font_ligatures),
    Setting::custom("font-code", |w, cx| w.font_control(FontSlot::Code, cx)),
    Setting::custom("font-interface", |w, cx| w.font_control(FontSlot::Interface, cx)),
    Setting::custom("settings-text-size", draw_text_size),
    Setting::custom("fonts-folder", |_, _| {
        open_button("fonts-folder", IconName::ExternalLink, text(SETTINGS_FONTS_OPEN_FONTS_FOLDER), |cx| {
            open_folder(&crate::active_profile::paths().fonts_dir(), cx)
        })
    }),
    Setting::custom("custom-fonts", |_, _| div().into_any_element()).describe(describe_custom),
];

/// The words of a step of the Settings text size.
fn text_size_label(size: TextSize) -> &'static str {
    text(match size {
        TextSize::Small => SETTINGS_FONTS_TEXT_SIZE_SMALL,
        TextSize::Default => SETTINGS_FONTS_TEXT_SIZE_DEFAULT,
        TextSize::Large => SETTINGS_FONTS_TEXT_SIZE_LARGE,
        TextSize::Larger => SETTINGS_FONTS_TEXT_SIZE_LARGER,
    })
}

/// The Settings text size: a select of the four steps, applied to this window and the Theme Studio as
/// soon as it is picked (`style::text_scale`).
fn draw_text_size(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let current = settings(cx).settings_text_size;
    let choices = TextSize::ALL
        .iter()
        .map(|size| Choice::new(size.name().to_string(), text_size_label(*size)))
        .collect();
    window.select(
        "settings-text-size",
        div().child(text_size_label(current)).into_any_element(),
        style::SELECT_W,
        choices,
        current.name().to_string(),
        |s, name| {
            if let Some(size) = TextSize::from_name(name) {
                s.settings_text_size = size;
            }
        },
        cx,
    )
}

/// "2 families found: Berkeley Mono, Monaspace Neon".
fn describe_custom(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let families = cx.try_global::<LoadedFonts>().map(LoadedFonts::custom_families).unwrap_or_default();
    Some(match families.len() {
        0 => text(SETTINGS_FONTS_NONE_YET).into(),
        n => settings_fonts_families_found(whole(n), &families.join(", ")).into(),
    })
}

/// The specimen's text: a prompt, a result, code with the pairs ligatures join, and the
/// characters that tell fonts apart.
const SPECIMEN: [&str; 4] = [
    "→ wormhole (main) cargo test", // text: sample (the specimen's terminal text)
    "test result: ok. 57 passed; 0 failed", // text: sample (the specimen's terminal text)
    "fn resolve(&self) -> Result<Tokens>", // text: sample (the specimen's terminal text)
    "0O 1lI {}[] => != ~ |> <= www //", // text: sample (the specimen's terminal text)
];

/// One specimen, Terminal | Code | Interface, in the theme's terminal colors at the slot's
/// size (the terminal's line height too); a font under the pointer in an open picker shows
/// in it.
pub(in crate::settings_window) fn draw_specimen(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let store = cx.global::<SettingsStore>();
    let slot = window.specimen;
    let family = window
        .font_picker
        .hovered
        .clone()
        .filter(|_| window.font_picker.open == Some(slot))
        .unwrap_or_else(|| store.fonts.family(slot).to_string());
    let size = store.fonts.size(slot);
    let s = &store.settings;
    let line_height = if slot == FontSlot::Terminal { s.line_height } else { 1.4 };
    let term = &store.tokens.terminal;
    let ansi = |i: usize| theme_bridge::hsla(term.ansi[i]);
    let theme_name = store.tokens.name.clone();
    let features = crate::fonts::ligature_features(s.font_ligatures);
    let tabs = style::segmented(ui).children(FontSlot::ALL.into_iter().rev().enumerate().map(|(ix, tab)| {
        style::segment(("specimen-tab", ix), ui, tab.title(), tab == slot).on_click(cx.listener(
            move |this, _, _, cx| {
                this.specimen = tab;
                cx.notify();
            },
        ))
    }));
    // A line never wraps, so the specimen is four lines tall at any width; the column cuts one that is
    // longer, as a terminal's edge does.
    let line = |parts: Vec<(Hsla, &'static str)>| {
        div().flex().overflow_hidden().children(
            parts
                .into_iter()
                .map(|(color, text)| div().flex_none().whitespace_nowrap().text_color(color).child(text)),
        )
    };
    let fg = theme_bridge::hsla(term.foreground);
    let specimen = div()
        .flex()
        .flex_col()
        .px_4()
        .py_3()
        .overflow_hidden()
        .bg(theme_bridge::hsla(term.background))
        .font_family(SharedString::from(family.clone()))
        .font_features(features)
        .text_size(px(size))
        .line_height(relative(line_height))
        .child(line(vec![
            (ansi(2), "→ "),
            (ansi(4), "wormhole "), // text: sample (the specimen's terminal text)
            (ansi(5), "(main) "),   // text: sample (the specimen's terminal text)
            (fg, "cargo test"),     // text: sample (the specimen's terminal text)
        ]))
        .child(line(vec![(fg, "test result: "), (ansi(2), "ok"), (fg, ". 57 passed; 0 failed")])) // text: sample (the specimen's terminal text)
        .child(line(vec![
            (ansi(5), "fn "), // text: sample (the specimen's terminal text)
            (ansi(4), "resolve"),
            (fg, "(&self) -> Result<Tokens>"), // text: sample (the specimen's terminal text)
        ]))
        .child(line(vec![(fg, "{ a != b && c >= d }")])) // text: sample (the specimen's terminal text)
        .child(line(vec![(ui.muted, SPECIMEN[3])]));
    // What the specimen is set in, on one line that an ellipsis cuts where the column is narrow.
    let mut facts = vec![settings_fonts_specimen_size(display_name(&family), &format!("{size:.0}"))];
    if slot == FontSlot::Terminal {
        facts.push(settings_fonts_specimen_line_height(&format!("{:.2}", s.line_height)));
    }
    if slot != FontSlot::Interface {
        facts.push(
            text(if s.font_ligatures { SETTINGS_FONTS_LIGATURES_ON } else { SETTINGS_FONTS_LIGATURES_OFF })
                .to_string(),
        );
    }
    facts.push(settings_fonts_specimen_colours(&theme_name));
    let facts = div()
        .px_4()
        .py_2()
        .rule_top(ui)
        .text_caption()
        .text_color(ui.muted)
        .truncate()
        .child(facts.join("   "));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_col()
                .rounded(ui.radius_lg)
                .overflow_hidden()
                .outlined(ui, ui.border)
                .bg(ui.elevated)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_2()
                        .py_2()
                        .rule_bottom(ui)
                        .child(tabs)
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_caption()
                                .text_color(ui.faint)
                                .child(text(SETTINGS_FONTS_HOVER_A_FONT_IN_A_LIST)),
                        ),
                )
                .child(specimen)
                .child(facts),
        )
        .into_any_element()
}
