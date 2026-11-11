//! The Files tab's reads and its rename, on hostile and ordinary folders.

use std::fs;

use tempfile::TempDir;
use wormhole_core::storage::files::{TEXT_CAP, load};

use super::{link, write};
use crate::files_panel::{
    FileContentRecord, FileEntryKind, FileProblem, FilesFailure, FilesService, RenameFailure,
};

fn service() -> FilesService {
    FilesService
}

fn root() -> TempDir {
    tempfile::tempdir().expect("temp dir")
}

fn path(dir: &TempDir) -> String {
    dir.path().to_string_lossy().into_owned()
}

#[test]
fn a_folder_lists_folders_first_in_natural_order_and_marks_links_and_hostile_names() {
    let dir = root();
    let outside = root();
    for name in ["file10.txt", "file2.txt", "README.md"] {
        write(&dir.path().join(name), "x");
    }
    fs::create_dir(dir.path().join("src")).expect("mkdir");
    fs::create_dir(dir.path().join("Docs")).expect("mkdir");
    write(&dir.path().join("evil\nname"), "x");
    link(outside.path(), &dir.path().join("jump"));
    let listed = service().list(path(&dir), String::new()).expect("lists");
    let names: Vec<&str> = listed.entries.iter().map(|entry| entry.label.as_str()).collect();
    assert_eq!(names, ["Docs", "src", "evil\u{FFFD}name", "file2.txt", "file10.txt", "jump", "README.md"]);
    let kind = |name: &str| listed.entries.iter().find(|e| e.name == name).map(|e| e.kind);
    assert_eq!(kind("src"), Some(FileEntryKind::Folder));
    assert_eq!(kind("jump"), Some(FileEntryKind::Link), "a link is a link, never the folder it points at");
    let hostile = listed.entries.iter().find(|e| e.label.contains('\u{FFFD}')).expect("hostile entry");
    assert_eq!(hostile.name, "evil\nname", "the real name is what a path is made of");
    assert_eq!((listed.more, listed.at_least), (0, false));
}

#[test]
fn a_huge_folder_is_cut_at_the_cap_and_says_how_much_is_left() {
    let dir = root();
    for n in 0..5_003 {
        fs::File::create(dir.path().join(format!("f{n:05}"))).expect("create");
    }
    let listed = service().list(path(&dir), String::new()).expect("lists");
    assert_eq!(listed.entries.len(), 5_000);
    assert_eq!(listed.more, 3);
    assert_eq!(listed.entries[0].name, "f00000", "the front of the alphabet is kept");
}

#[test]
fn listing_never_walks_through_a_link_or_out_of_the_root() {
    let dir = root();
    let outside = root();
    write(&outside.path().join("secret.txt"), "secret");
    link(outside.path(), &dir.path().join("jump"));
    fs::create_dir(dir.path().join("real")).expect("mkdir");
    let files = service();
    assert_eq!(
        files.list(path(&dir), "jump".into()),
        Err(FilesFailure::Gone),
        "a link is not a folder to enter"
    );
    assert_eq!(files.list(path(&dir), "../".into()), Err(FilesFailure::Other), "no .. in a path");
    assert_eq!(files.list(path(&dir), "/etc".into()), Err(FilesFailure::Other), "no absolute path");
    assert_eq!(files.list(path(&dir), "real/nothing".into()), Err(FilesFailure::Gone));
    assert!(files.list(path(&dir), "real".into()).is_ok());
}

#[test]
fn a_file_reads_as_what_it_is_and_never_through_a_link_out_of_the_root() {
    let dir = root();
    let outside = root();
    write(&dir.path().join("a.rs"), "fn main() {}\n");
    write(&dir.path().join("blob.bin"), "ab\0cd");
    let png = [&b"\x89PNG\r\n\x1a\n"[..], b"rest"].concat();
    fs::write(dir.path().join("pic.png"), &png).expect("write");
    write(&dir.path().join("fake.png"), "not a picture");
    write(&outside.path().join("secret.txt"), "secret");
    link(&outside.path().join("secret.txt"), &dir.path().join("leak.txt"));
    link(outside.path(), &dir.path().join("jump"));
    fs::create_dir(dir.path().join("folder")).expect("mkdir");
    let files = service();
    let read = |p: &str| files.read(path(&dir), p.into());
    assert_eq!(
        read("a.rs"),
        FileContentRecord::Text { text: "fn main() {}\n".into(), size: 13, truncated: false }
    );
    assert_eq!(read("blob.bin"), FileContentRecord::Binary { size: 5 });
    assert_eq!(read("pic.png"), FileContentRecord::Image { bytes: png, size: 12 });
    assert!(
        matches!(read("fake.png"), FileContentRecord::Text { .. }),
        "a .png that is not one shows as text"
    );
    let outside_of_root = FileContentRecord::Unavailable { problem: FileProblem::Outside };
    assert_eq!(read("leak.txt"), outside_of_root, "a file link out of the root is refused");
    assert_eq!(read("jump/secret.txt"), outside_of_root, "so is a file below a folder link");
    assert_eq!(read("../x"), outside_of_root);
    assert_eq!(read("/etc/hosts"), outside_of_root);
    assert_eq!(read("folder"), FileContentRecord::Unavailable { problem: FileProblem::NotAFile });
    assert_eq!(read("missing"), FileContentRecord::Unavailable { problem: FileProblem::Gone });
}

