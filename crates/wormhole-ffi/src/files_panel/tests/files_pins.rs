//! The pinned folders live in the experiment's own `config.json`; one test owns them, since the file is
//! the process's and tests run in parallel.

use super::write;
use crate::files_panel::FilesService;

#[test]
fn pins_keep_their_order_follow_a_renamed_folder_and_ignore_what_cannot_be_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().canonicalize().expect("canonical");
    write(&root.join("keep/inner/x"), "x");
    write(&root.join("other/x"), "x");
    let files = FilesService::default();
    let (keep, inner, other) = (root.join("keep"), root.join("keep/inner"), root.join("other"));
    let pin = |p: &std::path::Path| files.set_pinned(p.to_string_lossy().into_owned(), true);
    for p in [&keep, &other, &inner] {
        pin(p);
    }
    files.set_pinned("relative/folder".into(), true);
    pin(&keep);
    let mine = |files: &FilesService| -> Vec<String> {
        files.pins().into_iter().filter(|p| p.starts_with(&root.to_string_lossy().into_owned())).collect()
    };
    let text = |p: &std::path::Path| p.to_string_lossy().into_owned();
    assert_eq!(mine(&files), [text(&keep), text(&other), text(&inner)], "in the order pinned, once each");
    assert!(!files.pins().iter().any(|p| p == "relative/folder"), "a relative path is never pinned");

    let moved = files.rename(text(&root), "keep".into(), "kept".into());
    assert_eq!(moved, Ok("kept".into()));
    assert_eq!(
        mine(&files),
        [text(&root.join("kept")), text(&other), text(&root.join("kept/inner"))],
        "the pin of a renamed folder, and of one below it, name the new place"
    );
    for p in [root.join("kept"), other, root.join("kept/inner")] {
        files.set_pinned(text(&p), false);
    }
    assert!(mine(&files).is_empty());
}
