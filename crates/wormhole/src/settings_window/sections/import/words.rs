//! The Import page's words: which catalog message says each setting, value and reason.
//! Core hands over enums (it writes no sentence); each is mapped to its key here, and the compiler
//! fails on a variant without a message.

use wormhole_core::config::OptionAsMeta;
use wormhole_core::import::{Reason, Setting, SettingValue, SkippedKind, command_line, visible};
use wormhole_text::Key;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

/// What a setting is called. `pair`: the proposal has a palette for each appearance, so the dark
/// one says so.
pub(super) fn setting_label(setting: Setting, pair: bool) -> &'static str {
    text(match setting {
        Setting::FontFamily => SETTINGS_IMPORT_SETTING_FONT_FAMILY,
        Setting::FontSize => SETTINGS_IMPORT_SETTING_FONT_SIZE,
        Setting::LineHeight => SETTINGS_IMPORT_SETTING_LINE_HEIGHT,
        Setting::Ligatures => SETTINGS_IMPORT_SETTING_LIGATURES,
        Setting::Theme if pair => SETTINGS_IMPORT_SETTING_THEME_DARK,
        Setting::Theme => SETTINGS_IMPORT_SETTING_THEME,
        Setting::ThemeLight => SETTINGS_IMPORT_SETTING_THEME_LIGHT,
        Setting::Opacity => SETTINGS_IMPORT_SETTING_OPACITY,
        Setting::Blur => SETTINGS_IMPORT_SETTING_BLUR,
        Setting::OptionAsMeta => SETTINGS_IMPORT_SETTING_OPTION_AS_META,
        Setting::ScrollbackLines => SETTINGS_IMPORT_SETTING_SCROLLBACK,
        Setting::BellSound => SETTINGS_IMPORT_SETTING_BELL_SOUND,
        Setting::BellVisual => SETTINGS_IMPORT_SETTING_BELL_VISUAL,
        Setting::DefaultShell => SETTINGS_IMPORT_SETTING_DEFAULT_SHELL,
        Setting::ShellProfile => SETTINGS_IMPORT_SETTING_SHELL_PROFILE,
    })
}

/// What Wormhole would set, as a short value; text from the other terminal's files is spelled out
/// where it holds anything invisible.
pub(super) fn value_text(value: &SettingValue) -> String {
    let flag = |on: bool| text(if on { SETTINGS_IMPORT_VALUE_ON } else { SETTINGS_IMPORT_VALUE_OFF });
    match value {
        SettingValue::Font(font) => visible(&font.name),
        SettingValue::Number(number) => number_text(*number),
        SettingValue::Count(lines) => lines.to_string(),
        SettingValue::Flag(on) => flag(*on).to_string(),
        SettingValue::Meta(meta) => match meta {
            OptionAsMeta::Off => text(SETTINGS_IMPORT_VALUE_OFF),
            OptionAsMeta::Left => text(SETTINGS_IMPORT_VALUE_META_LEFT),
            OptionAsMeta::Right => text(SETTINGS_IMPORT_VALUE_META_RIGHT),
            OptionAsMeta::Both => text(SETTINGS_IMPORT_VALUE_META_BOTH),
        }
        .to_string(),
        SettingValue::Palette(theme) => visible(&theme.name),
        SettingValue::Shell(path) => visible(path),
        SettingValue::Profile(profile) => command_line(profile),
    }
}

/// A number without a trailing `.0` (`13`, `1.2`, `0.85`).
fn number_text(number: f32) -> String {
    let rounded = (number * 100.0).round() / 100.0;
    format!("{rounded}")
}

/// Why a key of the other terminal's settings is not taken.
pub(super) fn reason_text(reason: &Reason) -> String {
    let said = |key: Key| text(key).to_string();
    match reason {
        Reason::NoSuchSetting(kind) => said(match kind {
            SkippedKind::CursorShape => SETTINGS_IMPORT_REASON_CURSOR_SHAPE,
            SkippedKind::CursorBlink => SETTINGS_IMPORT_REASON_CURSOR_BLINK,
            SkippedKind::Padding => SETTINGS_IMPORT_REASON_PADDING,
            SkippedKind::CopyOnSelect => SETTINGS_IMPORT_REASON_COPY_ON_SELECT,
            SkippedKind::ConfirmClose => SETTINGS_IMPORT_REASON_CONFIRM_CLOSE,
            SkippedKind::WorkingDirectory => SETTINGS_IMPORT_REASON_WORKING_DIRECTORY,
            SkippedKind::Keybinding => SETTINGS_IMPORT_REASON_KEYBINDING,
            SkippedKind::Environment => SETTINGS_IMPORT_REASON_ENVIRONMENT,
            SkippedKind::FontVariant => SETTINGS_IMPORT_REASON_FONT_VARIANT,
            SkippedKind::CellWidth => SETTINGS_IMPORT_REASON_CELL_WIDTH,
            SkippedKind::Other => SETTINGS_IMPORT_REASON_OTHER,
        }),
        Reason::UnsupportedValue => said(SETTINGS_IMPORT_REASON_UNSUPPORTED_VALUE),
        Reason::DynamicLua => said(SETTINGS_IMPORT_REASON_DYNAMIC_LUA),
        Reason::BuiltInTheme(name) => settings_import_reason_built_in_theme(&visible(name)),
        Reason::Executes => said(SETTINGS_IMPORT_REASON_EXECUTES),
        Reason::Unreadable => said(SETTINGS_IMPORT_REASON_UNREADABLE),
        Reason::IncludeCycle => said(SETTINGS_IMPORT_REASON_INCLUDE_CYCLE),
        Reason::IncludeLimit => said(SETTINGS_IMPORT_REASON_INCLUDE_LIMIT),
        Reason::OldFormat => said(SETTINGS_IMPORT_REASON_OLD_FORMAT),
        Reason::OutsideHome => said(SETTINGS_IMPORT_REASON_OUTSIDE_HOME),
        Reason::PartialPalette => said(SETTINGS_IMPORT_REASON_PARTIAL_PALETTE),
        Reason::FontNotInstalled(name) => settings_import_reason_font_not_installed(&visible(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_font_that_is_not_installed_is_named() {
        let said = reason_text(&Reason::FontNotInstalled("JetBrainsMono-Regular".into()));
        assert!(said.contains("JetBrainsMono-Regular") && said.contains("isn't installed"), "{said}");
    }

    #[test]
    fn a_number_drops_its_trailing_zero() {
        assert_eq!(number_text(13.0), "13");
        assert_eq!(number_text(1.2), "1.2");
        assert_eq!(number_text(0.85), "0.85");
    }
}
