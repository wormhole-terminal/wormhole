//! What the review of the Studio's second half found, one test per finding, each over the real
//! rules of a themes folder the test made (`Themes::USER` over `test_dir`) wherever the finding was
//! about what the folder holds.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_kit::TestAppContext;
use wormhole_core::config::ThemeMode;
use wormhole_core::theme::saved_versions::SavedVersion;
use wormhole_core::theme::tokens::appearance::add_appearance;
use wormhole_core::theme::tokens::{Appearance, ThemeFile, copy, read_theme_file, resolve};

use super::document::{Document, Subject, places};
use super::modal::Modal;
use super::opening::{Inputs, Target, open_target};
use super::save::Notice;
use super::themes::{Found, Loaded, Stamp, Themes, test_dir};
use super::update_from::{Check, State};
use super::v1_tests::{DUSK, builtin_themes, own_dusk};
use super::window_tests::{open_doc, open_with, with};
use super::{Edit, StudioWindow};
use crate::settings::SettingsStore;

/// A pair made from a built-in the way the Studio makes one: Mine, with the accent shared.
fn pair_from(slug: &str, to: Appearance) -> ThemeFile {
    let base = copy::copy_builtin_base(slug, &|_| None, "2026-10-30").expect("base");
    let mut file = ThemeFile {
        version: Some(1),
        id: Some("mine".into()),
        name: Some("Mine".into()),
        base: Some(Box::new(base)),
        ..ThemeFile::default()
    };
    file.ui.insert("primary".into(), "#ff8800".into());
    let have = resolve(&file, &|_| None).expect("resolves");
    add_appearance(&file, &have, to, &|_| None, "2026-10-30").expect("a pair")
}

/// The pair written in a themes folder of the test's own, open in the Studio.
fn open_pair(
    cx: &mut TestAppContext,
    name: &str,
) -> (PathBuf, gpui_kit::AnyWindowHandle, gpui_kit::Entity<StudioWindow>) {
    let dir = test_dir::make(name);
    let file = pair_from("wormhole.dark", Appearance::Light);
    let path = dir.join("mine.json");
    std::fs::write(&path, file.to_json_pretty()).expect("write");
    let doc = Document::new(file, Subject::Own { path: path.clone() }, Stamp::of(&path));
    let (handle, studio) = open_doc(cx, Themes::USER, doc);
    (dir, handle, studio)
}

fn picks(cx: &gpui_kit::App) -> (String, String) {
    let s = &cx.global::<SettingsStore>().settings;
    (s.theme.clone(), s.theme_light.clone())
}

/// 1. Saving an appearance of a pair that is the pick leaves both picks, and the dark pick still
/// names the dark appearance (a path names the first, light one).
#[gpui_kit::test]
fn saving_a_pair_leaves_its_picks_and_dark_still_shows_dark(cx: &mut TestAppContext) {
    let (dir, handle, studio) = open_pair(cx, "pair-save");
    cx.update(|cx| {
        SettingsStore::update(cx, |s| {
            s.theme_mode = ThemeMode::System;
            s.theme_light = "mine.light".into();
            s.theme = "mine.dark".into();
        })
    });
    with(cx, handle, &studio, |studio, window, cx| {
        studio.set_shown("dark", window, cx);
        studio.select_token("terminal.background", window, cx);
        studio.set_selected_value(Some("#101820".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        assert!(!studio.dirty(), "saved: {:?}", studio.notice);
        assert_eq!(picks(cx), ("mine.dark".to_string(), "mine.light".to_string()));
        let dark_pick = picks(cx).0;
        let found = (studio.themes.locate)(&dark_pick);
        assert!(matches!(found, Found::Variant(_, ref v) if v == "dark"), "dark mode shows Dark: {found:?}");
        assert!(studio.in_use(cx), "and it is in use");
    });
    std::fs::remove_dir_all(&dir).ok();
}

/// 1. A pick by an appearance's name ("Mine Dark") is a pick of the pair too; and renaming the
/// theme keeps each pick on the appearance it was on.
#[gpui_kit::test]
fn picks_by_an_appearances_name_follow_a_rename(cx: &mut TestAppContext) {
    let (dir, handle, studio) = open_pair(cx, "pair-rename");
    cx.update(|cx| {
        SettingsStore::update(cx, |s| {
            s.theme_mode = ThemeMode::System;
            s.theme_light = "Mine Light".into();
            s.theme = "Mine Dark".into();
        })
    });
    with(cx, handle, &studio, |studio, window, cx| {
        studio.set_shown("dark", window, cx);
        studio.select_token("terminal.background", window, cx);
        studio.set_selected_value(Some("#101820".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        assert!(!studio.dirty(), "saved: {:?}", studio.notice);
        assert_eq!(picks(cx), ("Mine Dark".to_string(), "Mine Light".to_string()), "named picks stay");

        studio.record_step(super::document::history::StepLabel::Rename, false, cx);
        studio.doc.file.name = Some("Evening".into());
        studio.request_save(window, cx);
        assert!(!studio.dirty(), "saved: {:?}", studio.notice);
        let (dark, light) = picks(cx);
        let found = |pick: &str| (studio.themes.locate)(pick);
        assert!(matches!(found(&dark), Found::Variant(_, ref v) if v == "dark"), "{dark}");
        assert!(matches!(found(&light), Found::Variant(_, ref v) if v == "light"), "{light}");
    });
    std::fs::remove_dir_all(&dir).ok();
}

/// 2. The chip on a value that works out differently in each appearance leaves the other one.
#[gpui_kit::test]
fn the_chip_leaves_the_other_appearance_as_it_resolved(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        studio.add_appearance(Appearance::Light, window, cx);
        studio.set_shown("dark", window, cx);
        let look = |studio: &StudioWindow, shown: &str| {
            (studio.themes.resolve)(&places::folded(&studio.doc.file, Some(shown))).expect("resolves")
        };
        let token = "syntax.keyword";
        let (light, dark) = (look(studio, "light").get(token), look(studio, "dark").get(token));
        assert_ne!(light, dark, "the two differ, or this proves nothing");
        studio.toggle_reach(token, window, cx);
        assert_eq!(places::reach(&studio.doc.file, Some("dark"), token), places::Reach::ThisAppearance);
        assert_eq!(look(studio, "light").get(token), light, "light as it was");
        assert_eq!(look(studio, "dark").get(token), dark, "dark as it was");
    });
}

/// 3. A fresh second appearance has nothing to update from, in either appearance; a generated one
/// has no source at all.
#[gpui_kit::test]
fn a_fresh_pair_shows_no_update_from(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        studio.add_appearance(Appearance::Light, window, cx);
        assert_eq!(studio.doc.shown.as_deref(), Some("light"));
        assert_eq!(studio.update_check.state, State::UpToDate, "the counterpart as it was copied");
        studio.set_shown("dark", window, cx);
        assert_eq!(studio.update_check.state, State::UpToDate);
    });
}

