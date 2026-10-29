//! Moving a theme file to the Trash while it may be a pick: the one rule for the Settings card and
//! the Theme Studio. A pick that names the file goes back to the appearance's default first (so the
//! app never falls back with an error), and goes back to what it was if the move fails. The
//! question that asks first is here too, so both say the same words.

use std::path::Path;

use gpui_kit::{App, PromptButton, PromptLevel, Window};
use wormhole_core::config::Settings;
use wormhole_core::theme::registry::{DEFAULT_THEME, DEFAULT_THEME_LIGHT};
use wormhole_text::keys::{
    SETTINGS_APPEARANCE_MOVE_TO_TRASH, SETTINGS_APPEARANCE_TRASH_DETAIL, SETTINGS_APPEARANCE_TRASH_EDITS,
    STUDIO_CANCEL, settings_appearance_trash_both, settings_appearance_trash_dark,
    settings_appearance_trash_light, settings_appearance_trash_message,
};
use wormhole_text::text;

use super::SettingsStore;

/// Moves a file to the Trash; a test passes one that touches nothing.
pub(crate) type Trash = fn(&Path) -> anyhow::Result<()>;

/// Which picks name the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Affected {
    pub(crate) dark: bool,
    pub(crate) light: bool,
}

impl Affected {
    pub(crate) fn any(self) -> bool {
        self.dark || self.light
    }
}

/// The picks `selects` says name the theme.
pub(crate) fn affected(s: &Settings, selects: &dyn Fn(&str) -> bool) -> Affected {
    Affected { dark: selects(&s.theme), light: selects(&s.theme_light) }
}

/// The picks of `which` go to the defaults of their appearance.
pub(crate) fn reset_to_defaults(s: &mut Settings, which: Affected) {
    if which.dark {
        s.theme = DEFAULT_THEME.into();
    }
    if which.light {
        s.theme_light = DEFAULT_THEME_LIGHT.into();
    }
}

/// What the question adds to "you can get it back": which picks change and to what, and that
/// unsaved edits (`edits`) are dropped.
pub(crate) fn detail(which: Affected, edits: bool) -> String {
    let mut parts = vec![text(SETTINGS_APPEARANCE_TRASH_DETAIL).to_string()];
    match (which.dark, which.light) {
        (true, true) => parts.push(settings_appearance_trash_both(DEFAULT_THEME, DEFAULT_THEME_LIGHT)),
        (true, false) => parts.push(settings_appearance_trash_dark(DEFAULT_THEME)),
        (false, true) => parts.push(settings_appearance_trash_light(DEFAULT_THEME_LIGHT)),
        (false, false) => {}
    }
    if edits {
        parts.push(text(SETTINGS_APPEARANCE_TRASH_EDITS).to_string());
    }
    parts.join(" ")
}

/// The sheet: "Move “name” to the Trash?". Resolves to `true` only for Move to Trash.
pub(crate) fn ask(
    name: &str,
    which: Affected,
    edits: bool,
    window: &mut Window,
    cx: &mut App,
) -> impl Future<Output = bool> + use<> {
    let buttons = [
        PromptButton::ok(text(SETTINGS_APPEARANCE_MOVE_TO_TRASH)),
        PromptButton::cancel(text(STUDIO_CANCEL)),
    ];
    let answer = window.prompt(
        PromptLevel::Warning,
        &settings_appearance_trash_message(name),
        Some(&detail(which, edits)),
        &buttons,
        cx,
    );
    async move { answer.await.ok() == Some(0) }
}

/// Move the file at `path` to the Trash. The picks that name it (`selects`) are set to the
/// defaults first; if the move fails they are put back and nothing has changed.
pub(crate) fn trash(
    cx: &mut App,
    path: &Path,
    selects: &dyn Fn(&str) -> bool,
    move_to_trash: Trash,
) -> anyhow::Result<()> {
    let before = cx.global::<SettingsStore>().settings.clone();
    let which = affected(&before, selects);
    if which.any() {
        SettingsStore::update(cx, |s| reset_to_defaults(s, which));
    }
    if let Err(error) = move_to_trash(path) {
        if which.any() {
            SettingsStore::update(cx, |s| {
                s.theme = before.theme.clone();
                s.theme_light = before.theme_light.clone();
            });
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use gpui_kit::TestAppContext;

    use super::*;

    fn settings() -> Settings {
        Settings { theme: "/t/mine.json".into(), theme_light: "Nord".into(), ..Settings::default() }
    }

    #[test]
    fn only_the_picks_that_name_the_file_go_back_to_the_defaults() {
        let mut s = settings();
        let which = affected(&s, &|v| v == "/t/mine.json");
        assert_eq!(which, Affected { dark: true, light: false });
        reset_to_defaults(&mut s, which);
        assert_eq!((s.theme.as_str(), s.theme_light.as_str()), (DEFAULT_THEME, "Nord"));
        let mut s = settings();
        let which = affected(&s, &|v| v == "Nord");
        reset_to_defaults(&mut s, which);
        assert_eq!((s.theme.as_str(), s.theme_light.as_str()), ("/t/mine.json", DEFAULT_THEME_LIGHT));
    }

    #[test]
    fn the_question_says_which_pick_changes_and_what_is_lost() {
        let dark = detail(Affected { dark: true, light: false }, false);
        assert!(dark.contains(DEFAULT_THEME) && !dark.contains(DEFAULT_THEME_LIGHT), "{dark}");
        let both = detail(Affected { dark: true, light: true }, true);
        assert!(both.contains(DEFAULT_THEME) && both.contains(DEFAULT_THEME_LIGHT), "{both}");
        assert!(both.contains("unsaved"), "{both}");
        assert_eq!(detail(Affected::default(), false), "You can get it back from the Trash.");
    }

    #[gpui_kit::test]
    fn a_failed_move_puts_the_picks_back(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(SettingsStore::for_test());
            SettingsStore::update(cx, |s| *s = settings());
            let failed =
                trash(cx, Path::new("/t/mine.json"), &|v| v == "/t/mine.json", |_| anyhow::bail!("stub"));
            assert!(failed.is_err());
            let s = &cx.global::<SettingsStore>().settings;
            assert_eq!((s.theme.as_str(), s.theme_light.as_str()), ("/t/mine.json", "Nord"));
            trash(cx, Path::new("/t/mine.json"), &|v| v == "/t/mine.json", |_| Ok(())).expect("moved");
            let s = &cx.global::<SettingsStore>().settings;
            assert_eq!((s.theme.as_str(), s.theme_light.as_str()), (DEFAULT_THEME, "Nord"));
        });
    }
}
