use super::*;

#[test]
fn a_file_that_is_not_a_database_is_moved_aside_and_replaced() {
    let dir = temp_dir("not-a-db");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("wormhole.db"), "this is not a sqlite file, ".repeat(500)).expect("writes");
    // A leftover WAL next to the broken file must not end up beside (and be replayed into) the new one.
    std::fs::write(dir.join("wormhole.db-wal"), "stale wal").expect("writes");

    let mut store = Store::open_at(&dir).expect("starts fresh");
    let aside = store.recovered_from().expect("says it recovered").to_path_buf();
    assert!(aside.file_name().expect("name").to_string_lossy().starts_with("wormhole.db.corrupt-"));
    assert!(std::fs::read(&aside).expect("kept").starts_with(b"this is not"));

    // The new database works.
    assert!(store.load().expect("loads").is_empty());
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    drop(store);
    let store = Store::open_at(&dir).expect("reopens");
    assert!(store.recovered_from().is_none(), "only the open that recovered says so");
    assert_eq!(store.load().expect("loads")[0].sessions.len(), 1);
    let mains = corrupt_files(&dir).into_iter().filter(|n| !n.ends_with("-wal") && !n.ends_with("-shm"));
    assert_eq!(mains.count(), 1, "the old file is set aside once, not on every open");
}

#[test]
fn a_database_with_a_damaged_page_is_moved_aside_and_replaced() {
    use std::io::{Seek, SeekFrom, Write};
    let dir = temp_dir("damaged-page");
    {
        let mut store = Store::open_at(&dir).expect("opens");
        let mut record = window(1, (1..=5).map(|id| session(id, "/a")).collect());
        record.sessions[0].notes = vec![note("kept")];
        store.save_window(&record).expect("saves");
    }
    // Page 2 is the root of the first table: its page type byte becomes garbage.
    let mut file = std::fs::OpenOptions::new().write(true).open(dir.join("wormhole.db")).expect("opens file");
    file.seek(SeekFrom::Start(4096)).expect("seeks");
    file.write_all(&[0xff; 4096]).expect("damages");
    drop(file);

    let store = Store::open_at(&dir).expect("starts fresh");
    assert!(store.recovered_from().is_some());
    assert!(store.load().expect("loads").is_empty());
    assert_eq!(corrupt_files(&dir).len(), 1);
}

#[test]
fn a_healthy_database_is_opened_as_it_is() {
    let dir = temp_dir("healthy");
    {
        let mut store = Store::open_at(&dir).expect("opens");
        assert!(store.recovered_from().is_none());
        store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    }
    let store = Store::open_at(&dir).expect("reopens");
    assert!(store.recovered_from().is_none());
    assert_eq!(store.load().expect("loads")[0].sessions.len(), 1);
    assert!(corrupt_files(&dir).is_empty());
}

#[test]
fn moving_a_database_aside_takes_its_wal_and_shm_along_and_never_overwrites() {
    use crate::store::move_aside;
    let dir = temp_dir("move-aside");
    std::fs::create_dir_all(&dir).expect("dir");
    let db = dir.join("wormhole.db");
    for round in 0..2 {
        for (suffix, content) in [("", "db"), ("-wal", "wal"), ("-shm", "shm")] {
            std::fs::write(format!("{}{suffix}", db.display()), format!("{content}{round}")).expect("writes");
        }
        let aside = move_aside(&db, "corrupt").expect("moves");
        for (suffix, content) in [("", "db"), ("-wal", "wal"), ("-shm", "shm")] {
            let moved = std::fs::read_to_string(format!("{}{suffix}", aside.display())).expect("moved");
            assert_eq!(moved, format!("{content}{round}"));
            assert!(!std::path::Path::new(&format!("{}{suffix}", db.display())).exists());
        }
    }
    assert_eq!(corrupt_files(&dir).len(), 6, "two rounds, three files each, none overwritten");
}
