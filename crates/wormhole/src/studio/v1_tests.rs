//! The Studio's second half, in a headless GPUI: a theme with a light and a dark appearance, the
//! update from its base, its earlier saves, a file from a newer Wormhole, and the preview chip.
//! Nothing reads or writes a themes folder; the stand-ins are `Themes` functions.

use std::path::PathBuf;

use gpui_kit::TestAppContext;
use wormhole_core::theme::saved_versions::SavedVersion;
use wormhole_core::theme::tokens::{ThemeFile, copy, read_theme_file};

use super::Edit;
use super::document::Subject;
use super::document::places::{Origin, Reach, origin, reach};
use super::modal::Modal;
use super::save::Notice;
use super::themes::{Found, Loaded, Themes};
use super::window_tests::{edit, open_with, themes, with};
use crate::settings::SettingsStore;

/// Wormhole Dark is in use, so the Studio opens on it as a copy with nothing of its own yet.
pub(super) fn builtin_themes() -> Themes {
    Themes { fresh_base: |from| copy::copy_builtin_base(from, &|_| None, "2026-10-30").ok(), ..themes() }
}

pub(super) const DUSK: &str = r##"{"name": "Dusk", "extends": "Nord"}"##;

/// Dusk, a file of the person's own, is in use.
pub(super) fn own_dusk() -> Themes {
    Themes {
        locate: |_| Found::File("/themes/dusk.json".into()),
        read: |_| Ok(Loaded { file: read_theme_file(DUSK)?, stamp: None }),
        ..themes()
    }
}

fn primary(studio: &super::StudioWindow, cx: &gpui_kit::App) -> String {
    let _ = studio;
    cx.global::<SettingsStore>().tokens.get("primary").map(|c| c.to_hex()).unwrap_or_default()
}

#[gpui_kit::test]
fn adding_an_appearance_makes_a_pair_in_one_undo_step(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        let to = studio.missing_appearance().expect("a single theme lacks one");
        studio.add_appearance(to, window, cx);
        assert_eq!(studio.doc.file.variants.len(), 2);
        assert_eq!(studio.doc.shown.as_deref(), Some("light"), "the new appearance is shown");
        assert!(studio.dirty());
        assert_eq!(studio.history.undo_len(), 1);
        assert!(studio.missing_appearance().is_none(), "it has both now");

        studio.undo(window, cx);
        assert!(studio.doc.file.variants.is_empty());
        assert!(!studio.dirty(), "undone to the saved file");
        assert_eq!(studio.doc.shown, None);
    });
}

#[gpui_kit::test]
fn the_chip_moves_a_value_as_one_step_and_the_switch_is_not_a_step(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        studio.add_appearance(wormhole_core::theme::tokens::Appearance::Light, window, cx);
        let steps = studio.history.undo_len();
        let file = &studio.doc.file;
        assert_eq!(origin(file, Some("light"), "terminal.ansi.1"), Origin::Base);
        assert_eq!(reach(file, Some("light"), "terminal.ansi.1"), Reach::ThisAppearance);

        studio.set_shown("dark", window, cx);
        assert_eq!(studio.history.undo_len(), steps, "a view change is not an edit");

        // The accent is shared; "this appearance" makes it each one's own.
        studio.toggle_reach("primary", window, cx);
        assert_eq!(reach(&studio.doc.file, Some("dark"), "primary"), Reach::ThisAppearance);
        assert_eq!(studio.history.undo_len(), steps + 1);
        studio.undo(window, cx);
        assert_eq!(reach(&studio.doc.file, Some("dark"), "primary"), Reach::Both);
    });
}

#[gpui_kit::test]
fn a_value_edited_in_one_appearance_leaves_the_other_alone(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        studio.add_appearance(wormhole_core::theme::tokens::Appearance::Light, window, cx);
        let look = |studio: &super::StudioWindow, shown: &str| {
            let file = super::document::places::folded(&studio.doc.file, Some(shown));
            (studio.themes.resolve)(&file).expect("resolves")
        };
        let dark_before = look(studio, "dark");
        // A surface is an appearance's own: the light one is edited and the dark one is as it was.
        studio.select_token("terminal.background", window, cx);
        studio.set_selected_value(Some("#112233".into()), Edit::Field, window, cx);
        assert_eq!(studio.doc.shown.as_deref(), Some("light"));
        assert_eq!(look(studio, "light").terminal.background.to_hex(), "#112233");
        assert_eq!(studio.doc.own("terminal.background").as_deref(), Some("#112233"));
        assert_eq!(look(studio, "dark"), dark_before, "the dark appearance is untouched");
        studio.set_shown("dark", window, cx);
        assert_eq!(origin(&studio.doc.file, Some("dark"), "terminal.background"), Origin::Base);
        assert_eq!(studio.doc.own("terminal.background"), None);

        // The accent is shared: editing it changes both.
        studio.select_token("primary", window, cx);
        studio.set_selected_value(Some("#445566".into()), Edit::Field, window, cx);
        assert_eq!(look(studio, "light").get("primary").map(|c| c.to_hex()).as_deref(), Some("#445566"));
        assert_eq!(look(studio, "dark").get("primary").map(|c| c.to_hex()).as_deref(), Some("#445566"));
    });
}

