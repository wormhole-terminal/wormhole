//! The two colour rows of Appearance, the primary and the branch, over the one colour control
//! (`select::color_select`): this says what each setting is now, in the control's words, and what a choice does
//! to it.
//!
//! The presets are the folder colours as the theme draws them (`FolderColor::color`), taken from the theme
//! before the person's own colours are laid over it, so choosing one doesn't move the others. A stored
//! colour that equals a preset shows as that preset; any other shows as Custom with its own swatch, and a
//! value written by hand that isn't a colour at all shows as it is, until another is chosen.

use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::config::BranchColor;
use wormhole_core::theme::folder_color::FolderColor;
use wormhole_core::theme::tokens::{BranchTint, Color, ThemeTokens};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::select::{self, ColorChoice, Picked, Swatch, Value};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore, settings};
use crate::settings_window::registry::row_label;
use crate::settings_window::{SettingsWindow, ui};
use crate::theme_bridge::hsla;

/// The eight presets: the folder colours, in the theme's own look.
fn presets(base: &ThemeTokens) -> Vec<(FolderColor, Color)> {
    FolderColor::ALL.into_iter().map(|name| (name, name.color(base))).collect()
}

/// Whether a stored colour is a preset, by the colour it makes (the hex code's bytes).
fn preset_at(presets: &[(FolderColor, Color)], color: Color) -> Option<usize> {
    presets.iter().position(|(_, preset)| preset.to_bytes() == color.with_alpha(1.0).to_bytes())
}

/// The value of a colour that is the person's own: a preset when it is one, else Custom with its text.
fn own(presets: &[(FolderColor, Color)], written: &str, color: Option<Color>) -> Value {
    match color.and_then(|color| preset_at(presets, color)) {
        Some(at) => Value::Preset(at),
        None => Value::Custom { text: written.trim().to_string().into(), color: color.map(hsla) },
    }
}

fn swatches(presets: &[(FolderColor, Color)]) -> Vec<Swatch> {
    presets.iter().map(|(name, color)| Swatch { name: name.label().into(), color: hsla(*color) }).collect()
}

/// The primary in force: the person's, else the theme's own.
fn primary_in_force(settings: &Settings, base: &ThemeTokens) -> Color {
    settings.primary().or_else(|| base.get("primary")).map_or(Color::BLACK, |c| c.with_alpha(1.0))
}

/// The branch colour as the person set it (not the readable one the window draws): their own, the
/// primary's, else the theme's.
fn branch_in_force(settings: &Settings, base: &ThemeTokens) -> Color {
    let theme = base.get("git.branch").map_or(Color::BLACK, |c| c.with_alpha(1.0));
    match settings.branch_tint() {
        Some(BranchTint::Color(color)) => color,
        Some(BranchTint::Primary) => primary_in_force(settings, base),
        None => theme,
    }
}

/// The colours the editors start from, for `SettingsWindow::sync_color_picks`.
pub(in crate::settings_window) fn in_force(store: &SettingsStore) -> (Color, Color) {
    let base = store.base_tokens();
    (primary_in_force(&store.settings, base), branch_in_force(&store.settings, base))
}

/// The primary's control.
pub(in crate::settings_window) fn draw_primary(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let store = cx.global::<SettingsStore>();
    let base = store.base_tokens();
    let presets = presets(base);
    let value = if store.settings.uses_theme_primary() {
        Value::Theme
    } else {
        own(&presets, &store.settings.primary_color, store.settings.primary())
    };
    let choice = ColorChoice {
        theme: hsla(base.get("primary").map_or(Color::BLACK, |c| c.with_alpha(1.0))),
        extras: Vec::new(),
        presets: swatches(&presets),
        value,
        pick: Rc::new(|picked, cx| match picked {
            Picked::Theme => update_settings(cx, Settings::use_theme_primary),
            Picked::Hex(hex) => {
                // A drag passes through many shades that round to the code already set.
                let now = settings(cx);
                if now.uses_theme_primary() || now.primary_color != hex {
                    update_settings(cx, |s| s.choose_primary(hex));
                }
            }
            Picked::Extra(_) => {}
        }),
    };
    select::color_select("primary-color", ui, row_label("primary-color"), &window.primary_pick, choice)
        .into_any_element()
}

/// The branch colour's control: the theme's colour, the primary, the presets, Custom….
pub(super) fn draw_branch(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let store = cx.global::<SettingsStore>();
    let (base, s) = (store.base_tokens(), &store.settings);
    let presets = presets(base);
    let value = match s.branch_color {
        BranchColor::Theme => Value::Theme,
        BranchColor::Primary => Value::Extra(0),
        BranchColor::Custom => {
            let color = match s.branch_tint() {
                Some(BranchTint::Color(color)) => Some(color),
                _ => None,
            };
            own(&presets, &s.branch_color_custom, color)
        }
    };
    let choice = ColorChoice {
        theme: hsla(base.get("git.branch").map_or(Color::BLACK, |c| c.with_alpha(1.0))),
        extras: vec![Swatch {
            name: text(SETTINGS_APPEARANCE_BRANCH_PRIMARY).into(),
            color: hsla(primary_in_force(s, base)),
        }],
        presets: swatches(&presets),
        value,
        pick: Rc::new(|picked, cx| match picked {
            Picked::Theme => update_settings(cx, |s| s.choose_branch_source(BranchColor::Theme)),
            Picked::Extra(_) => update_settings(cx, |s| s.choose_branch_source(BranchColor::Primary)),
            Picked::Hex(hex) => {
                let now = settings(cx);
                if now.branch_color != BranchColor::Custom || now.branch_color_custom != hex {
                    update_settings(cx, |s| s.choose_branch_color(hex));
                }
            }
        }),
    };
    select::color_select("branch-color", ui, row_label("branch-color"), &window.branch_pick, choice)
        .into_any_element()
}

/// What both colours are by default, for the Changed page.
pub(super) fn theme_words() -> SharedString {
    text(COLOR_PICK_THEME).into()
}
