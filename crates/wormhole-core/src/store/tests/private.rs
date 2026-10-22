//! The data folder Wormhole makes is its owner's alone: folders `0700`, files `0600`.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::*;
use crate::fsutil::{self, LinkPolicy};
use crate::profile::{DataRoot, NewProfile, ProfileList, StartFrom};

/// Every folder and file under `dir` with a mode other than expected: `(path, mode)`.
fn not_private(dir: &Path, found: &mut Vec<(String, u32)>) {
    for entry in std::fs::read_dir(dir).expect("lists") {
        let path = entry.expect("entry").path();
        let meta = std::fs::symlink_metadata(&path).expect("stat");
        let mode = meta.permissions().mode() & 0o777;
        // A program keeps the umask's mode (it may be installed for everyone elsewhere): in the
        // data folder its `0700` folder is what keeps it private.
        if path.ends_with("bin/wh") {
            assert_eq!(mode & 0o700, 0o700, "the owner can run it");
            continue;
        }
        let expected = if meta.is_dir() { 0o700 } else { 0o600 };
        if mode != expected {
            found.push((path.display().to_string(), mode));
        }
        if meta.is_dir() {
            not_private(&path, found);
        }
    }
}

#[test]
fn data_folder_is_private() {
    let base = tempfile::tempdir().expect("temp dir");
    let data = base.path().join("Wormhole");
    let root = DataRoot::new(&data);

    // The database, its WAL and shared memory, with a window and saved text.
    let mut store = Store::open_at(&data).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    store.save_scrollbacks(&[(1, "saved text")]).expect("saves");
    // A second profile: its folder, `profiles.json`, and its own database.
    let mut profiles = ProfileList::default();
    let id = profiles
        .create(
            &root,
            NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
        )
        .expect("creates");
    let work = Store::open_for(&root.profile(&id)).expect("opens");
    work.set_meta("k", "v").expect("writes");
    // What settings, trust, notes sync, the panic note and the `wh` command are written with.
    for file in ["config.json", "trusted-projects.json", "notes-sync.json", "logs/last-panic.txt"] {
        fsutil::write_atomic(&data.join(file), b"{}", LinkPolicy::Follow).expect("writes");
    }
    fsutil::write_atomic(&root.profile(&id).config_file(), b"{}", LinkPolicy::Follow).expect("writes");
    fsutil::write_atomic_executable(&data.join("bin/wh"), b"#!/bin/sh\n", LinkPolicy::Follow)
        .expect("writes");

    assert!(data.join("wormhole.db-wal").exists() && data.join("profiles.json").exists());
    let mut found = Vec::new();
    not_private(&data, &mut found);
    assert_eq!(std::fs::metadata(&data).expect("stat").permissions().mode() & 0o777, 0o700);
    assert!(found.is_empty(), "not owner-only: {found:?}");
}
