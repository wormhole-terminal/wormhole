//! Reviewing and taking another terminal's settings: one terminal's proposal as rows a person ticks
//! (what it says, what Wormhole would set), what does not carry over and why, and the one apply that
//! saves the ticked palettes as theme files and sets the rest in a single settings update. Every rule is
//! core's (`import::scan_one`, `save_themes`, `apply`); the words are the catalogs' (the GPUI page's
//! `import/words.rs`, which core's enums map into here the same way).
//!
//! A PostScript font name (iTerm2, Terminal.app) is a face, not a family: only the system can say which
//! family it belongs to, so Swift passes a resolver and a font it cannot resolve stays under "Doesn't
//! carry over", as in the GPUI page.

use std::path::PathBuf;
use std::sync::Arc;

use wormhole_core::config::OptionAsMeta;
use wormhole_core::import::{
    self, Env, Found, Proposed, Reason, Setting, SettingValue, SkippedKind, Source, command_line, visible,
};
use wormhole_text::Key;
use wormhole_text::keys::{
    SETTINGS_IMPORT_REASON_CELL_WIDTH, SETTINGS_IMPORT_REASON_CONFIRM_CLOSE,
    SETTINGS_IMPORT_REASON_COPY_ON_SELECT, SETTINGS_IMPORT_REASON_CURSOR_BLINK,
    SETTINGS_IMPORT_REASON_CURSOR_SHAPE, SETTINGS_IMPORT_REASON_DYNAMIC_LUA,
    SETTINGS_IMPORT_REASON_ENVIRONMENT, SETTINGS_IMPORT_REASON_EXECUTES, SETTINGS_IMPORT_REASON_FONT_VARIANT,
    SETTINGS_IMPORT_REASON_INCLUDE_CYCLE, SETTINGS_IMPORT_REASON_INCLUDE_LIMIT,
    SETTINGS_IMPORT_REASON_KEYBINDING, SETTINGS_IMPORT_REASON_OLD_FORMAT, SETTINGS_IMPORT_REASON_OTHER,
    SETTINGS_IMPORT_REASON_OUTSIDE_HOME, SETTINGS_IMPORT_REASON_PADDING,
    SETTINGS_IMPORT_REASON_PARTIAL_PALETTE, SETTINGS_IMPORT_REASON_UNREADABLE,
    SETTINGS_IMPORT_REASON_UNSUPPORTED_VALUE, SETTINGS_IMPORT_REASON_WORKING_DIRECTORY,
    SETTINGS_IMPORT_SETTING_BELL_SOUND, SETTINGS_IMPORT_SETTING_BELL_VISUAL, SETTINGS_IMPORT_SETTING_BLUR,
    SETTINGS_IMPORT_SETTING_DEFAULT_SHELL, SETTINGS_IMPORT_SETTING_FONT_FAMILY,
    SETTINGS_IMPORT_SETTING_FONT_SIZE, SETTINGS_IMPORT_SETTING_LIGATURES,
    SETTINGS_IMPORT_SETTING_LINE_HEIGHT, SETTINGS_IMPORT_SETTING_OPACITY,
    SETTINGS_IMPORT_SETTING_OPTION_AS_META, SETTINGS_IMPORT_SETTING_SCROLLBACK,
    SETTINGS_IMPORT_SETTING_SHELL_PROFILE, SETTINGS_IMPORT_SETTING_THEME, SETTINGS_IMPORT_SETTING_THEME_DARK,
    SETTINGS_IMPORT_SETTING_THEME_LIGHT, SETTINGS_IMPORT_VALUE_META_BOTH, SETTINGS_IMPORT_VALUE_META_LEFT,
    SETTINGS_IMPORT_VALUE_META_RIGHT, SETTINGS_IMPORT_VALUE_OFF, SETTINGS_IMPORT_VALUE_ON,
    settings_import_count, settings_import_failed, settings_import_reason_built_in_theme,
    settings_import_reason_font_not_installed, settings_import_summary,
};
use wormhole_text::{text, whole};