#[test]
fn a_generated_appearance_is_not_updated_from_its_source() {
    let base = copy::copy_builtin_base("nord", &|_| None, "2026-10-30").expect("base");
    let file = ThemeFile {
        version: Some(1),
        id: Some("mine".into()),
        name: Some("Mine".into()),
        base: Some(Box::new(base)),
        ..ThemeFile::default()
    };
    let have = resolve(&file, &|_| None).expect("resolves");
    let pair = add_appearance(&file, &have, Appearance::Light, &|_| None, "2026-10-30").expect("a pair");
    let themes = builtin_themes();
    assert_eq!(
        Check::run(&pair, Some("light"), &themes).state,
        State::NoBase,
        "generated: nothing to update"
    );
    assert_eq!(Check::run(&pair, Some("dark"), &themes).state, State::UpToDate, "the original copy");
}

/// 5. Add Appearance on a New Theme with nothing under it.
#[gpui_kit::test]
fn a_new_theme_with_no_base_can_get_its_other_appearance(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        let inputs = super::opening_inputs(cx);
        let doc = open_target(&Target::New, &inputs, &studio.themes).expect("a blank theme");
        studio.load(doc, window, cx);
        assert!(studio.doc.file.base.is_none());
        let to = studio.missing_appearance().expect("the menu offers it");
        studio.add_appearance(to, window, cx);
        assert!(!matches!(studio.notice, Some(Notice::Failed(_))), "{:?}", studio.notice);
        assert_eq!(studio.doc.file.variants.len(), 2, "a pair now");
        assert!(studio.doc.is_pair() && studio.missing_appearance().is_none());
        let other = if to == Appearance::Light { "light" } else { "dark" };
        assert_eq!(studio.doc.shown.as_deref(), Some(other));
        let made = (studio.themes.resolve)(&studio.doc.folded()).expect("resolves");
        assert_eq!(made.appearance, to);
    });
}

/// 6. Restoring a save of a file from a newer Wormhole leaves it one this version won't write over.
#[gpui_kit::test]
fn restoring_a_version_keeps_a_newer_file_unwritable(cx: &mut TestAppContext) {
    let newer = Themes {
        read: |_| {
            let mut file = read_theme_file(DUSK)?;
            file.read.newer = true;
            file.read.skipped = 2;
            Ok(Loaded { file, stamp: None })
        },
        versions: |_, _| vec![SavedVersion { path: PathBuf::from("/v/1.json"), saved: 1_000, pinned: false }],
        read_version: |_| {
            let mut file = read_theme_file(DUSK)?;
            file.ui.insert("primary".into(), "#222222".into());
            Ok(file)
        },
        overwrite: |_, _, _| panic!("a newer theme is never written over"),
        ..own_dusk()
    };
    let (handle, studio) = open_with(cx, newer);
    with(cx, handle, &studio, |studio, window, cx| {
        assert!(studio.doc.is_newer());
        studio.open_versions(cx);
        studio.select_version(1, cx);
        studio.restore_version(window, cx);
        assert!(matches!(studio.modal, None | Some(Modal::Update(_))));
        assert!(studio.dirty(), "restored");
        assert!(studio.doc.is_newer(), "still a file from a newer Wormhole");
        assert_eq!(studio.doc.file.read.skipped, 2);
        studio.request_save(window, cx);
        assert!(matches!(studio.notice, Some(Notice::Failed(_))), "the guard still holds");
    });
}