#[test]
fn a_big_file_shows_its_start_and_a_huge_image_is_not_read_at_all() {
    let dir = root();
    let big = "x".repeat(TEXT_CAP as usize + 10);
    write(&dir.path().join("big.log"), &big);
    let FileContentRecord::Text { text, size, truncated } = service().read(path(&dir), "big.log".into())
    else {
        panic!("a log shows as text");
    };
    assert!(truncated);
    assert_eq!(size, TEXT_CAP + 10);
    assert_eq!(text.len() as u64, TEXT_CAP, "only the first cap's worth is read");
    // 51 MiB of nothing (a sparse file): over the image cap, so it is a size and not a picture.
    let sparse = fs::File::create(dir.path().join("huge.png")).expect("create");
    sparse.set_len(51 << 20).expect("sparse");
    assert_eq!(service().read(path(&dir), "huge.png".into()), FileContentRecord::Binary { size: 51 << 20 });
    // The service and core agree on what a file is.
    assert!(matches!(
        load(dir.path(), "big.log".as_ref()),
        wormhole_core::storage::files::Loaded::Text { .. }
    ));
}

#[test]
fn a_rename_keeps_the_folder_refuses_what_exists_and_goes_through_no_link() {
    let dir = root();
    let outside = root();
    write(&dir.path().join("a.txt"), "a");
    write(&dir.path().join("b.txt"), "b");
    write(&dir.path().join("sub/inner.txt"), "i");
    write(&outside.path().join("victim.txt"), "v");
    link(outside.path(), &dir.path().join("jump"));
    link(&dir.path().join("a.txt"), &dir.path().join("alias"));
    let files = service();
    let rename = |from: &str, to: &str| files.rename(path(&dir), from.into(), to.into());
    assert_eq!(rename("a.txt", "c.txt"), Ok("c.txt".into()));
    assert!(dir.path().join("c.txt").exists() && !dir.path().join("a.txt").exists());
    assert_eq!(rename("sub/inner.txt", "outer.txt"), Ok("sub/outer.txt".into()), "in its own folder");
    assert_eq!(rename("b.txt", "c.txt"), Err(RenameFailure::Exists), "nothing is replaced");
    assert_eq!(fs::read_to_string(dir.path().join("c.txt")).expect("kept"), "a");
    for bad in ["", "  ", ".", "..", "a/b", "a\0b", "evil\u{202e}txt", &"x".repeat(256)] {
        assert_eq!(rename("b.txt", bad), Err(RenameFailure::Name), "{bad:?}");
    }
    assert_eq!(rename("jump/victim.txt", "mine.txt"), Err(RenameFailure::Gone), "not through a folder link");
    assert!(outside.path().join("victim.txt").exists(), "what is outside is untouched");
    assert_eq!(rename("../up", "x"), Err(RenameFailure::Gone));
    assert_eq!(rename("", "x"), Err(RenameFailure::Gone), "the root itself is not renamed");
    assert_eq!(rename("nothing", "x"), Err(RenameFailure::Gone));
    assert_eq!(rename("b.txt", "b.txt"), Ok("b.txt".into()), "the same name changes nothing");
    assert_eq!(
        rename("alias", "renamed-alias"),
        Ok("renamed-alias".into()),
        "a link is renamed as the link it is"
    );
    assert!(dir.path().join("c.txt").exists(), "what it pointed at stays");
}

#[test]
fn a_rename_that_only_changes_case_works_on_any_volume() {
    let dir = root();
    write(&dir.path().join("readme.md"), "r");
    let renamed = service().rename(path(&dir), "readme.md".into(), "README.md".into());
    assert_eq!(renamed, Ok("README.md".into()));
    let names: Vec<String> = fs::read_dir(dir.path())
        .expect("read")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["README.md"]);
}
