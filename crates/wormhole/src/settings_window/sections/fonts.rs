//! Fonts: one specimen for the three fonts, then the terminal font with its line height
//! and ligatures, the code and interface fonts, and the user's own fonts.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::fonts::{FontSlot, display_name};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::open_button;
use crate::fonts::LoadedFonts;
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
        let height = settings(cx).line_height;
        let value = format!("{height:.2}");
        let native = super::super::native_slider::control(
            w,
            "line-height",
            text(SETTINGS_FONTS_LINE_HEIGHT),
            f64::from(height),
            1.0,
            2.0,
            0.05,
            w.stepper_enabled("line-height", cx),
            cx,
            |value, _, cx| {
                let height = ((value * 20.0).round() / 20.0).clamp(1.0, 2.0) as f32;
                if (settings(cx).line_height - height).abs() > f32::EPSILON {
                    crate::settings::update_settings(cx, |s| s.line_height = height);
                }
            },
        );
        style::slider_control(
            ui,
            native.unwrap_or_else(|| {
                gpui_kit::component::slider::Slider::new(&w.line_height_slider)
                    .accessibility_label(text(SETTINGS_FONTS_LINE_HEIGHT))
                    .into_any_element()
            }),
            value,
            &["1.0", "1.5", "2.0"],
            false,
        )
        .into_any_element()
    }),
    toggle!("font-ligatures", font_ligatures),
    Setting::custom("font-code", |w, cx| w.font_control(FontSlot::Code, cx)),
    Setting::custom("font-interface", |w, cx| w.font_control(FontSlot::Interface, cx)),
    Setting::custom("fonts-folder", |w, cx| {
        open_button(
            w,
            cx,
            "fonts-folder",
            IconName::ExternalLink,
            text(SETTINGS_FONTS_OPEN_FONTS_FOLDER),
            |cx| open_folder(&crate::active_profile::paths().fonts_dir(), cx),
        )
    }),
    Setting::custom("custom-fonts", |_, _| div().into_any_element()).describe(describe_custom),
];

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
    "→ wormhole (main) cargo test --workspace", // text: sample (the specimen's terminal text)
    "test result: ok. 57 passed; 0 failed",     // text: sample (the specimen's terminal text)
    "fn resolve(&self) -> Result<Tokens> { a != b && c >= d }", // text: sample (the specimen's terminal text)
    "0O 1lI {}[] => != ~ |> <= www // ligatures", // text: sample (the specimen's terminal text)
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
    let tabs = |cx: &mut Context<SettingsWindow>| -> AnyElement {
        if window.native_switch_allowed("specimen-tab", cx) {
            if let Some(native) = &window.native_segments {
                let slots: Vec<_> = FontSlot::ALL.into_iter().rev().collect();
                let selected = slots.iter().position(|tab| *tab == slot).unwrap_or(0);
                let options = slots
                    .iter()
                    .map(|tab| crate::platform::controls::SegmentOption {
                        label: tab.title().to_owned(),
                        symbol: None,
                    })
                    .collect();
                let page = cx.weak_entity();
                super::super::native_segments::control_options(
                    window,
                    native.clone(),
                    "specimen-tab",
                    options,
                    selected,
                    true,
                    None,
                    cx,
                    move |index, _, cx| {
                        if let Some(tab) = slots.get(index).copied() {
                            page.update(cx, |this, cx| {
                                this.specimen = tab;
                                cx.notify();
                            })
                            .ok();
                        }
                    },
                )
            } else {
                specimen_tabs_fallback(ui, slot, cx)
            }
        } else {
            specimen_tabs_fallback(ui, slot, cx)
        }
    };
    let line = |parts: Vec<(Hsla, &'static str)>| {
        div().flex().flex_wrap().children(
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
            (fg, "cargo test --workspace"), // text: sample (the specimen's terminal text)
        ]))
        .child(line(vec![(fg, "test result: "), (ansi(2), "ok"), (fg, ". 57 passed; 0 failed")])) // text: sample (the specimen's terminal text)
        .child(line(vec![
            (ansi(5), "fn "), // text: sample (the specimen's terminal text)
            (ansi(4), "resolve"),
            (fg, "(&self) -> Result<Tokens> { a != b && c >= d }"), // text: sample (the specimen's terminal text)
        ]))
        .child(line(vec![(ui.muted, SPECIMEN[3])]));
    let facts = div()
        .flex()
        .flex_wrap()
        .gap_x_4()
        .px_4()
        .py_2()
        .rule_top(ui)
        .text_caption()
        .text_color(ui.muted)
        .child(settings_fonts_specimen_size(display_name(&family), &format!("{size:.0}")))
        .when(slot == FontSlot::Terminal, |d| {
            d.child(settings_fonts_specimen_line_height(&format!("{:.2}", s.line_height)))
        })
        .when(slot != FontSlot::Interface, |d| {
            d.child(if s.font_ligatures {
                text(SETTINGS_FONTS_LIGATURES_ON)
            } else {
                text(SETTINGS_FONTS_LIGATURES_OFF)
            })
        })
        .child(settings_fonts_specimen_colours(&theme_name));
    let tabs = tabs(cx);
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

fn specimen_tabs_fallback(
    ui: crate::settings::UiColors,
    slot: FontSlot,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    style::segmented(ui)
        .children(FontSlot::ALL.into_iter().rev().enumerate().map(|(ix, tab)| {
            style::segment(("specimen-tab", ix), ui, tab.title(), tab == slot).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.specimen = tab;
                    cx.notify();
                },
            ))
        }))
        .into_any_element()
}
