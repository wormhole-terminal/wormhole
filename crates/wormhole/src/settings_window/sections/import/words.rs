//! The Import page's words: which catalog message says each setting, value, reason and comparison.
//! Core hands over enums and ids (it writes no sentence); each is mapped to its key here, and a test
//! walks the data so a new id without a message fails.

use wormhole_core::config::OptionAsMeta;
use wormhole_core::import::{
    Comparison, Reason, Setting, SettingValue, SkippedKind, Source, command_line, visible,
};
use wormhole_text::Key;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

/// Whose comparison a card shows: a terminal's, or Wormhole's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Subject {
    Terminal(Source),
    Wormhole,
}

impl Subject {
    /// The name its messages carry (`settings-import-gap-<key>-<id>`).
    fn key(self) -> &'static str {
        match self {
            Subject::Terminal(source) => source.key(),
            Subject::Wormhole => "wormhole",
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Subject::Terminal(source) => crate::settings::import::terminal_name(source),
            Subject::Wormhole => text(SETTINGS_IMPORT_TERMINAL_WORMHOLE),
        }
    }
}

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

/// The one-line strength of a subject.
pub(super) fn strength_text(subject: Subject, comparison: &Comparison) -> Option<&'static str> {
    let key = match (subject.key(), comparison.strength) {
        ("ghostty", "native_tabs_graphics") => SETTINGS_IMPORT_STRENGTH_GHOSTTY_NATIVE_TABS_GRAPHICS,
        ("iterm2", "broadest_features") => SETTINGS_IMPORT_STRENGTH_ITERM2_BROADEST_FEATURES,
        ("terminal-app", "built_into_macos") => SETTINGS_IMPORT_STRENGTH_TERMINAL_APP_BUILT_INTO_MACOS,
        ("alacritty", "minimal_fast") => SETTINGS_IMPORT_STRENGTH_ALACRITTY_MINIMAL_FAST,
        ("kitty", "fast_scriptable") => SETTINGS_IMPORT_STRENGTH_KITTY_FAST_SCRIPTABLE,
        ("wezterm", "multiplexer_lua") => SETTINGS_IMPORT_STRENGTH_WEZTERM_MULTIPLEXER_LUA,
        ("warp", "agent_focused") => SETTINGS_IMPORT_STRENGTH_WARP_AGENT_FOCUSED,
        ("zed", "terminal_in_editor") => SETTINGS_IMPORT_STRENGTH_ZED_TERMINAL_IN_EDITOR,
        ("vscode", "terminal_in_editor") => SETTINGS_IMPORT_STRENGTH_VSCODE_TERMINAL_IN_EDITOR,
        ("wormhole", "agent_sidebar") => SETTINGS_IMPORT_STRENGTH_WORMHOLE_AGENT_SIDEBAR,
        _ => return None,
    };
    Some(text(key))
}

