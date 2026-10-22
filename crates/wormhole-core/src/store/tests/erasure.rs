//! Deleted terminal text leaves the disk: not in the database's free pages, not in the WAL.

use super::*;

const MARKER: &str = "SECRET-MARKER-1";

/// Whether `needle` is anywhere in the raw bytes of the database or its WAL.
fn on_disk(dir: &std::path::Path, needle: &str) -> Vec<String> {
    ["wormhole.db", "wormhole.db-wal"]
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
fn deleted_rows_are_overwritten() {
    let dir = temp_dir("secure-delete");
    let store = Store::open_at(&dir).expect("opens");
    assert_eq!(count(&store, "PRAGMA secure_delete"), 1);
}
