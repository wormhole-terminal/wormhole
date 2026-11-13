//! What choosing, saving and renaming a theme does to the two picks (`theme`, `theme_light`) and the mode: core's
//! rule (`config::picks`), shared with the GPUI app, so the Theme Studio, the onboarding's Look step and a
//! Settings card all set the same thing. Nothing here moves the mode but Match system's own button.

use std::path::Path;

use wormhole_core::config::{ThemeMode, picks};
use wormhole_core::theme::registry::{ThemeId, UserThemes};
use wormhole_core::theme::tokens::Appearance;

use super::handle::SettingsHandle;

fn appearance(dark: bool) -> Appearance {
    if dark { Appearance::Dark } else { Appearance::Light }
}

fn same_file(a: &Path, b: &Path) -> bool {
    a == b || std::fs::canonicalize(a).ok().zip(std::fs::canonicalize(b).ok()).is_some_and(|(a, b)| a == b)
}

#[uniffi::export]
impl SettingsHandle {
    /// Whether the pick `setting` (a name, a slug or a path) names the theme saved in the file `path`: the file
    /// itself or, for a pair, either of its appearances.
    pub fn theme_selects(&self, setting: String, path: String) -> bool {
        let themes = UserThemes::read(&self.paths.themes_dir());
        selects(&themes, &setting, Path::new(&path))
    }

    /// Whether the theme picker leaves `value` (a theme as `theme` names it) out: core's rule, which counts a built-in's
    /// name and slug as one theme.
    pub fn theme_hidden(&self, value: String) -> bool {
        self.shared.lock().effective().theme_hidden(&value)
    }

    /// Leaves `value` out of the theme picker, or brings it back (core's `Settings::set_theme_hidden`). Whether anything
    /// changed.
    pub fn set_theme_hidden(&self, value: String, hidden: bool) -> bool {
        self.shared.lock().update(|s| {
            s.set_theme_hidden(&value, hidden);
        })
    }

    /// A theme of the kind `dark` or light was chosen (`value`: what a pick holds): it is the pick the mode says a
    /// choice sets. Whether a pick changed.
    pub fn pick_theme(&self, value: String, dark: bool) -> bool {
        self.shared.lock().update(|s| picks::set_pick(s, value, appearance(dark)))
    }

    /// A theme was saved in the file `path`: nothing for a theme that is a pick already, else it becomes the pick
    /// of its own kind. Whether a pick changed.
    pub fn pick_theme_after_save(&self, value: String, dark: bool, path: String) -> bool {
        let themes = UserThemes::read(&self.paths.themes_dir());
        let selects = |setting: &str| selects(&themes, setting, Path::new(&path));
        self.shared.lock().update(|s| picks::pick_after_save(s, value, appearance(dark), &selects))
    }

    /// A theme was renamed: a pick that is one of the names `renames` (flat pairs, old then new) changes to what
    /// replaces it. Picks that name a file or an id are not touched. Whether a pick changed.
    pub fn rename_theme_picks(&self, renames: Vec<String>) -> bool {
        self.shared.lock().update(|s| {
            for pick in [&mut s.theme, &mut s.theme_light] {
                if let Some(pair) = renames.chunks_exact(2).find(|pair| pick.eq_ignore_ascii_case(&pair[0])) {
                    pick.clone_from(&pair[1]);
                }
            }
        })
    }

    /// Use with Match system: the light pick is the pair's light appearance, the dark pick its dark one.
    pub fn follow_system_with(&self, light: String, dark: String) -> bool {
        self.shared.lock().update(|s| {
            s.theme_mode = ThemeMode::System;
            s.theme_light = light;
            s.theme = dark;
        })
    }

    /// The icon theme in force: the setting, or the bundled one while the Icon themes flag is off (core's rule).
    pub fn icon_theme_in_force(&self) -> String {
        self.shared.lock().effective().icon_theme_in_force().to_string()
    }
}

fn selects(themes: &UserThemes, setting: &str, path: &Path) -> bool {
    match themes.select_or_path(setting) {
        Some(ThemeId::File(file) | ThemeId::Variant(file, _)) => same_file(&file, path),
        _ => false,
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods, reason = "a test writing its own scratch theme folder")]
mod tests {
    use std::sync::Arc;

    use wormhole_core::config::ThemeMode;
    use wormhole_core::storage::paths::{DataRoot, ProfileId};

    use super::SettingsHandle;
    use crate::settings::SettingsListener;

    struct Quiet;
    impl SettingsListener for Quiet {
        fn wake(&self) {}
    }

    fn picks(handle: &SettingsHandle) -> (String, String, ThemeMode) {
        let state = handle.shared.lock();
        let s = state.effective();
        (s.theme.clone(), s.theme_light.clone(), s.theme_mode)
    }

    #[test]
    fn a_saved_theme_becomes_the_pick_of_its_kind_once_and_never_moves_the_mode() {
        let dir = tempfile::tempdir().expect("data");
        let handle = SettingsHandle::open_at(
            &DataRoot::new(dir.path()),
            &ProfileId::default_profile(),
            Arc::new(Quiet),
        );
        let themes = handle.paths.themes_dir();
        std::fs::create_dir_all(&themes).expect("themes");
        let file = themes.join("Mine.json");
        std::fs::write(&file, r##"{"name": "Mine", "extends": "Nord"}"##).expect("theme");
        let path = file.to_string_lossy().into_owned();
        let (dark, light, mode) = picks(&handle);

        assert!(
            handle.pick_theme_after_save(path.clone(), false, path.clone()),
            "a light theme, not a pick yet"
        );
        assert_eq!(picks(&handle), (dark.clone(), path.clone(), mode), "the light pick, the mode untouched");
        assert!(
            !handle.pick_theme_after_save(path.clone(), true, path.clone()),
            "already a pick: nothing changes"
        );
        assert!(
            handle.theme_selects("Mine".into(), path.clone())
                && !handle.theme_selects("Nord".into(), path.clone())
        );

        assert!(
            handle.rename_theme_picks(vec![path.clone(), "mine_id".into()]),
            "picks naming it by its file follow"
        );
        assert_eq!(picks(&handle).1, "mine_id");
        assert!(!handle.rename_theme_picks(vec!["Other".into(), "x".into()]));
        assert!(handle.pick_theme("Nord".into(), true));
        assert_eq!(picks(&handle).0, "Nord");
        assert!(handle.follow_system_with("L".into(), "D".into()));
        assert_eq!(picks(&handle), ("D".into(), "L".into(), ThemeMode::System));
        let _ = light;
    }
}
