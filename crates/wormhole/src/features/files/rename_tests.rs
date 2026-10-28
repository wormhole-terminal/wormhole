//! Rename in a real window: the field opens with the name selected but not its extension, ↩ renames on the
//! disk and the selection and an open viewer follow, a name that is taken is refused with the notice chip's
//! words and the field stays for another try, Esc cancels, and the root has no Rename.
//!
//! Everything is renamed inside the test's scratch data dir, which the harness removes afterwards.

use std::fs;
use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::file_tree::{Row, RowKind};
use wormhole_text::{Language, with_language};

use super::tests::{click, feature, go, repo, row, rows};
use crate::app_state::OpenWindow;
use crate::settings::update_settings;
use crate::window::test_harness::{draw, drive, isolate_in, read_cx, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// Rename… on the row at `rel`.
fn start_rename(cx: &mut TestAppContext, window: &OpenWindow, rel: &str) {
    let target = row(cx, window, rel);
    drive(cx, window, |ws, window, cx| {
        let session = ws.active_session().id;
        feature(ws).update(cx, |files, fcx| files.start_rename(session, &target, window, fcx));
    });
    draw(cx, window);
}

/// What is typed in the field, and what of it is selected, if a row is being renamed.
fn field(cx: &mut TestAppContext, window: &OpenWindow) -> Option<(String, std::ops::Range<usize>)> {
    read_cx(cx, window, |ws, cx| {
        let files = feature(ws);
        let renaming = files.read(cx).renaming.as_ref()?;
        let input = renaming.input.read(cx);
        Some((input.value().to_string(), input.selected_range()))
    })
}

fn type_and_commit(cx: &mut TestAppContext, window: &OpenWindow, name: &str) {
    drive(cx, window, |ws, window, cx| {
        feature(ws).update(cx, |files, fcx| {
            files.type_rename(name, window, fcx);
            files.commit_rename(window, fcx);
        });
    });
    cx.executor().run_until_parked();
    // The background rename lands, then the folder is read again.
    cx.run_until_parked();
    draw(cx, window);
    cx.run_until_parked();
}

fn selected(cx: &mut TestAppContext, window: &OpenWindow) -> Option<PathBuf> {
    read_cx(cx, window, |ws, cx| feature(ws).read(cx).of.values().next().and_then(|of| of.selected.clone()))
}

#[gpui_kit::test]
fn the_name_is_selected_without_its_extension_and_a_folder_whole(cx: &mut TestAppContext) {
    if isolate("the_name_is_selected_without_its_extension_and_a_folder_whole") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    start_rename(cx, &window, "README.md");
    assert_eq!(field(cx, &window), Some(("README.md".to_string(), 0..6)), "Finder selects `README`");
    assert!(read_cx(cx, &window, |ws, _| ws.is_editing()), "the window hears that a field has the keys");
    drive(cx, &window, |ws, _, cx| feature(ws).update(cx, |files, cx| files.cancel_rename(cx)));
    assert!(!read_cx(cx, &window, |ws, _| ws.is_editing()));

    start_rename(cx, &window, "docs");
    assert_eq!(field(cx, &window), Some(("docs".to_string(), 0..4)), "a folder: all of it");
}

#[gpui_kit::test]
fn a_rename_changes_the_disk_and_the_selection_and_the_viewer_follow(cx: &mut TestAppContext) {
    if isolate("a_rename_changes_the_disk_and_the_selection_and_the_viewer_follow") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    click(cx, &window, "README.md");
    cx.run_until_parked();
    assert!(read_cx(cx, &window, |ws, cx| ws.is_covered(cx)));

    start_rename(cx, &window, "README.md");
    type_and_commit(cx, &window, "GUIDE.md");

    assert!(repo.join("GUIDE.md").is_file() && !repo.join("README.md").exists(), "renamed on disk");
    assert_eq!(selected(cx, &window), Some(PathBuf::from("GUIDE.md")), "the selection follows");
    assert!(rows(cx, &window).contains(&"GUIDE.md".to_string()), "{:?}", rows(cx, &window));
    assert!(!rows(cx, &window).contains(&"README.md".to_string()));
    assert_eq!(field(cx, &window), None, "the field is gone");
    let viewing =
        read_cx(cx, &window, |ws, cx| feature(ws).read(cx).viewer.as_ref().map(|viewer| viewer.path.clone()));
    assert_eq!(viewing, Some(PathBuf::from("GUIDE.md")), "the viewer follows the file");

    // A folder with the open file in it: the viewer's path moves with it.
    click(cx, &window, "src");
    cx.run_until_parked();
    click(cx, &window, "src/main.rs");
    cx.run_until_parked();
    start_rename(cx, &window, "src");
    type_and_commit(cx, &window, "code");
    assert!(repo.join("code/main.rs").is_file());
    let viewing =
        read_cx(cx, &window, |ws, cx| feature(ws).read(cx).viewer.as_ref().map(|viewer| viewer.path.clone()));
    assert_eq!(viewing, Some(PathBuf::from("code/main.rs")));
    assert_eq!(selected(cx, &window), Some(PathBuf::from("code/main.rs")));
}

#[gpui_kit::test]
fn a_name_that_is_taken_is_refused_with_a_message_and_nothing_is_overwritten(cx: &mut TestAppContext) {
    if isolate("a_name_that_is_taken_is_refused_with_a_message_and_nothing_is_overwritten") {
        return;
    }
    with_language(Language::ENGLISH, || {
        let window = start(cx, 1);
        cx.update(|cx| {
            update_settings(cx, |s| _ = s.feature_flags.insert(FeatureFlag::Notices.key().into(), true));
        });
        let repo = repo("project");
        go(cx, &window, &repo);
        start_rename(cx, &window, "README.md");
        type_and_commit(cx, &window, "docs");
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "hello", "still there");
        assert!(repo.join("docs").is_dir());
        assert_eq!(
            field(cx, &window).map(|(typed, _)| typed),
            Some("docs".to_string()),
            "the field stays open with what was typed, for another try"
        );
        let said: Vec<String> =
            cx.update(|cx| crate::notice::notices(cx).into_iter().map(|report| report.words).collect());
        assert_eq!(said, ["Not renamed: something with that name exists already"]);

        // A name that can't be one gets its own words, and a good one then works.
        type_and_commit(cx, &window, "a/b");
        assert!(repo.join("README.md").is_file());
        type_and_commit(cx, &window, "NOTES.md");
        assert!(repo.join("NOTES.md").is_file());
        assert_eq!(field(cx, &window), None);
    });
}

