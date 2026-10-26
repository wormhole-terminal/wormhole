//! Taking a proposal: the ticked values into [`Settings`], and an imported palette into a theme file.
//!
//! The app calls the two in order, the second inside one `Config::update`:
//!
//! ```ignore
//! let saved = import::save_themes(&themes_dir, &chosen)?;
//! config.update(|settings| import::apply(settings, &chosen, &saved));
//! ```
//!
//! Values are already in Wormhole's ranges, and `Config::update` clamps once more.

use std::path::Path;

use super::proposal::{Proposed, Setting, SettingValue};
use crate::config::{Config, Settings};
use crate::theme::Theme;

/// Where the palettes of the chosen values were saved, as `theme` and `theme_light` name them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SavedThemes {
    pub dark: Option<String>,
    pub light: Option<String>,
}

/// Save each chosen palette as a theme file in `themes` (a profile's themes folder). A name that is
/// taken by a different file gets a new one, and the same palette saved again reuses its file.
pub fn save_themes(themes: &Path, chosen: &[Proposed]) -> Result<SavedThemes, String> {
    let mut saved = SavedThemes::default();
    for proposed in chosen {
        let SettingValue::Palette(theme) = &proposed.value else { continue };
        let path = Config::save_palette_to(themes, &theme.name, &ghostty_text(theme))?;
        let value = path.to_string_lossy().into_owned();
        match proposed.setting {
            Setting::Theme => saved.dark = Some(value),
            Setting::ThemeLight => saved.light = Some(value),
            _ => {}
        }
    }
    Ok(saved)
}

/// Set what `chosen` says. A palette is set from `saved`; call [`save_themes`] first.
pub fn apply(settings: &mut Settings, chosen: &[Proposed], saved: &SavedThemes) {
    for proposed in chosen {
        match (&proposed.setting, &proposed.value) {
            (Setting::FontFamily, SettingValue::Font(font)) if !font.family().trim().is_empty() => {
                settings.font_family = font.family().trim().to_string();
            }
            (Setting::FontSize, SettingValue::Number(points)) => settings.font_size = *points,
            (Setting::LineHeight, SettingValue::Number(multiplier)) => settings.line_height = *multiplier,
            (Setting::Ligatures, SettingValue::Flag(on)) => settings.font_ligatures = *on,
            (Setting::Opacity, SettingValue::Number(opacity)) => settings.opacity = *opacity,
            (Setting::Blur, SettingValue::Flag(on)) => settings.blur = *on,
            (Setting::OptionAsMeta, SettingValue::Meta(meta)) => settings.option_as_meta = *meta,
            (Setting::ScrollbackLines, SettingValue::Count(lines)) => settings.scrollback_lines = *lines,
            (Setting::BellSound, SettingValue::Flag(on)) => settings.bell_sound = *on,
            (Setting::BellVisual, SettingValue::Flag(on)) => settings.bell_visual = *on,
            (Setting::DefaultShell, SettingValue::Shell(path)) => settings.default_shell = path.clone(),
            (Setting::ShellProfile, SettingValue::Profile(profile)) => {
                // A profile of the same name is replaced: the name is the key a terminal remembers.
                settings.shells.retain(|existing| existing.name != profile.name);
                settings.shells.push(profile.clone());
            }
            (Setting::Theme, SettingValue::Palette(_)) => {
                if let Some(path) = &saved.dark {
                    settings.theme = path.clone();
                }
            }
            (Setting::ThemeLight, SettingValue::Palette(_)) => {
                if let Some(path) = &saved.light {
                    settings.theme_light = path.clone();
                }
            }
            _ => {}
        }
    }
}

/// A palette in Ghostty's theme format, which the theme folder reads as a file without an
/// extension.
fn ghostty_text(theme: &Theme) -> String {
    let hex = |color: u32| format!("#{color:06x}");
    let mut text = String::new();
    for (i, color) in theme.palette.iter().enumerate() {
        text.push_str(&format!("palette = {i}={}\n", hex(*color)));
    }
    text.push_str(&format!(
        "background = {}\nforeground = {}\n",
        hex(theme.background),
        hex(theme.foreground)
    ));
    text.push_str(&format!("cursor-color = {}\n", hex(theme.cursor)));
    if let Some(color) = theme.cursor_text {
        text.push_str(&format!("cursor-text = {}\n", hex(color)));
    }
    text.push_str(&format!("selection-background = {}\n", hex(theme.selection_background)));
    if let Some(color) = theme.selection_foreground {
        text.push_str(&format!("selection-foreground = {}\n", hex(color)));
    }
    text
}

#[cfg(test)]
pub(super) fn palette_text(theme: &Theme) -> String {
    ghostty_text(theme)
}
