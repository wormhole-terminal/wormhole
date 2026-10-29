//! Which theme a pick names, and what choosing a theme sets: one rule for the theme picker and
//! the Theme Studio. Pure, so it is tested without a window.

use wormhole_core::config::{Settings, ThemeMode};
use wormhole_core::theme::tokens::Appearance;

/// Set `value`, a theme of the `appearance` kind, as the pick the mode says a choice sets: Dark
/// mode the dark pick, Light mode the light one, and under Match system the pick of the theme's
/// own kind.
pub(crate) fn set_pick(s: &mut Settings, value: String, appearance: Appearance) {
    let light = match s.theme_mode {
        ThemeMode::Dark => false,
        ThemeMode::Light => true,
        ThemeMode::System => appearance == Appearance::Light,
    };
    if light { s.theme_light = value } else { s.theme = value }
}

/// Whether the theme is already a pick, either one: `selects` says whether a pick's value names
/// it (the catalog's rule: a name, a path or a slug).
pub(crate) fn is_pick(s: &Settings, selects: &dyn Fn(&str) -> bool) -> bool {
    selects(&s.theme) || selects(&s.theme_light)
}

/// What saving a theme does to the picks: nothing for a theme that is a pick already, else it
/// becomes the pick of its own appearance by [`set_pick`].
pub(crate) fn pick_after_save(
    s: &mut Settings,
    value: String,
    appearance: Appearance,
    selects: &dyn Fn(&str) -> bool,
) {
    if !is_pick(s, selects) {
        set_pick(s, value, appearance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(mode: ThemeMode) -> Settings {
        Settings {
            theme_mode: mode,
            theme: "dark-pick".into(),
            theme_light: "light-pick".into(),
            ..Settings::default()
        }
    }

    #[test]
    fn a_choice_sets_the_pick_the_mode_says() {
        let table = [
            (ThemeMode::Dark, Appearance::Light, "mine", "light-pick"),
            (ThemeMode::Light, Appearance::Dark, "dark-pick", "mine"),
            (ThemeMode::System, Appearance::Dark, "mine", "light-pick"),
            (ThemeMode::System, Appearance::Light, "dark-pick", "mine"),
        ];
        for (mode, appearance, dark, light) in table {
            let mut s = settings(mode);
            set_pick(&mut s, "mine".into(), appearance);
            assert_eq!((s.theme.as_str(), s.theme_light.as_str()), (dark, light), "{mode:?} {appearance:?}");
        }
    }

    #[test]
    fn saving_a_theme_that_is_a_pick_changes_no_pick() {
        for mode in [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System] {
            for appearance in [Appearance::Dark, Appearance::Light] {
                let mut s = settings(mode);
                pick_after_save(&mut s, "mine".into(), appearance, &|v| v == "light-pick");
                assert_eq!((s.theme.as_str(), s.theme_light.as_str()), ("dark-pick", "light-pick"));
            }
        }
    }

    #[test]
    fn saving_a_new_theme_sets_the_pick_of_its_own_appearance() {
        let mut s = settings(ThemeMode::System);
        pick_after_save(&mut s, "mine".into(), Appearance::Light, &|_| false);
        assert_eq!((s.theme.as_str(), s.theme_light.as_str()), ("dark-pick", "mine"));
    }
}
