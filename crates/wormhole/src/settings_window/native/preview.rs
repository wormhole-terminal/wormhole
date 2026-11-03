//! What the previews draw from: the terminal's real fonts, the themes in effect for dark and light, and the accent.
//! The window's previews read these instead of a table of samples, so a Menlo terminal shows Menlo and the picked
//! theme shows its own colours.

use gpui_kit::App;
use serde_json::{Value, json};
use wormhole_core::config::Settings;
use wormhole_core::theme::catalog::ThemeEntry;
use wormhole_core::theme::fonts::FontSlot;
use wormhole_core::theme::tokens::Appearance;

use super::themes;
use crate::settings::SettingsStore;

pub(super) fn build(entries: &[ThemeEntry], s: &Settings, cx: &App) -> Value {
    let fonts = &cx.global::<SettingsStore>().fonts;
    let font = |slot| json!([fonts.family(slot), fonts.size(slot)]);
    let side = |appearance| {
        let entry = themes::effective(entries, s, appearance);
        let accent = s
            .primary()
            .or_else(|| entry.and_then(|e| e.tokens.get("primary")))
            .map(|c| Value::from(themes::hex(c)));
        json!({ "palette": entry.map(themes::palette), "accent": accent })
    };
    json!({
        "fonts": { "terminal": font(FontSlot::Terminal), "code": font(FontSlot::Code), "interface": font(FontSlot::Interface) },
        "line_height": s.line_height,
        "ligatures": s.font_ligatures,
        "dark": side(Appearance::Dark),
        "light": side(Appearance::Light),
    })
}
