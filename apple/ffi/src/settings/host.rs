//! What the select rows list and where the buttons that open things point, from core: the shells
//! (`terminal::shells::choices`), the icon themes on disk (`icon_theme::discover`) and the folders of
//! the profile and the data folder. Swift opens a folder (Finder is the platform's) and lists the
//! apps (`open_in` needs the system's list of installed ones).

use wormhole_core::terminal::shells::{self, ShellKind};
use wormhole_core::theme::icon_theme;
use wormhole_text::keys::settings_terminal_login_shell;

use super::handle::SettingsHandle;

/// The value `icon_theme` holds to show no file icons (stored, not interface text).
const NO_ICONS: &str = "None";

/// One choice of a select row: the value the file stores and the words shown for it.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsOptionRow {
    pub value: String,
    pub label: String,
}

/// Folders and files a button opens, as absolute paths.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsPaths {
    pub config_file: String,
    pub data_dir: String,
    pub logs_dir: String,
    pub fonts_dir: String,
    pub icon_themes_dir: String,
}

fn text(path: impl AsRef<std::path::Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
}

#[uniffi::export]
impl SettingsHandle {
    /// The Default shell select: the login shell, the profiles of `config.json` and the shells on this
    /// Mac, in that order. The login shell's value is the empty string.
    pub fn shell_options(&self) -> Vec<SettingsOptionRow> {
        let choices = shells::choices(self.shared.lock().effective());
        choices
            .into_iter()
            .map(|choice| SettingsOptionRow {
                label: match choice.kind {
                    ShellKind::Login => settings_terminal_login_shell(&choice.command_line),
                    ShellKind::Profile | ShellKind::System => choice.command_line,
                },
                value: choice.key,
            })
            .collect()
    }

    /// The File icon theme select: the bundled theme, the ones in the profile's `icon-themes` folder,
    /// and none.
    pub fn icon_theme_options(&self) -> Vec<SettingsOptionRow> {
        let mut names = vec![icon_theme::builtin_family().name.clone()];
        names.extend(icon_theme::discover(&self.paths.icon_themes_dir()).into_iter().map(|(name, _)| name));
        names.dedup();
        names.push(NO_ICONS.into());
        names.into_iter().map(|name| SettingsOptionRow { value: name.clone(), label: name }).collect()
    }

    /// Where the Open buttons go.
    pub fn paths(&self) -> SettingsPaths {
        SettingsPaths {
            config_file: text(self.paths.config_file()),
            data_dir: text(self.root.base()),
            logs_dir: text(self.root.logs_dir()),
            fonts_dir: text(self.paths.fonts_dir()),
            icon_themes_dir: text(self.paths.icon_themes_dir()),
        }
    }
}
