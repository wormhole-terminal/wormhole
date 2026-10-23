//! The terminals' saved text in its own file: moved there from the main database, sealed, out of
//! backups, started again when damaged.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::erasure::on_disk;
use super::migrations::empty_v6_database;
use super::*;
use crate::storage::store::saved_text::platform::Platform;
use rusqlite::Connection;

const MARKER: &str = "SECRET-MARKER-9";

/// The operating system, played by the test (never the real Keychain): a fixed key, or none, as a
/// Keychain that is locked or was denied gives; and what it was asked to keep out of backups.
#[derive(Default)]
pub(super) struct FakePlatform {
    /// `None`: no key to be had.
    pub key: Option<[u8; 32]>,
    pub key_asked: AtomicUsize,
    /// The accounts asked for, and thrown away.
    pub accounts: Mutex<Vec<String>>,
    pub forgotten: Mutex<Vec<String>>,
    pub excluded: Mutex<Vec<PathBuf>>,
}

impl FakePlatform {
    pub(super) fn with_key(byte: u8) -> Arc<Self> {
        Arc::new(Self { key: Some([byte; 32]), ..Self::default() })
    }

    pub(super) fn without_key() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl Platform for FakePlatform {
    fn text_key(&self, _: &Path, account: &str) -> anyhow::Result<zeroize::Zeroizing<[u8; 32]>> {
        self.key_asked.fetch_add(1, Ordering::Relaxed);
        self.accounts.lock().expect("lock").push(account.to_string());
        self.key.map(zeroize::Zeroizing::new).ok_or_else(|| anyhow::anyhow!("the Keychain said no"))
    }

    fn forget_key(&self, _: &Path, account: &str) -> anyhow::Result<()> {
        self.forgotten.lock().expect("lock").push(account.to_string());
        Ok(())
    }

    fn exclude_from_backup(&self, path: &Path) -> anyhow::Result<()> {
        self.excluded.lock().expect("lock").push(path.to_path_buf());
        Ok(())
    }
}

/// A database as a version 6 build left it, with a terminal whose text spills into overflow pages.
fn old_database_with_text(dir: &Path) {
    let conn = empty_v6_database(dir);
    conn.execute_batch(
        "INSERT INTO windows (id, position, active) VALUES (1, 0, 0);
         INSERT INTO sessions (id, window_id, position, name) VALUES (1, 1, 0, 'kept');
         INSERT INTO tabs (id, session_id, tab_id, position, cwd) VALUES (1, 1, 1, 0, '/a');",
    )
    .expect("rows");
    conn.execute("INSERT INTO scrollback (tab_id, hash, text) VALUES (1, 7, ?1)", [text_with_marker()])
        .expect("text");
}

fn text_with_marker() -> String {
    format!("$ cat .env\nTOKEN={MARKER}\n").repeat(400)
}

#[test]
fn an_upgrade_moves_the_text_to_its_own_file_and_erases_it_from_the_main_one() {
    let dir = temp_dir("saved-moves");
    old_database_with_text(&dir);
    assert_eq!(on_disk(&dir, MARKER), ["wormhole.db"]);

    let store = Store::open_at(&dir).expect("upgrades");
    let windows = store.load().expect("loads");
    assert_eq!(windows[0].sessions[0].scrollback.as_deref(), Some(text_with_marker().as_str()));
    // Sealed in its own file, and gone from the main one.
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new());
    assert!(std::fs::metadata(dir.join("scrollback.db")).expect("the file").len() > 4 * 4096);
    // Nor in the copy kept from before the upgrade.
    for entry in std::fs::read_dir(&dir).expect("lists") {
        let path = entry.expect("entry").path();
        let name = path.file_name().expect("name").to_string_lossy().into_owned();
        if name.starts_with("wormhole.db.") {
            let bytes = std::fs::read(&path).expect("reads");
            assert!(!bytes.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()), "in {name}");
        }
    }
    assert_eq!(count(&store, "SELECT COUNT(*) FROM sqlite_master WHERE name = 'scrollback'"), 0);
}

