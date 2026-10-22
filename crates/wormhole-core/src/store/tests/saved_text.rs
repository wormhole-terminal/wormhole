//! The terminals' saved text in its own file: moved there from the main database, sealed, out of
//! backups, started again when damaged.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::erasure::on_disk;
use super::migrations::empty_v6_database;
use super::*;
use crate::store::saved_text::platform::Platform;

const MARKER: &str = "SECRET-MARKER-9";

/// The operating system, played by the test (never the real Keychain): a fixed key, or none, as a
/// Keychain that is locked or was denied gives; and what it was asked to keep out of backups.
#[derive(Default)]
pub(super) struct FakePlatform {
    /// `None`: no key to be had.
    pub key: Option<[u8; 32]>,
    pub key_asked: AtomicUsize,
    pub excluded: Mutex<Vec<PathBuf>>,
}

impl FakePlatform {
    pub fn with_key(byte: u8) -> Arc<Self> {
        Arc::new(Self { key: Some([byte; 32]), ..Self::default() })
    }

    pub fn without_key() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl Platform for FakePlatform {
    fn text_key(&self, _: &Path) -> anyhow::Result<zeroize::Zeroizing<[u8; 32]>> {
        self.key_asked.fetch_add(1, Ordering::Relaxed);
        self.key.map(zeroize::Zeroizing::new).ok_or_else(|| anyhow::anyhow!("the Keychain said no"))
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

/// The real call, on the file the store makes (macOS only: it has Time Machine).
#[cfg(target_os = "macos")]
#[test]
fn the_real_file_carries_the_backup_exclusion() {
    let dir = temp_dir("saved-xattr");
    let _store = Store::open_at(&dir).expect("opens");
    assert!(crate::store::saved_text::platform::is_excluded_from_backup(&dir.join("scrollback.db")));
    assert!(!crate::store::saved_text::platform::is_excluded_from_backup(&dir.join("wormhole.db")));
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
