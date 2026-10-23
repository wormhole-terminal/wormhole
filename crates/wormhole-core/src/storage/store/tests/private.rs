//! The data folder Wormhole makes is its owner's alone: folders `0700`, files `0600`.
use std::path::Path;

use super::*;
use crate::config::profile::{NewProfile, ProfileList, StartFrom};
use crate::storage::fsutil::{self, LinkPolicy};
use crate::storage::paths::DataRoot;

/// Every folder and file under `dir` with a mode other than expected: `(path, mode)`.
fn not_private(dir: &Path, found: &mut Vec<(String, u32)>) {
    for entry in std::fs::read_dir(dir).expect("lists") {
        let path = entry.expect("entry").path();
        let meta = std::fs::symlink_metadata(&path).expect("stat");
        let mode = crate::platform::fs::mode(&meta) & 0o777;
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
    assert_eq!(crate::platform::fs::mode(&std::fs::metadata(&data).expect("stat")) & 0o777, 0o700);
    assert!(found.is_empty(), "not owner-only: {found:?}");
}

#[test]
fn launch_tightens_only_wormholes_names_and_only_in_its_data_folder() {
    let base = tempfile::tempdir().expect("temp dir");
    let loose = |path: &Path| {
        std::fs::set_permissions(path, crate::platform::fs::permissions(0o644)).expect("chmod");
    };
    // A folder someone pointed `WORMHOLE_DATA_DIR` at that holds no Wormhole data: left alone.
    let elsewhere = base.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("dir");
    std::fs::write(elsewhere.join("notes.txt"), "x").expect("writes");
    loose(&elsewhere.join("notes.txt"));
    std::fs::set_permissions(&elsewhere, crate::platform::fs::permissions(0o755)).expect("chmod");
    assert!(!super::super::is_data_dir(&elsewhere, true));

    // Wormhole's: its names tightened, a stranger's file beside them not.
    let data = base.path().join("data");
    std::fs::create_dir_all(data.join("profiles/work")).expect("dir");
    for name in ["wormhole.db", "wormhole.db-wal", "config.json", "profiles/work/config.json", "notes.txt"] {
        std::fs::write(data.join(name), "x").expect("writes");
        loose(&data.join(name));
    }
    assert!(super::super::is_data_dir(&data, true));
    super::super::make_data_dir_private(&data);
    let mode =
        |name: &str| crate::platform::fs::mode(&std::fs::metadata(data.join(name)).expect("stat")) & 0o777;
    for name in ["wormhole.db", "wormhole.db-wal", "config.json", "profiles/work/config.json"] {
        assert_eq!(mode(name), 0o600, "{name}");
    }
    assert_eq!(mode("profiles"), 0o700);
    assert_eq!(mode("notes.txt"), 0o644);
}