#[test]
fn the_file_and_its_journal_are_kept_out_of_backups() {
    let dir = temp_dir("saved-excluded");
    let platform = FakePlatform::with_key(1);
    let mut store = Store::open_in(&dir, platform.clone()).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    store.save_scrollbacks(&[(1, "text")]).expect("saves");
    let excluded = platform.excluded.lock().expect("lock").clone();
    for name in ["scrollback.db", "scrollback.db-wal", "scrollback.db-shm"] {
        assert!(excluded.contains(&dir.join(name)), "{name} in {excluded:?}");
    }
    assert!(!excluded.iter().any(|path| path.ends_with("wormhole.db")), "the layout stays in backups");
}

/// The real call, on the file the store makes (it does nothing where there is no Time Machine).
#[test]
fn the_real_file_carries_the_backup_exclusion() {
    let dir = temp_dir("saved-xattr");
    let _store = Store::open_at(&dir).expect("opens");
    assert!(crate::platform::backup::is_excluded(&dir.join("scrollback.db")));
    assert!(!crate::platform::backup::is_excluded(&dir.join("wormhole.db")));
}

#[test]
fn a_damaged_text_file_is_started_again_and_the_rest_is_kept() {
    let dir = temp_dir("saved-damaged");
    {
        let mut store = Store::open_at(&dir).expect("opens");
        store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
        store.save_scrollbacks(&[(1, "text")]).expect("saves");
    }
    std::fs::write(dir.join("scrollback.db"), b"not a database, not at all, not even close").expect("writes");
    for name in ["scrollback.db-wal", "scrollback.db-shm"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    let mut store = Store::open_at(&dir).expect("opens");
    let windows = store.load().expect("loads");
    assert_eq!(windows[0].sessions[0].cwd.as_deref(), Some("/a"), "the terminal is kept");
    assert_eq!(windows[0].sessions[0].scrollback, None, "its text is gone");
    assert_eq!(store.save_scrollbacks(&[(1, "again")]).expect("saves"), 1);
}

#[test]
fn text_of_a_terminal_the_main_database_lost_is_forgotten_at_open() {
    let dir = temp_dir("saved-orphans");
    {
        let mut store = Store::open_at(&dir).expect("opens");
        store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
        store.save_scrollbacks(&[(1, "one"), (2, "two")]).expect("saves");
        // As if the app died between deleting the terminal and its text.
        store
            .conn
            .execute_batch("DELETE FROM main.sessions WHERE id = 2; DELETE FROM main.tabs WHERE id = 2;")
            .expect("deletes");
    }
    let store = Store::open_at(&dir).expect("opens");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM saved.scrollback"), 1);
}

#[test]
fn without_a_key_an_upgrade_drops_the_text_rather_than_move_it_unsealed() {
    let dir = temp_dir("saved-moves-no-key");
    old_database_with_text(&dir);
    let store = Store::open_in(&dir, FakePlatform::without_key()).expect("upgrades");
    let windows = store.load().expect("loads");
    assert_eq!(windows[0].sessions[0].cwd.as_deref(), Some("/a"), "the terminal is kept");
    assert_eq!(windows[0].sessions[0].scrollback, None);
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new());
    assert_eq!(count(&store, "SELECT COUNT(*) FROM saved.scrollback"), 0);
}

/// Windows, a terminal with a note and a task, and its text: what must survive whatever happens to
/// the text's file.
fn layout_with_text(store: &mut Store) {
    let mut first = session(1, "/a");
    first.notes = vec![note("a note")];
    first.tasks = vec![task("a task")];
    store.save_window(&window(1, vec![first])).expect("saves");
    store.save_scrollbacks(&[(1, "text")]).expect("saves");
}

fn assert_layout_kept_and_saving_works(store: &mut Store) {
    let windows = store.load().expect("loads");
    let first = &windows[0].sessions[0];
    assert_eq!(first.cwd.as_deref(), Some("/a"));
    assert_eq!((first.notes[0].body.as_str(), first.tasks[0].text.as_str()), ("a note", "a task"));
    store.save_window(&windows[0]).expect("the layout still saves");
    store.save_scrollbacks(&[(1, "after")]).expect("text saves without an error");
}

