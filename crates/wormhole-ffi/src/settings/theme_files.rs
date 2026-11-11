//! The theme files of a profile, from the Appearance page's cards: Import theme (a file copied into the profile's
//! themes folder and selected, core's `Config::import_theme`) and Move to Trash for a theme the person imported
//! (the picks that name its file go back to the defaults first, as the GPUI app's `theme_trash` does; Swift moves
//! the file, and puts the picks back with `restore_theme_picks` when that fails).

use wormhole_core::theme::registry::{DEFAULT_THEME, DEFAULT_THEME_LIGHT, UserThemes};
use wormhole_text::keys::{
    SETTINGS_APPEARANCE_TRASH_DETAIL, settings_appearance_trash_both, settings_appearance_trash_dark,
    settings_appearance_trash_light, settings_appearance_trash_message,
};
use wormhole_text::text;

use super::error::SettingsError;
use super::handle::SettingsHandle;
use crate::isolation;

/// What the question before moving a theme to the Trash says, and where its file is.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ThemeTrashPlan {
    pub path: String,
    /// "Move “Mine” to the Trash?"
    pub message: String,
    /// "You can get it back from the Trash. Your dark theme changes to …"
    pub detail: String,
    /// The picks that name it, which the move changes.
    pub dark: bool,
    pub light: bool,
}

#[uniffi::export]
impl SettingsHandle {
    /// Copies the theme file at `path` into this profile's themes folder and selects it; a file that is no theme is
    /// refused with its reason. The folder is the profile's, so a file picked in a scratch folder is only read.
    pub fn import_theme(&self, path: String) -> Result<(), SettingsError> {
        isolation::require()?;
        self.shared
            .lock()
            .import_theme(std::path::Path::new(&path))
            .map_err(|reason| SettingsError::Invalid { reason })
    }

    /// The question for moving the user theme `value` (its slug, name or file) to the Trash, `None` when it is no
    /// theme of this profile's folder (a built-in has no file).
    pub fn theme_trash_plan(&self, value: String) -> Option<ThemeTrashPlan> {
        let themes = UserThemes::read(&self.paths.themes_dir());
        let theme = themes.named(&value).or_else(|| themes.by_path(&value))?;
        let (dark, light) = self.picks_naming(&themes, &theme.slug);
        let mut parts = vec![text(SETTINGS_APPEARANCE_TRASH_DETAIL).to_string()];
        match (dark, light) {
            (true, true) => parts.push(settings_appearance_trash_both(DEFAULT_THEME, DEFAULT_THEME_LIGHT)),
            (true, false) => parts.push(settings_appearance_trash_dark(DEFAULT_THEME)),
            (false, true) => parts.push(settings_appearance_trash_light(DEFAULT_THEME_LIGHT)),
            (false, false) => {}
        }
        Some(ThemeTrashPlan {
            path: theme.path.to_string_lossy().into_owned(),
            message: settings_appearance_trash_message(&theme.name),
            detail: parts.join(" "),
            dark,
            light,
        })
    }

    /// The picks that name theme `value` go to the defaults of their appearance, so the app never falls back with an
    /// error once the file is gone. Returns what they were (`theme`, `theme_light`) for `restore_theme_picks`.
    pub fn reset_theme_picks(&self, value: String) -> Vec<String> {
        let themes = UserThemes::read(&self.paths.themes_dir());
        let Some(slug) = themes.named(&value).or_else(|| themes.by_path(&value)).map(|t| t.slug.clone())
        else {
            return Vec::new();
        };
        let (dark, light) = self.picks_naming(&themes, &slug);
        let before = {
            let state = self.shared.lock();
            vec![state.effective().theme.clone(), state.effective().theme_light.clone()]
        };
        self.shared.lock().update(|s| {
            if dark {
                s.theme = DEFAULT_THEME.into();
            }
            if light {
                s.theme_light = DEFAULT_THEME_LIGHT.into();
            }
        });
        before
    }

    /// Puts `theme` and `theme_light` back to what `reset_theme_picks` returned (the move to the Trash failed).
    pub fn restore_theme_picks(&self, before: Vec<String>) {
        if let [theme, light] = before.as_slice() {
            self.shared.lock().update(|s| {
                s.theme.clone_from(theme);
                s.theme_light.clone_from(light);
            });
        }
    }
}

impl SettingsHandle {
    /// Which of the two picks name the user theme with `slug`.
    fn picks_naming(&self, themes: &UserThemes, slug: &str) -> (bool, bool) {
        let names = |value: &str| {
            themes.named(value).or_else(|| themes.by_path(value)).is_some_and(|t| t.slug == slug)
        };
        let state = self.shared.lock();
        (names(&state.effective().theme), names(&state.effective().theme_light))
    }
}