#[gpui_kit::test]
fn update_from_applies_as_one_undo_step_and_up_to_date_is_off(cx: &mut TestAppContext) {
    // The source has a different accent than the copy.
    let changed = Themes {
        fresh_base: |from| {
            let mut base = copy::copy_builtin_base(from, &|_| None, "2026-10-30").ok()?;
            base.file.ui.insert("primary".into(), "#ff00aa".into());
            Some(base)
        },
        ..themes()
    };
    let (handle, studio) = open_with(cx, changed);
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.update_check.state, super::update_from::State::Changed(1));
        studio.open_update_from(cx);
        assert!(matches!(studio.modal, Some(Modal::Update(_))));
        assert!(!studio.dirty(), "opening the sheet writes nothing");
        studio.apply_update(window, cx);
        assert!(studio.modal.is_none());
        assert!(studio.dirty());
        assert_eq!(studio.history.undo_len(), 1);
        assert_eq!(
            studio.doc.file.base.as_ref().and_then(|b| b.file.ui.get("primary")).map(String::as_str),
            Some("#ff00aa")
        );
        assert_eq!(studio.update_check.state, super::update_from::State::UpToDate);
        studio.undo(window, cx);
        assert!(!studio.dirty());
    });

    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, _, cx| {
        assert_eq!(studio.update_check.state, super::update_from::State::UpToDate);
        studio.open_update_from(cx);
        assert!(studio.modal.is_none(), "nothing to update");
    });
}

fn two_saves() -> Themes {
    Themes {
        versions: |_, _| {
            vec![
                SavedVersion { path: PathBuf::from("/v/2.json"), saved: 2_000, pinned: false },
                SavedVersion { path: PathBuf::from("/v/1.original.json"), saved: 1_000, pinned: true },
            ]
        },
        read_version: |v| {
            let mut file = read_theme_file(DUSK)?;
            let color = if v.pinned { "#111111" } else { "#222222" };
            file.ui.insert("primary".into(), color.into());
            Ok(file)
        },
        ..own_dusk()
    }
}

#[gpui_kit::test]
fn restore_loads_a_version_as_unsaved_edits_and_can_be_undone(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, two_saves());
    with(cx, handle, &studio, |studio, window, cx| {
        assert!(matches!(studio.doc.subject, Subject::Own { .. }));
        studio.open_versions(cx);
        let Some(Modal::Versions(panel)) = &studio.modal else { panic!("the panel is open") };
        assert_eq!(panel.row_count(), 3, "the file now, and two saves");
        // Selecting previews without touching the file.
        studio.select_version(2, cx);
        assert_eq!(primary(studio, cx), "#111111");
        assert!(!studio.dirty());
        studio.restore_version(window, cx);
        assert!(studio.modal.is_none());
        assert!(studio.dirty(), "restored as unsaved edits");
        assert_eq!(studio.doc.own("primary").as_deref(), Some("#111111"));
        assert_eq!(studio.history.undo_len(), 1);
        studio.undo(window, cx);
        assert!(!studio.dirty());
    });
}

#[gpui_kit::test]
fn closing_the_panel_puts_the_preview_back(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, two_saves());
    with(cx, handle, &studio, |studio, _, cx| {
        studio.open_versions(cx);
        studio.select_version(1, cx);
        assert_eq!(primary(studio, cx), "#222222");
        studio.close_modal(cx);
        assert_eq!(primary(studio, cx), "#81a1c1", "the file under edit is shown again");
    });
}