/// 7. New Theme From a theme of the person's that shares a built-in's name, or another file's.
#[test]
fn copying_your_theme_copies_that_file() {
    let dir = test_dir::make("copy-of");
    let write = |file: &str, text: &str| std::fs::write(dir.join(file), text).expect("write");
    write("dracula.json", r##"{"name": "Dracula", "extends": "Nord", "ui": {"primary": "#123456"}}"##);
    write("a-mine.json", r##"{"name": "Mine", "extends": "Nord", "ui": {"primary": "#aa0000"}}"##);
    write("b-mine.json", r##"{"name": "Mine", "extends": "Nord", "ui": {"primary": "#00aa00"}}"##);
    let themes = Themes::USER;
    assert!(!(themes.catalog)().iter().all(|e| e.builtin), "the catalog has the person's themes");
    let inputs = Inputs {
        shown: "Nord".into(),
        project: None,
        in_use: wormhole_core::theme::tokens::builtin_tokens()
            .into_iter()
            .find(|t| t.name == "Nord")
            .unwrap(),
    };
    let copy_of = |file: &str| {
        let doc = open_target(&Target::CopyOf(dir.join(file)), &inputs, &themes).expect("opens");
        let base = doc.file.base.clone().expect("a base");
        (base.from, base.file.ui.get("primary").cloned())
    };
    assert_eq!(copy_of("dracula.json"), ("dracula_2".into(), Some("#123456".into())), "not the built-in");
    assert_eq!(copy_of("a-mine.json"), ("mine".into(), Some("#aa0000".into())));
    assert_eq!(copy_of("b-mine.json"), ("mine_2".into(), Some("#00aa00".into())), "the second of one name");
    std::fs::remove_dir_all(&dir).ok();
}

/// 8. Redo of Add Appearance previews the appearance the window shows, not the first one.
#[gpui_kit::test]
fn redoing_add_appearance_previews_the_shown_appearance(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, builtin_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        studio.add_appearance(Appearance::Light, window, cx);
        studio.undo(window, cx);
        assert!(studio.doc.file.variants.is_empty());
        studio.redo(window, cx);
        assert_eq!(studio.doc.file.variants.len(), 2);
        let shown = studio.doc.shown.clone();
        assert_eq!(shown.as_deref(), Some("dark"), "a pair opens on the dark one");
        let previewed = cx.global::<SettingsStore>().tokens.appearance;
        let shown_look = (studio.themes.resolve)(&studio.doc.folded()).expect("resolves").appearance;
        assert_eq!(previewed, shown_look, "what is previewed is what is shown");
        assert_eq!(previewed, Appearance::Dark);
    });
}

static SELECTS: AtomicUsize = AtomicUsize::new(0);

/// 9. Whether a theme is in use is worked out when the document or the pick changes, never when
/// the window draws.
#[gpui_kit::test]
fn whether_a_theme_is_in_use_is_not_looked_up_in_a_draw(cx: &mut TestAppContext) {
    let themes = Themes {
        selects: |setting, _| {
            SELECTS.fetch_add(1, Ordering::SeqCst);
            setting == "/themes/dusk.json"
        },
        ..own_dusk()
    };
    let (handle, studio) = open_with(cx, themes);
    let after_open = SELECTS.load(Ordering::SeqCst);
    assert!(after_open >= 1, "worked out when the window opened");
    with(cx, handle, &studio, |studio, window, cx| {
        assert!(!studio.in_use(cx));
        // Draws, and edits that move the preview, look nothing up.
        window.refresh();
        cx.notify();
        studio.in_use(cx);
    });
    cx.run_until_parked();
    cx.update(|cx| {
        cx.refresh_windows();
    });
    cx.run_until_parked();
    assert_eq!(SELECTS.load(Ordering::SeqCst), after_open, "no lookup while drawing");
    // Another pick is looked up once, and is what the title row says.
    cx.update(|cx| SettingsStore::update(cx, |s| s.theme = "/themes/dusk.json".into()));
    cx.run_until_parked();
    assert_eq!(SELECTS.load(Ordering::SeqCst), after_open + 1);
    with(cx, handle, &studio, |studio, _, cx| assert!(studio.in_use(cx), "now in use"));
}