use super::error::SettingsError;
use super::handle::SettingsHandle;
use crate::isolation;

/// What Swift implements to turn a PostScript font name into its family (CoreText), or `None`.
#[uniffi::export(with_foreign)]
pub trait FontFamilyResolver: Send + Sync {
    fn family(&self, postscript_name: String) -> Option<String>;
}

/// One setting a terminal would bring over.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsImportItem {
    /// What the setting is called here (`Font size`; the dark palette says so when there is a pair).
    pub label: String,
    /// What the other terminal's file says, spelled out where it holds anything invisible.
    pub from: String,
    /// What Wormhole would set.
    pub value: String,
    /// A shell profile: the whole command, which runs in every new terminal.
    pub command: Option<String>,
    pub default_on: bool,
}

/// A key of the other terminal's settings that is not taken, and why.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsImportSkip {
    pub key: String,
    pub reason: String,
}

/// One terminal, opened: the files read, what it would bring and what it would not.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsImportReview {
    pub id: String,
    pub app: String,
    pub count: String,
    pub files: Vec<String>,
    pub items: Vec<SettingsImportItem>,
    pub skipped: Vec<SettingsImportSkip>,
}

/// What an apply did: the line that says so.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsImportDone {
    pub brought: u32,
    pub summary: String,
}

fn setting_label(setting: Setting, pair: bool) -> &'static str {
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

/// A number without a trailing `.0` (`13`, `1.2`, `0.85`).
fn number_text(number: f32) -> String {
    format!("{}", (number * 100.0).round() / 100.0)
}

