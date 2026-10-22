//! The data of a version from before the rename (the `Andromeda` folder) moves to `Wormhole` once.

use rusqlite::Connection;

use super::super::migrate_legacy_data_in;
use super::temp_dir;

/// An `Andromeda` folder as 0.2 leaves it: a database in WAL mode whose last row is only in the
/// journal, a config, a log.
fn old_install(base: &std::path::Path) -> Connection {
    let old = base.join("Andromeda");
    std::fs::create_dir_all(old.join("logs")).expect("mkdir");
    std::fs::write(old.join("config.json"), r#"{ "font_size": 15 }"#).expect("config");
    std::fs::write(old.join("logs/andromeda.2026-10-10.log"), "a line").expect("log");
    let conn = Connection::open(old.join("andromeda.db")).expect("opens");
    conn.pragma_update(None, "journal_mode", "WAL").expect("wal");
    conn.pragma_update(None, "wal_autocheckpoint", 0).expect("no checkpoint");
    conn.execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('kept');").expect("write");
    // Left open, as a crash would: the row is in andromeda.db-wal, not yet in the database.
    conn
}

#[test]
fn the_old_folder_and_its_database_move_once_with_a_copy_kept() {
    let base = temp_dir("rename-move");
    let _open = old_install(&base);
    assert!(base.join("Andromeda/andromeda.db-wal").exists(), "the fixture writes a journal");

    let moved = migrate_legacy_data_in(&base).expect("moves");
    let new = base.join("Wormhole");
    assert_eq!(moved, Some(new.clone()));
    assert!(!base.join("Andromeda").exists());
    assert_eq!(std::fs::read_to_string(new.join("config.json")).expect("config"), r#"{ "font_size": 15 }"#);
    assert!(new.join("logs/andromeda.2026-10-10.log").exists());
    assert!(!new.join("andromeda.db").exists() && new.join("andromeda-before-wormhole.db").exists());

    // The journal moved with the database: the row only it held is still there.
    let conn = Connection::open(new.join("wormhole.db")).expect("opens");
    let value: String = conn.query_row("SELECT v FROM t", [], |row| row.get(0)).expect("the row");
    assert_eq!(value, "kept");
    let backup = Connection::open(new.join("andromeda-before-wormhole.db")).expect("opens");
    let value: String = backup.query_row("SELECT v FROM t", [], |row| row.get(0)).expect("the row");
    assert_eq!(value, "kept", "the copy is a whole database too");

    assert_eq!(migrate_legacy_data_in(&base).expect("runs"), None, "only once");
}

#[test]
fn nothing_moves_when_the_new_folder_exists_or_there_is_no_old_one() {
    let base = temp_dir("rename-none");
    std::fs::create_dir_all(&base).expect("mkdir");
    assert_eq!(migrate_legacy_data_in(&base).expect("runs"), None, "a first install");

    let _open = old_install(&base);
    std::fs::create_dir_all(base.join("Wormhole")).expect("mkdir");
    assert_eq!(migrate_legacy_data_in(&base).expect("runs"), None);
    assert!(base.join("Andromeda/andromeda.db").exists(), "the old folder is left as it was");
}

#[test]
fn an_earlier_version_still_running_keeps_its_folder() {
    let base = temp_dir("rename-running");
    let _open = old_install(&base);
    // The running one holds the instance lock in its folder.
    let lock = std::fs::File::create(base.join("Andromeda/instance.lock")).expect("lock file");
    lock.lock().expect("locked");
    assert!(migrate_legacy_data_in(&base).is_err(), "it says why it waits");
    assert!(base.join("Andromeda/andromeda.db").exists() && !base.join("Wormhole").exists());
    drop(lock);
    // A child process another test forks in this instant holds a copy of the lock until it execs,
    // so the release can take a moment to show.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let moved = loop {
        match migrate_legacy_data_in(&base) {
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20))
            }
            other => break other,
        }
    };
    assert_eq!(moved.expect("moves"), Some(base.join("Wormhole")), "once it quit");
}