#[gpui_kit::test]
fn esc_and_the_same_name_close_the_field_and_change_nothing(cx: &mut TestAppContext) {
    if isolate("esc_and_the_same_name_close_the_field_and_change_nothing") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    start_rename(cx, &window, "README.md");
    drive(cx, &window, |ws, window, cx| {
        feature(ws).update(cx, |files, fcx| {
            files.type_rename("other.md", window, fcx);
            files.cancel_rename(fcx);
        });
    });
    assert_eq!(field(cx, &window), None);
    assert!(repo.join("README.md").is_file() && !repo.join("other.md").exists());

    start_rename(cx, &window, "README.md");
    type_and_commit(cx, &window, "README.md");
    assert_eq!(field(cx, &window), None, "the same name only closes the field");
    assert!(repo.join("README.md").is_file());
}

#[gpui_kit::test]
fn the_root_and_a_note_cannot_be_renamed(cx: &mut TestAppContext) {
    if isolate("the_root_and_a_note_cannot_be_renamed") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    let root =
        Row { path: PathBuf::new(), depth: 0, label: String::new(), kind: RowKind::Folder { open: true } };
    let note = Row { path: Path::new("docs").into(), depth: 1, label: String::new(), kind: RowKind::Empty };
    drive(cx, &window, |ws, window, cx| {
        let session = ws.active_session().id;
        feature(ws).update(cx, |files, fcx| {
            files.start_rename(session, &root, window, fcx);
            files.start_rename(session, &note, window, fcx);
        });
    });
    assert_eq!(field(cx, &window), None);
}

#[gpui_kit::test]
fn a_link_is_renamed_and_not_what_it_points_at(cx: &mut TestAppContext) {
    if isolate("a_link_is_renamed_and_not_what_it_points_at") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    wormhole_core::platform::fs::symlink(repo.join("README.md"), repo.join("alias")).unwrap();
    go(cx, &window, &repo);
    start_rename(cx, &window, "alias");
    type_and_commit(cx, &window, "pointer");
    assert!(fs::symlink_metadata(repo.join("pointer")).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "hello");
}

#[gpui_kit::test]
fn a_renamed_folder_stays_open_and_its_pin_follows_it(cx: &mut TestAppContext) {
    if isolate("a_renamed_folder_stays_open_and_its_pin_follows_it") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    click(cx, &window, "src");
    cx.run_until_parked();
    drive(cx, &window, |ws, _, cx| {
        feature(ws).update(cx, |files, fcx| {
            files.set_pinned(&repo.join("src"), true, fcx);
            files.set_pinned(&repo.join("docs"), true, fcx);
        });
    });
    assert!(rows(cx, &window).contains(&"v src".to_string()));

    start_rename(cx, &window, "src");
    type_and_commit(cx, &window, "code");
    let shown = rows(cx, &window);
    let at = shown.iter().position(|line| line == "v code").expect("still open under its new name");
    assert_eq!(&shown[at + 1..at + 3], ["lib.rs", "main.rs"], "with what it held: {shown:?}");
    let pinned = read_cx(cx, &window, |ws, cx| feature(ws).read(cx).pinned(cx));
    assert_eq!(pinned, [repo.join("code"), repo.join("docs")], "the pin moved and kept its place");
}

#[gpui_kit::test]
fn a_rename_that_comes_back_for_a_closed_terminal_makes_nothing_for_it(cx: &mut TestAppContext) {
    if isolate("a_rename_that_comes_back_for_a_closed_terminal_makes_nothing_for_it") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    drive(cx, &window, |ws, _, cx| {
        feature(ws).update(cx, |files, fcx| {
            // A terminal that is gone: no entry of its own comes back.
            files.followed_rename(99, Path::new("a"), Path::new("b"), fcx);
            assert!(!files.of.contains_key(&99));
        });
    });
}

#[gpui_kit::test]
fn a_viewer_follows_a_rename_only_in_the_folder_it_was_opened_in(cx: &mut TestAppContext) {
    if isolate("a_viewer_follows_a_rename_only_in_the_folder_it_was_opened_in") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    click(cx, &window, "README.md");
    cx.run_until_parked();
    let path = |cx: &mut TestAppContext| {
        read_cx(cx, &window, |ws, cx| feature(ws).read(cx).viewer.as_ref().map(|v| v.path.clone()))
    };
    // The same relative path, in another folder (the terminal was pointed elsewhere): not this viewer's.
    drive(cx, &window, |ws, _, cx| {
        let session = ws.active_session().id;
        feature(ws).update(cx, |files, fcx| {
            files.viewer_renamed(
                session,
                Path::new("/elsewhere"),
                Path::new("README.md"),
                Path::new("X.md"),
                fcx,
            );
        });
    });
    assert_eq!(path(cx), Some(PathBuf::from("README.md")));
}