fn value_text(value: &SettingValue) -> String {
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

fn reason_text(reason: &Reason) -> String {
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

fn found_in(env: &Env, source: &str, fonts: &Arc<dyn FontFamilyResolver>) -> Option<(Source, Found)> {
    let source = Source::from_key(source)?;
    let mut found = import::scan_one(env, source)?;
    found.proposal.resolve_fonts(|name| fonts.family(name.to_string()));
    Some((source, found))
}

fn env_of(home: Option<String>) -> Option<Env> {
    match home {
        Some(home) => Some(Env::new(PathBuf::from(home))),
        None => Env::from_process(),
    }
}

fn review_of(source: Source, found: &Found) -> SettingsImportReview {
    let pair = found.proposal.get(Setting::ThemeLight).is_some();
    SettingsImportReview {
        id: source.key().to_string(),
        app: text(source.name_key()).to_string(),
        count: settings_import_count(whole(found.proposal.values.len())),
        files: found.files.iter().map(|file| file.to_string_lossy().into_owned()).collect(),
        items: found
            .proposal
            .values
            .iter()
            .map(|proposed| SettingsImportItem {
                label: setting_label(proposed.setting, pair).to_string(),
                from: visible(&proposed.from),
                value: value_text(&proposed.value),
                command: match &proposed.value {
                    SettingValue::Profile(profile) => Some(command_line(profile)),
                    _ => None,
                },
                default_on: proposed.default_on,
            })
            .collect(),
        skipped: found
            .proposal
            .skipped
            .iter()
            .map(|skipped| SettingsImportSkip {
                key: visible(&skipped.key),
                reason: reason_text(&skipped.reason),
            })
            .collect(),
    }
}

/// One terminal's proposal for the page to open (its settings, resolved fonts and all). Reads files:
/// off the main thread. `None` when the terminal has no settings under `home`.
#[uniffi::export]
pub fn settings_import_review(
    home: Option<String>,
    source: String,
    fonts: Arc<dyn FontFamilyResolver>,
) -> Option<SettingsImportReview> {
    let env = env_of(home)?;
    found_in(&env, &source, &fonts).map(|(source, found)| review_of(source, &found))
}

#[uniffi::export]
impl SettingsHandle {
    /// Brings the `ticked` rows (indexes into the review's items) of `source` over, in one settings update:
    /// a palette is saved as a theme file in this profile's folder first. The terminal's files are read
    /// again; a proposal that no longer has the rows that were shown is refused, so a tick never lands on
    /// another row.
    pub fn import_apply(
        &self,
        home: Option<String>,
        source: String,
        ticked: Vec<u32>,
        shown: u32,
        fonts: Arc<dyn FontFamilyResolver>,
    ) -> Result<SettingsImportDone, SettingsError> {
        isolation::require()?;
        let failed = |why: String| SettingsError::Invalid { reason: settings_import_failed(&why) };
        let env = env_of(home).ok_or_else(|| failed("no home folder".into()))?;
        let (_, found) =
            found_in(&env, &source, &fonts).ok_or_else(|| failed("its settings are gone".into()))?;
        let proposal = &found.proposal;
        if proposal.values.len() != shown as usize {
            return Err(failed("its settings changed: scan again".into()));
        }
        let chosen: Vec<Proposed> =
            ticked.iter().filter_map(|ix| proposal.values.get(*ix as usize)).cloned().collect();
        let saved = import::save_themes(&self.paths.themes_dir(), &chosen).map_err(failed)?;
        self.shared.lock().update(|settings| import::apply(settings, proposal, &chosen, &saved));
        let left = proposal.values.len().saturating_sub(chosen.len()) + proposal.skipped.len();
        Ok(SettingsImportDone {
            brought: u32::try_from(chosen.len()).unwrap_or(u32::MAX),
            summary: settings_import_summary(whole(chosen.len()), &left.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use wormhole_core::import::Env;
    use wormhole_core::storage::paths::{DataRoot, ProfileId};

    use super::{FontFamilyResolver, found_in, review_of};
    use crate::settings::handle::{SettingsHandle, SettingsListener};

    struct NoFonts;
    impl FontFamilyResolver for NoFonts {
        fn family(&self, _: String) -> Option<String> {
            None
        }
    }
    struct Quiet;
    impl SettingsListener for Quiet {
        fn wake(&self) {}
    }

    #[test]
    fn the_ticked_rows_of_a_found_terminal_are_set_and_the_rest_left() {
        let home = tempfile::tempdir().expect("temp home");
        let dir = home.path().join(".config/ghostty");
        std::fs::create_dir_all(&dir).expect("folder");
        crate::settings::tests::write_file(
            &dir.join("config"),
            "font-size = 15\nbackground-opacity = 0.9\ncursor-style = bar\n",
        );
        let fonts: Arc<dyn FontFamilyResolver> = Arc::new(NoFonts);
        let env = Env::new(home.path().to_path_buf());
        let (source, found) = found_in(&env, "ghostty", &fonts).expect("found");
        let review = review_of(source, &found);
        let size = review.items.iter().position(|item| item.label == "Font size").expect("font size row");
        assert_eq!(review.items[size].value, "15");
        assert!(!review.skipped.is_empty(), "the cursor style has no setting here: {review:?}");

        let data = tempfile::tempdir().expect("temp data dir");
        let handle = SettingsHandle::open_at(
            &DataRoot::new(data.path()),
            &ProfileId::default_profile(),
            Arc::new(Quiet),
        );
        let done = handle
            .import_apply(
                Some(home.path().to_string_lossy().into_owned()),
                "ghostty".into(),
                vec![size as u32],
                review.items.len() as u32,
                fonts.clone(),
            )
            .expect("applied");
        assert_eq!(done.brought, 1);
        assert!(handle.values().contains("\"font_size\":15"), "{}", handle.values());
        let stale = handle.import_apply(
            Some(home.path().to_string_lossy().into_owned()),
            "ghostty".into(),
            vec![0],
            99,
            fonts,
        );
        assert!(stale.is_err(), "a proposal that is not the one shown is refused");
    }
}
