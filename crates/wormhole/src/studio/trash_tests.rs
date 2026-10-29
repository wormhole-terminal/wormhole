//! Move to Trash in the theme menu: who has the item, what the sheet says, and what a move or a
//! failed one does to the picks. The move itself is a stub: nothing leaves a temp dir, no file is
//! trashed.

use gpui_kit::TestAppContext;
use wormhole_core::theme::registry::DEFAULT_THEME;

use super::document::Subject;
use super::themes::Themes;
use super::window_tests::{failed, open_with, own_themes, with};
use crate::settings::SettingsStore;

const DUSK: &str = "/themes/dusk.json";

fn trashing(trash: crate::settings::theme_trash::Trash) -> Themes {
    Themes { selects: |setting, path| setting == path.to_string_lossy(), trash, ..own_themes() }
}

fn open_dusk(
    cx: &mut TestAppContext,
    themes: Themes,
) -> (gpui_kit::AnyWindowHandle, gpui_kit::Entity<super::StudioWindow>) {
    let (handle, studio) = open_with(cx, themes);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.open_target(&super::opening::Target::Own(DUSK.into()), window, cx);
    });
    (handle, studio)
}

fn menu(
    cx: &mut TestAppContext,
    handle: gpui_kit::AnyWindowHandle,
    studio: &gpui_kit::Entity<super::StudioWindow>,
) -> Vec<String> {
    with(cx, handle, studio, |studio, window, cx| studio.open_theme_menu(window, cx));
    crate::platform::record::open().expect("the menu opened").outline()
}

#[gpui_kit::test]
fn only_one_of_your_own_themes_can_be_moved_to_the_trash(cx: &mut TestAppContext) {
    let item = "- Move to Trash\u{2026}".to_string();
    // A built-in's copy (Untitled) has no file to trash.
    let (handle, studio) = open_with(cx, trashing(|_| Ok(())));
    assert!(!menu(cx, handle, &studio).contains(&item));
    // A copy of a hand-written family file is Untitled as well, and a project's theme is no file
    // of the themes folder.
    with(cx, handle, &studio, |studio, _, _| {
        assert!(!studio.can_trash());
        assert!(matches!(studio.doc.subject, Subject::Untitled { .. }));
    });
    let (handle, studio) = open_dusk(cx, trashing(|_| Ok(())));
    assert!(menu(cx, handle, &studio).contains(&item));
}

#[gpui_kit::test]
fn trashing_a_picked_theme_sets_that_pick_to_the_default_and_leaves_the_other(cx: &mut TestAppContext) {
    let (handle, studio) = open_dusk(cx, trashing(|_| Ok(())));
    cx.update(|cx| {
        SettingsStore::update(cx, |s| {
            s.theme = DUSK.into();
            s.theme_light = "Nord".into();
        });
    });
    menu(cx, handle, &studio);
    super::window_tests::pick_from_menu(cx, handle, "Move to Trash\u{2026}");
    let (message, detail) = cx.pending_prompt().expect("the question");
    assert_eq!(message, "Move \u{201c}Dusk\u{201d} to the Trash?");
    assert!(detail.contains("get it back from the Trash") && detail.contains(DEFAULT_THEME), "{detail}");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    let s = cx.update(|cx| cx.global::<SettingsStore>().settings.clone());
    assert_eq!((s.theme.as_str(), s.theme_light.as_str()), (DUSK, "Nord"), "Cancel changes nothing");
    pick_and_move(cx, handle, &studio);
    let s = cx.update(|cx| cx.global::<SettingsStore>().settings.clone());
    assert_eq!((s.theme.as_str(), s.theme_light.as_str()), (DEFAULT_THEME, "Nord"));
    with(cx, handle, &studio, |studio, _, _| {
        assert!(matches!(studio.doc.subject, Subject::Untitled { .. }), "the theme now in use is open");
    });
}

fn pick_and_move(
    cx: &mut TestAppContext,
    handle: gpui_kit::AnyWindowHandle,
    studio: &gpui_kit::Entity<super::StudioWindow>,
) {
    menu(cx, handle, studio);
    super::window_tests::pick_from_menu(cx, handle, "Move to Trash\u{2026}");
    cx.simulate_prompt_answer("Move to Trash");
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_failed_move_changes_nothing_and_says_so(cx: &mut TestAppContext) {
    let (handle, studio) = open_dusk(cx, trashing(|_| anyhow::bail!("stub")));
    cx.update(|cx| SettingsStore::update(cx, |s| s.theme = DUSK.into()));
    pick_and_move(cx, handle, &studio);
    let s = cx.update(|cx| cx.global::<SettingsStore>().settings.clone());
    assert_eq!(s.theme, DUSK, "the pick is back");
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.doc.subject, Subject::Own { path: DUSK.into() });
        assert_eq!(studio.notice, failed("\u{201c}Dusk\u{201d} could not be moved to the Trash."));
    });
}