#[gpui_kit::test]
fn a_newer_theme_will_not_be_saved_over(cx: &mut TestAppContext) {
    let newer = Themes {
        read: |_| {
            let mut file = read_theme_file(DUSK)?;
            file.read.newer = true;
            file.read.skipped = 2;
            Ok(Loaded { file, stamp: None })
        },
        overwrite: |_, _, _| panic!("a newer theme is never written over"),
        ..own_dusk()
    };
    let (handle, studio) = open_with(cx, newer);
    with(cx, handle, &studio, |studio, window, cx| {
        assert!(studio.doc.is_newer());
        edit(studio, window, cx);
        studio.request_save(window, cx);
        assert!(matches!(studio.notice, Some(Notice::Failed(_))));
        assert!(studio.dirty());
        assert!(studio.missing_appearance().is_some());
    });
}

#[gpui_kit::test]
fn previewing_a_theme_not_in_use_shows_the_chip_until_it_is_used(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        assert!(studio.in_use(cx), "an untouched copy of the theme in use");
        edit(studio, window, cx);
        assert!(!studio.in_use(cx), "edits make it a theme that is not in use");
    });
    let (handle, studio) = open_with(cx, own_dusk());
    with(cx, handle, &studio, |studio, _, cx| {
        // `selects` says no pick names Dusk (the stub), so it is only being previewed.
        assert!(!studio.in_use(cx));
    });
    let (handle, studio) = open_with(cx, Themes { selects: |_, _| true, ..own_dusk() });
    with(cx, handle, &studio, |studio, _, cx| assert!(studio.in_use(cx)));
}

#[test]
fn a_loaded_file_flags_what_it_skipped() {
    let file: ThemeFile = read_theme_file(DUSK).expect("reads");
    assert!(!file.read.newer);
}

#[gpui_kit::test]
fn new_theme_is_blank_and_new_theme_from_copies_into_the_base(cx: &mut TestAppContext) {
    // The person's own themes folder, with a theme of the same name as a built-in in it: New Theme
    // From lists it, and copying it is `CopyOf`, not the built-in.
    let dir = super::themes::test_dir::make("new-from");
    std::fs::write(
        dir.join("dracula.json"),
        r##"{"name": "Dracula", "extends": "Nord", "ui": {"primary": "#123456"}}"##,
    )
    .expect("write");
    let (handle, studio) = open_with(cx, Themes::USER);
    with(cx, handle, &studio, |studio, window, cx| studio.open_theme_menu(window, cx));
    let outline = crate::platform::record::open().expect("the menu opened").outline();
    assert!(outline.contains(&"> New Theme From".to_string()), "{outline:?}");

    super::window_tests::pick_from_menu(cx, handle, "New Theme\u{2026}");
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.doc.name(), "Untitled");
        assert!(studio.doc.file.base.is_none() && studio.doc.file.extends.is_none());
        assert!(!studio.dirty());
        assert_eq!(studio.update_check.state, super::update_from::State::NoBase);
    });

    with(cx, handle, &studio, |studio, window, cx| studio.open_theme_menu(window, cx));
    super::window_tests::pick_from_menu(cx, handle, "New Theme From > Nord");
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.doc.name(), "Untitled (from Nord)");
        assert_eq!(studio.doc.file.base.as_ref().map(|b| b.from.as_str()), Some("nord"));
    });

    with(cx, handle, &studio, |studio, window, cx| studio.open_theme_menu(window, cx));
    super::window_tests::pick_from_menu(cx, handle, "New Theme From > Dracula");
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.doc.name(), "Untitled (from Dracula)");
        let base = studio.doc.file.base.as_ref().expect("a base");
        assert_eq!(base.from, "dracula_2", "the person's theme, not the built-in Dracula");
        assert_eq!(base.file.ui.get("primary").map(String::as_str), Some("#123456"));
    });
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui_kit::test]
fn use_with_match_system_sets_both_picks(cx: &mut TestAppContext) {
    use wormhole_core::config::ThemeMode;
    let pair_themes = Themes {
        read: |_| {
            let file = read_theme_file(
                r##"{"version": 1, "id": "mine", "name": "Mine", "base": {"from": "nord"}, "variants": [{"name": "light"}, {"name": "dark"}]}"##,
            )?;
            Ok(Loaded { file, stamp: None })
        },
        ..own_dusk()
    };
    let (handle, studio) = open_with(cx, pair_themes);
    with(cx, handle, &studio, |studio, _, cx| {
        assert!(studio.doc.is_pair());
        assert!(studio.can_follow_system(cx));
        studio.follow_system(cx);
        let s = &cx.global::<SettingsStore>().settings;
        assert_eq!(
            (s.theme_mode, s.theme_light.as_str(), s.theme.as_str()),
            (ThemeMode::System, "mine.light", "mine.dark")
        );
        assert!(!studio.can_follow_system(cx), "both are picks now");
    });
}
