//! The controls of the appearance page the native window writes through: each setting's menu or switch over the settings.

use wormhole_core::config::ThemeMode;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::{Current, MenuOption, Setting, toggle};

const MODES: &[MenuOption] = &[
    MenuOption::text(SETTINGS_APPEARANCE_MATCH_SYSTEM),
    MenuOption::text(SETTINGS_APPEARANCE_DARK),
    MenuOption::text(SETTINGS_APPEARANCE_LIGHT),
];

/// The opacities the menu lists, in per cent.
const OPACITIES: [u32; 5] = [100, 95, 90, 80, 70];

/// "80 %".
fn percent(n: u32) -> String {
    format!("{n} %")
}

pub(super) const SETTINGS: &[Setting] = &[
    Setting::menu(
        "theme-mode",
        MODES,
        |s| match s.theme_mode {
            ThemeMode::System => 0,
            ThemeMode::Dark => 1,
            ThemeMode::Light => 2,
        },
        |s, ix| {
            s.theme_mode = match ix {
                0 => ThemeMode::System,
                1 => ThemeMode::Dark,
                _ => ThemeMode::Light,
            }
        },
    ),
    Setting::menu_of_values(
        "opacity",
        &[
            MenuOption::counted(percent, OPACITIES[0]),
            MenuOption::counted(percent, OPACITIES[1]),
            MenuOption::counted(percent, OPACITIES[2]),
            MenuOption::counted(percent, OPACITIES[3]),
            MenuOption::counted(percent, OPACITIES[4]),
        ],
        // An opacity written in config.json (or left by the slider this menu replaced) that the menu does not
        // list shows as it is, checked.
        |s| match OPACITIES.iter().position(|&p| (s.opacity * 100.0).round() as u32 == p) {
            Some(at) => Current::At(at),
            None => Current::Other(percent((s.opacity * 100.0).round() as u32)),
        },
        |s, ix| s.opacity = OPACITIES[ix.min(OPACITIES.len() - 1)] as f32 / 100.0,
    ),
    toggle!("blur", blur),
    toggle!("liquid-glass", liquid_glass),
    toggle!("borders", borders),
    toggle!("column-dividers", column_dividers),
    toggle!("status-bar", status_bar),
    toggle!("icon-animations", icon_animations),
];
