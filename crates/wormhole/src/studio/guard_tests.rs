//! The Studio's guards in a headless GPUI: what a save, a rename and a switch refuse or keep (see
//! `window_tests` for the helpers and the rest of the window).

use gpui_kit::TestAppContext;

use super::document::Subject;
use super::save::Notice;
use super::themes::{Found, Saved, Themes};
use super::window_tests::{edit, failed, name_it, open_with, own_themes, themes, with};

/// Choosing the theme that is being edited, edits and all, opens nothing and asks nothing: reading it
/// again would show the file from before the edits, and a Save would then write that over them.
#[gpui_kit::test]
fn choosing_the_theme_being_edited_does_nothing(cx: &mut TestAppContext) {
    let own = Themes { locate: |_| Found::File("/themes/dusk.json".into()), ..own_themes() };
    let (handle, studio) = open_with(cx, own);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.open_target(&super::opening::Target::Own("/themes/dusk.json".into()), window, cx);
        assert!(!studio.asking && studio.dirty(), "the edits are still there");
        assert_eq!(studio.doc.file.ui.get("primary").map(String::as_str), Some("#3d59a1"));
    });
    assert!(!cx.has_pending_prompt());
}

/// Save before a switch writes the file first and reads the next theme after it: what is opened is
/// what is on disk then.
#[gpui_kit::test]
fn the_next_theme_is_read_after_the_save(cx: &mut TestAppContext) {
    use std::cell::RefCell;
    thread_local!(static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) });
    let own = Themes {
        locate: |_| Found::File("/themes/dusk.json".into()),
        read: |path| {
            LOG.with(|l| l.borrow_mut().push(format!("read {}", path.display())));
            let file = wormhole_core::theme::tokens::parse_theme_file(r##"{"name": "Dusk"}"##)?;
            Ok(super::themes::Loaded { file, stamp: None })
        },
        overwrite: |path, _, _| {
            LOG.with(|l| l.borrow_mut().push("write".to_string()));
            Ok(Saved { path: path.to_path_buf(), stamp: None, lost_comments: false })
        },
        ..themes()
    };
    let (handle, studio) = open_with(cx, own);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.open_target(&super::opening::Target::Own("/themes/other.json".into()), window, cx);
        assert!(studio.asking);
    });
    cx.simulate_prompt_answer("Save");
    cx.run_until_parked();
    let log = LOG.with(|l| l.borrow().clone());
    assert_eq!(
        log,
        ["read /themes/dusk.json", "read /themes/other.json", "write", "read /themes/other.json"]
    );
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.doc.subject, Subject::Own { path: "/themes/other.json".into() });
        assert!(!studio.dirty());
    });
}

/// A theme file from a newer Wormhole is never written over: Duplicate is how to keep it.
#[gpui_kit::test]
fn a_file_from_a_newer_wormhole_is_not_saved_over(cx: &mut TestAppContext) {
    let own = Themes {
        locate: |_| Found::File("/themes/dusk.json".into()),
        read: |_| {
            let mut file = wormhole_core::theme::tokens::parse_theme_file(
                r##"{"version": 9, "id": "dusk", "name": "Dusk", "glow": true}"##,
            )?;
            let base =
                wormhole_core::theme::tokens::copy::copy_builtin_base("nord", &|_| None, "2026-10-29")?;
            file.base = Some(Box::new(base));
            Ok(super::themes::Loaded { file, stamp: None })
        },
        overwrite: |_, _, _| panic!("a newer file is never written"),
        save_new: |file| {
            assert!(!file.read.newer, "the copy is this version's own file");
            Ok(Saved { path: "/themes/dusk-copy.json".into(), stamp: None, lost_comments: false })
        },
        ..themes()
    };
    let (handle, studio) = open_with(cx, own);
    with(cx, handle, &studio, |studio, window, cx| {
        assert!(studio.doc.file.read.newer);
        edit(studio, window, cx);
        studio.request_save(window, cx);
        assert_eq!(studio.notice, failed(wormhole_text::text(wormhole_text::keys::STUDIO_NEWER_FILE)));
        assert!(studio.dirty());
        studio.duplicate(window, cx);
        assert!(matches!(studio.notice, Some(Notice::Saved { .. })), "{:?}", studio.notice);
    });
}

/// A name that one version of a family has is not saved under, and no Replace question is asked.
#[gpui_kit::test]
fn a_versions_name_is_refused_before_replace_is_offered(cx: &mut TestAppContext) {
    let themes = Themes {
        version_named: |name| name == "Pair Light",
        saved_path: |_| Some("/themes/pair.json".into()),
        save_new: |_| panic!("nothing is written"),
        ..themes()
    };
    let (handle, studio) = open_with(cx, themes);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        name_it(studio, "Pair Light", window, cx);
        assert!(!studio.asking);
        let Some(Notice::Failed(text)) = studio.notice.clone() else { panic!("{:?}", studio.notice) };
        assert!(text.contains("Pair Light"), "{text}");
    });
    assert!(!cx.has_pending_prompt());
}

/// A rename to the name another theme has is refused, and the old name stays.
#[gpui_kit::test]
fn renaming_to_another_themes_name_is_refused(cx: &mut TestAppContext) {
    let themes = Themes {
        locate: |_| Found::File("/themes/dusk.json".into()),
        saved_path: |name| (name == "Other").then(|| "/themes/other.json".into()),
        ..own_themes()
    };
    let (handle, studio) = open_with(cx, themes);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.start_rename(window, cx);
        studio.name.update(cx, |input, cx| input.set_value("Other", window, cx));
        studio.finish_rename(window, cx);
        assert_eq!(studio.doc.name(), "Dusk");
        assert_eq!(studio.notice, failed(&wormhole_text::keys::studio_name_taken("Other")));
        assert!(!studio.dirty());
        // Its own name is no clash.
        studio.start_rename(window, cx);
        studio.name.update(cx, |input, cx| input.set_value("Dusk", window, cx));
        studio.finish_rename(window, cx);
        assert!(studio.notice == failed(&wormhole_text::keys::studio_name_taken("Other")));
    });
}

/// Replacing your own theme through a copy keeps the id its file has.
#[gpui_kit::test]
fn replacing_a_theme_through_a_copy_keeps_its_file_id(cx: &mut TestAppContext) {
    use std::cell::RefCell;
    thread_local!(static ASKED: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) });
    let themes = Themes {
        saved_path: |_| Some("/themes/mine.json".into()),
        new_id: |name, itself| {
            ASKED.with(|a| *a.borrow_mut() = itself.map(std::path::Path::to_path_buf));
            wormhole_core::theme::tokens::slug::derive_id(name)
        },
        save_new: |_| Ok(Saved { path: "/themes/mine.json".into(), stamp: None, lost_comments: false }),
        ..themes()
    };
    let (handle, studio) = open_with(cx, themes);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.doc.file.version = Some(1);
        studio.write_new("Mine".into(), cx);
    });
    assert_eq!(ASKED.with(|a| a.borrow().clone()), Some("/themes/mine.json".into()));
}