#[test]
fn an_unwritable_text_file_is_started_again_and_costs_nothing_else() {
    let dir = temp_dir("saved-read-only");
    layout_with_text(&mut Store::open_at(&dir).expect("opens"));
    for name in ["scrollback.db-wal", "scrollback.db-shm"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    let file = dir.join("scrollback.db");
    std::fs::set_permissions(&file, crate::platform::fs::permissions(0o400)).expect("chmod");
    let mut store = Store::open_at(&dir).expect("opens");
    assert!(store.recovered_from().is_none(), "the main database is never set aside for it");
    assert_layout_kept_and_saving_works(&mut store);
    assert_eq!(store.load().expect("loads")[0].sessions[0].scrollback.as_deref(), Some("after"));
}

#[test]
fn a_text_file_that_cant_be_replaced_falls_back_to_memory() {
    let dir = temp_dir("saved-in-memory");
    layout_with_text(&mut Store::open_at(&dir).expect("opens"));
    // Something SQLite can't open and the store can't delete: a folder in its place.
    for name in ["scrollback.db", "scrollback.db-wal", "scrollback.db-shm"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    std::fs::create_dir_all(dir.join("scrollback.db").join("inside")).expect("dir");
    let mut store = Store::open_at(&dir).expect("opens");
    assert!(store.recovered_from().is_none());
    assert_layout_kept_and_saving_works(&mut store);
    drop(store);
    assert!(dir.join("scrollback.db").is_dir(), "left as it was");
    assert!(dir.join("wormhole.db").is_file());
}

#[test]
fn a_text_file_that_refuses_the_move_doesnt_stop_the_upgrade() {
    let dir = temp_dir("saved-move-fails");
    old_database_with_text(&dir);
    Connection::open(dir.join("wormhole.db"))
        .expect("opens")
        .execute_batch(
            "INSERT INTO notes (session_id, position, body) VALUES (1, 0, 'a note');
             INSERT INTO tasks (session_id, position, text) VALUES (1, 0, 'a task');",
        )
        .expect("notes");
    // A text file of today's layout that refuses every row.
    {
        let saved = Connection::open(dir.join("scrollback.db")).expect("opens");
        saved
            .execute_batch(
                "CREATE TABLE scrollback (tab_id INTEGER PRIMARY KEY, sealed BLOB NOT NULL);
                 CREATE TRIGGER refuse BEFORE INSERT ON scrollback BEGIN SELECT RAISE(ABORT, 'disk full'); END;
                 PRAGMA user_version = 1;",
            )
            .expect("layout");
    }
    let mut store = Store::open_in(&dir, FakePlatform::with_key(1)).expect("upgrades");
    assert!(store.recovered_from().is_none());
    assert_eq!(count(&store, "PRAGMA user_version"), 9);
    assert_eq!(count(&store, "SELECT COUNT(*) FROM saved.scrollback"), 0, "the text is dropped");
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new());
    store.conn.execute_batch("DROP TRIGGER saved.refuse").expect("drops");
    let windows = store.load().expect("loads");
    assert_eq!(windows[0].sessions[0].notes[0].body, "a note");
    assert_eq!(windows[0].sessions[0].tasks[0].text, "a task");
    assert_eq!(store.save_scrollbacks(&[(1, "after")]).expect("saves"), 1);
}

#[test]
fn the_key_is_kept_under_a_random_id_and_thrown_away_on_erase() {
    let dir = temp_dir("saved-account");
    let platform = FakePlatform::with_key(1);
    let mut store = Store::open_in(&dir, platform.clone()).expect("opens");
    layout_with_text(&mut store);
    let account = store.meta("saved_text_key_id").expect("reads").expect("an id");
    assert_eq!(account.len(), 32);
    assert!(account.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(platform.accounts.lock().expect("lock").as_slice(), std::slice::from_ref(&account));
    drop(store);
    // The same id across launches.
    let store = Store::open_in(&dir, platform.clone()).expect("opens");
    assert_eq!(store.meta("saved_text_key_id").expect("reads"), Some(account.clone()));
    assert_eq!(store.erase_saved_text().expect("erases"), 1);
    assert_eq!(platform.forgotten.lock().expect("lock").as_slice(), [account]);
}

/// A profile being deleted takes its key with it (here the development key file; in a release build
/// the Keychain item).
#[test]
fn a_deleted_profiles_key_goes_with_it() {
    let base = tempfile::tempdir().expect("temp dir");
    let data = base.path().join("Wormhole");
    let paths = crate::storage::paths::DataRoot::new(&data).default_profile();
    layout_with_text(&mut Store::open_for(&paths).expect("opens"));
    assert!(data.join("scrollback-key.dev-only").is_file());
    Store::forget_text_key_for(&paths);
    assert!(!data.join("scrollback-key.dev-only").exists());
}
