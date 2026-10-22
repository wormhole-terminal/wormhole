//! Deleted terminal text leaves the disk: not in the database's free pages, not in the WAL.

use super::*;

const MARKER: &str = "SECRET-MARKER-1";

/// Which of the databases and their WALs hold `needle` anywhere in their raw bytes.
pub(super) fn on_disk(dir: &std::path::Path, needle: &str) -> Vec<String> {
    ["wormhole.db", "wormhole.db-wal", "scrollback.db", "scrollback.db-wal"]
        .into_iter()
        .filter(|name| {
            std::fs::read(dir.join(name))
                .is_ok_and(|bytes| bytes.windows(needle.len()).any(|w| w == needle.as_bytes()))
        })
        .map(String::from)
        .collect()
}

/// Enough lines that the text spills into overflow pages, as real scrollback does.
fn scrollback_with_marker() -> String {
    format!("$ gh auth token\n{MARKER}-ghp_0123456789\n").repeat(400)
}

#[test]
fn a_closed_terminals_text_is_found_in_neither_the_database_nor_the_wal() {
    let dir = temp_dir("erase-closed");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
    store.save_scrollbacks(&[(1, scrollback_with_marker()), (2, "other text".into())]).expect("saves");
    // Once into the database file itself, then a newer copy into the WAL.
    store.checkpoint().expect("checkpoint");
    store.save_scrollbacks(&[(1, format!("{}more\n", scrollback_with_marker()))]).expect("saves");
    assert!(!on_disk(&dir, MARKER).is_empty(), "the test sees the text while the terminal is open");

    // Terminal 1 is closed: its window is saved without it.
    store.save_window(&window(1, vec![session(2, "/b")])).expect("saves");
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new());
    assert_eq!(store.load().expect("loads")[0].sessions[0].scrollback.as_deref(), Some("other text"));
}

#[test]
fn a_closed_windows_text_is_erased_too() {
    let dir = temp_dir("erase-window");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    store.save_window(&window(2, vec![session(2, "/b")])).expect("saves");
    store.save_scrollbacks(&[(2, scrollback_with_marker())]).expect("saves");
    store.checkpoint().expect("checkpoint");
    store.delete_window(2).expect("deletes");
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new());
}

#[test]
fn secure_delete_is_on() {
    let dir = temp_dir("secure-delete");
    let store = Store::open_at(&dir).expect("opens");
    assert_eq!(count(&store, "PRAGMA secure_delete"), 1);
}

#[test]
fn clearing_a_terminal_erases_its_saved_text_at_once() {
    let dir = temp_dir("erase-cleared");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
    store.save_scrollbacks(&[(1, scrollback_with_marker()), (2, "other text".into())]).expect("saves");
    store.checkpoint().expect("checkpoint");

    store.clear_scrollback(1).expect("clears");
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new());
    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].sessions[0].scrollback, None, "nothing comes back after a restart");
    assert_eq!(loaded[0].sessions[1].scrollback.as_deref(), Some("other text"), "only that terminal's");
    // The next save stores what the terminal shows now.
    assert_eq!(store.save_scrollbacks(&[(1, "$ ")]).expect("saves"), 1);
    assert_eq!(store.load().expect("loads")[0].sessions[0].scrollback.as_deref(), Some("$ "));
    // A terminal with nothing saved is no error.
    store.clear_scrollback(99).expect("clears nothing");
}

#[test]
fn text_an_older_version_deleted_is_erased_at_the_first_open() {
    let dir = temp_dir("erase-older");
    std::fs::create_dir_all(&dir).expect("dir");
    // A database as 1.2.0 left it: rows deleted without `secure_delete`, a WAL never truncated.
    {
        let conn = rusqlite::Connection::open(dir.join("wormhole.db")).expect("opens");
        conn.pragma_update(None, "journal_mode", "WAL").expect("wal");
        conn.pragma_update(None, "secure_delete", false).expect("off");
        conn.execute_batch("CREATE TABLE old_text (text TEXT)").expect("table");
        // More free pages than creating today's tables takes back, so some keep the text.
        for _ in 0..40 {
            conn.execute("INSERT INTO old_text VALUES (?1)", [scrollback_with_marker()]).expect("inserts");
        }
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(())).expect("checkpoint");
        conn.execute("DELETE FROM old_text", []).expect("deletes");
        conn.execute_batch("DROP TABLE old_text").expect("drops");
        // Left without a clean close, as a crash would: the WAL keeps its frames.
        std::mem::forget(conn);
    }
    assert!(!on_disk(&dir, MARKER).is_empty(), "the old database still holds the text");
    let store = Store::open_at(&dir).expect("opens");
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new());
    assert_eq!(store.meta("scrubbed").expect("reads").as_deref(), Some("1"), "only once");
}
