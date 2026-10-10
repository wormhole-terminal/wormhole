use super::*;

#[test]
fn sessions_without_a_window_or_tabs_are_repaired_on_open() {
    let dir = temp_dir("repair");
    {
        let store = Store::open_at(&dir).expect("opens");
        // What an interrupted or hand-edited database could hold: a session naming a
        // window that has no row, and one that never got a tab row.
        store
            .conn
            .execute_batch(
                "INSERT INTO sessions (id, position, window_id) VALUES (9, 0, 42);
                 INSERT INTO windows (id, position) VALUES (7, 0);
                 INSERT INTO sessions (id, position, window_id) VALUES (11, 0, 7);",
            )
            .expect("inserts");
    }
    let store = Store::open_at(&dir).expect("repairs");
    let loaded = store.load().expect("loads");
    let ids: Vec<i64> = loaded.iter().map(|w| w.id).collect();
    assert!(ids.contains(&42) && ids.contains(&7), "windows: {ids:?}");
    let orphan = &loaded.iter().find(|w| w.id == 42).expect("window 42").sessions[0];
    assert_eq!(orphan.id, 9);
    let tabless = &loaded.iter().find(|w| w.id == 7).expect("window 7").sessions[0];
    assert_eq!(tabless.id, 11);
    // Each has the terminal's row now, with the session's id.
    assert_eq!(count_rows(&store, "SELECT COUNT(*) FROM tabs WHERE id = session_id AND id IN (9, 11)"), 2);
}

fn count_rows(store: &Store, sql: &str) -> i64 {
    store.conn.query_row(sql, [], |row| row.get(0)).expect("counts")
}

#[test]
fn opening_applies_the_connection_settings_and_indexes() {
    let dir = temp_dir("pragmas");
    let store = Store::open_at(&dir).expect("opens");
    let pragma = |name: &str| -> i64 {
        store.conn.query_row(&format!("PRAGMA {name}"), [], |r| r.get(0)).expect("pragma")
    };
    assert_eq!(pragma("synchronous"), 1, "NORMAL");
    assert_eq!(pragma("foreign_keys"), 1);
    assert_eq!(pragma("busy_timeout"), 5000);
    assert_eq!(pragma("journal_size_limit"), 8 * 1024 * 1024);
    assert_eq!(pragma("user_version"), 7);
    let mode: String = store.conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).expect("mode");
    assert_eq!(mode, "wal");
    for index in ["sessions_window", "splits_window", "tabs_session", "notes_session", "tasks_session"] {
        let n = count(
            &store,
            &format!("SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = '{index}'"),
        );
        assert_eq!(n, 1, "{index}");
    }
}

/// A database a build from before migrations wrote, whether it set a `user_version` or not, is not
/// upgraded: it is moved aside, untouched, and the app starts with a fresh one.
#[test]
fn a_database_from_before_migrations_is_moved_aside_not_upgraded() {
    for (name, version) in [("older-unversioned", 0), ("older-versioned", 2)] {
        let dir = temp_dir(name);
        {
            let conn = old_database(&dir);
            conn.execute_batch(
                "INSERT INTO windows VALUES (1, 0, 0, NULL, NULL);
                 INSERT INTO sessions (id, position, name, window_id) VALUES (1, 0, 'old', 1);
                 INSERT INTO tabs VALUES (1, 1, 0, '/a', 'scrollback');",
            )
            .expect("data");
            conn.pragma_update(None, "user_version", version).expect("version");
        }
        let mut store = Store::open_at(&dir).expect("starts fresh");
        assert!(store.load().expect("loads").is_empty(), "{name}");
        assert_eq!(count(&store, "PRAGMA user_version"), 7, "{name}");
        let aside = store.recovered_from().expect("says where the old one went").to_path_buf();
        assert!(aside.file_name().expect("name").to_string_lossy().starts_with("wormhole.db.old-"));
        let old = Connection::open(&aside).expect("opens the old file");
        let name_kept: String = old.query_row("SELECT name FROM sessions", [], |r| r.get(0)).expect("kept");
        assert_eq!(name_kept, "old", "{name}: the old file is left as it was");
        let version_kept: i64 = old.pragma_query_value(None, "user_version", |r| r.get(0)).expect("version");
        assert_eq!(version_kept, version);

        // The new database works, and is opened as it is from then on.
        store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
        drop(store);
        let store = Store::open_at(&dir).expect("reopens");
        assert!(store.recovered_from().is_none());
        assert_eq!(store.load().expect("loads")[0].sessions.len(), 1);
        let mains =
            aside_files(&dir, ".old-").into_iter().filter(|n| !n.ends_with("-wal") && !n.ends_with("-shm"));
        assert_eq!(mains.count(), 1, "{name}: moved aside once");
    }
}

#[test]
fn a_database_from_a_newer_build_is_refused_rather_than_guessed_at() {
    let dir = temp_dir("newer");
    {
        let store = Store::open_at(&dir).expect("opens");
        store.conn.pragma_update(None, "user_version", 99).expect("version");
    }
    let err = Store::open_at(&dir).err().expect("refuses");
    assert!(format!("{err:#}").contains("schema version 99"), "{err:#}");
    assert!(dir.join("wormhole.db").exists(), "the database is left where it is");
}