/// A drawback's sentence.
pub(super) fn gap_text(subject: Subject, id: &str) -> Option<&'static str> {
    let key = match (subject.key(), id) {
        ("ghostty", "no_session_persistence") => SETTINGS_IMPORT_GAP_GHOSTTY_NO_SESSION_PERSISTENCE,
        ("ghostty", "agent_status_not_shown") => SETTINGS_IMPORT_GAP_GHOSTTY_AGENT_STATUS_NOT_SHOWN,
        ("ghostty", "command_notify_off") => SETTINGS_IMPORT_GAP_GHOSTTY_COMMAND_NOTIFY_OFF,
        ("ghostty", "no_sixel") => SETTINGS_IMPORT_GAP_GHOSTTY_NO_SIXEL,
        ("ghostty", "config_file_only") => SETTINGS_IMPORT_GAP_GHOSTTY_CONFIG_FILE_ONLY,
        ("iterm2", "quit_ends_programs") => SETTINGS_IMPORT_GAP_ITERM2_QUIT_ENDS_PROGRAMS,
        ("iterm2", "ligatures_disable_gpu") => SETTINGS_IMPORT_GAP_ITERM2_LIGATURES_DISABLE_GPU,
        ("iterm2", "ai_setup_required") => SETTINGS_IMPORT_GAP_ITERM2_AI_SETUP_REQUIRED,
        ("terminal-app", "no_kitty_keyboard") => SETTINGS_IMPORT_GAP_TERMINAL_APP_NO_KITTY_KEYBOARD,
        ("terminal-app", "no_inline_images") => SETTINGS_IMPORT_GAP_TERMINAL_APP_NO_INLINE_IMAGES,
        ("terminal-app", "no_agent_status") => SETTINGS_IMPORT_GAP_TERMINAL_APP_NO_AGENT_STATUS,
        ("alacritty", "no_tabs_or_splits") => SETTINGS_IMPORT_GAP_ALACRITTY_NO_TABS_OR_SPLITS,
        ("alacritty", "no_images") => SETTINGS_IMPORT_GAP_ALACRITTY_NO_IMAGES,
        ("alacritty", "no_ligatures") => SETTINGS_IMPORT_GAP_ALACRITTY_NO_LIGATURES,
        ("alacritty", "config_file_only") => SETTINGS_IMPORT_GAP_ALACRITTY_CONFIG_FILE_ONLY,
        ("alacritty", "no_agent_status") => SETTINGS_IMPORT_GAP_ALACRITTY_NO_AGENT_STATUS,
        ("kitty", "no_session_persistence") => SETTINGS_IMPORT_GAP_KITTY_NO_SESSION_PERSISTENCE,
        ("kitty", "gui_launch_env") => SETTINGS_IMPORT_GAP_KITTY_GUI_LAUNCH_ENV,
        ("kitty", "no_sixel") => SETTINGS_IMPORT_GAP_KITTY_NO_SIXEL,
        ("kitty", "config_file_only") => SETTINGS_IMPORT_GAP_KITTY_CONFIG_FILE_ONLY,
        ("wezterm", "no_release_since_2024") => SETTINGS_IMPORT_GAP_WEZTERM_NO_RELEASE_SINCE_2024,
        ("wezterm", "persistence_needs_setup") => SETTINGS_IMPORT_GAP_WEZTERM_PERSISTENCE_NEEDS_SETUP,
        ("wezterm", "kitty_keyboard_off") => SETTINGS_IMPORT_GAP_WEZTERM_KITTY_KEYBOARD_OFF,
        ("wezterm", "config_is_lua") => SETTINGS_IMPORT_GAP_WEZTERM_CONFIG_IS_LUA,
        ("wezterm", "macos_26_reports") => SETTINGS_IMPORT_GAP_WEZTERM_MACOS_26_REPORTS,
        ("warp", "programs_end_on_relaunch") => SETTINGS_IMPORT_GAP_WARP_PROGRAMS_END_ON_RELAUNCH,
        ("warp", "telemetry_for_free_ai") => SETTINGS_IMPORT_GAP_WARP_TELEMETRY_FOR_FREE_AI,
        ("warp", "login_for_some_features") => SETTINGS_IMPORT_GAP_WARP_LOGIN_FOR_SOME_FEATURES,
        ("warp", "offline_requested") => SETTINGS_IMPORT_GAP_WARP_OFFLINE_REQUESTED,
        ("warp", "no_sixel") => SETTINGS_IMPORT_GAP_WARP_NO_SIXEL,
        ("zed", "editor_panel") => SETTINGS_IMPORT_GAP_ZED_EDITOR_PANEL,
        ("zed", "no_images") => SETTINGS_IMPORT_GAP_ZED_NO_IMAGES,
        ("zed", "no_kitty_keyboard") => SETTINGS_IMPORT_GAP_ZED_NO_KITTY_KEYBOARD,
        ("zed", "terminal_ligatures") => SETTINGS_IMPORT_GAP_ZED_TERMINAL_LIGATURES,
        ("zed", "telemetry_on") => SETTINGS_IMPORT_GAP_ZED_TELEMETRY_ON,
        ("vscode", "editor_panel") => SETTINGS_IMPORT_GAP_VSCODE_EDITOR_PANEL,
        ("vscode", "programs_relaunched") => SETTINGS_IMPORT_GAP_VSCODE_PROGRAMS_RELAUNCHED,
        ("vscode", "images_off") => SETTINGS_IMPORT_GAP_VSCODE_IMAGES_OFF,
        ("vscode", "scrollback_1000") => SETTINGS_IMPORT_GAP_VSCODE_SCROLLBACK_1000,
        ("vscode", "telemetry_on") => SETTINGS_IMPORT_GAP_VSCODE_TELEMETRY_ON,
        ("wormhole", "macos_only") => SETTINGS_IMPORT_GAP_WORMHOLE_MACOS_ONLY,
        ("wormhole", "young_project") => SETTINGS_IMPORT_GAP_WORMHOLE_YOUNG_PROJECT,
        ("wormhole", "programs_end_on_quit") => SETTINGS_IMPORT_GAP_WORMHOLE_PROGRAMS_END_ON_QUIT,
        ("wormhole", "voiceover_no_terminal_text") => SETTINGS_IMPORT_GAP_WORMHOLE_VOICEOVER_NO_TERMINAL_TEXT,
        ("wormhole", "kitty_keyboard_partial") => SETTINGS_IMPORT_GAP_WORMHOLE_KITTY_KEYBOARD_PARTIAL,
        _ => return None,
    };
    Some(text(key))
}

#[cfg(test)]
mod tests {
    use wormhole_core::import::{WORMHOLE, comparison};

    use super::*;

    #[test]
    fn every_strength_and_drawback_has_its_sentence() {
        let subjects = Source::ALL
            .into_iter()
            .map(|source| (Subject::Terminal(source), comparison(source)))
            .chain([(Subject::Wormhole, &WORMHOLE)]);
        for (subject, comparison) in subjects {
            assert!(strength_text(subject, comparison).is_some_and(|t| !t.is_empty()), "{:?}", subject);
            for drawback in comparison.drawbacks {
                let said = gap_text(subject, drawback.id);
                assert!(said.is_some_and(|t| !t.is_empty()), "{:?} {}", subject, drawback.id);
            }
        }
    }

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
