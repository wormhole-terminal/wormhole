//! The terminals' saved text (what comes back after a restart), in a database file of its own
//! beside the profile's: `scrollback.db`, attached to the store's connection as `saved`.
//!
//! Apart because it is the one sensitive, large and churning part: the file is kept out of backups
//! (Time Machine) while the windows, notes and tasks in `wormhole.db` stay in them, a minute's save
//! no longer grows the main database's WAL, and erasing all of it touches nothing else. Nothing
//! but this text is in it, so a damaged file is simply started again.
//!
//! Every record is sealed (`cipher`) with the profile's key (`keys`: the login Keychain in a
//! release build): a program that copies the file gets nothing it can read. Without the key (no
//! Keychain, access denied) the text is not kept; it is never written unsealed. Nothing about the
//! text is stored beside it, not even a fingerprint: whether a terminal's text changed since it was
//! last written is remembered in memory ([`Keeper`]).
//!
//! One connection serves both files, so a statement can name `main.tabs` and `saved.scrollback`
//! together; a foreign key can't reach across files, so every delete of a terminal deletes its text
//! here too, and rows left behind (a crash between the two) go at the next open.

mod cipher;
mod keys;
pub(crate) mod platform;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context as _, Result};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use self::cipher::Cipher;
pub(super) use self::keys::DEV_KEY_FILE;
use self::platform::Platform;
use super::with_suffix;

/// The file, in the profile's folder.
pub(super) const FILE: &str = "scrollback.db";
/// The name the file is attached under.
const SCHEMA_NAME: &str = "saved";

/// The file's layout (its `user_version`). A file with another one is started again: what it holds
/// is only ever a minute's save away.
const LAYOUT_VERSION: i64 = 1;
const SCHEMA: &str = "
    CREATE TABLE saved.scrollback (
        tab_id INTEGER PRIMARY KEY,
        sealed BLOB NOT NULL
    );
";

/// The sealing side of a profile's saved text: its key, asked for once per launch (the Keychain
/// may ask the user), and what was last written for each terminal.
pub(super) struct Keeper {
    platform: Arc<dyn Platform>,
    dir: PathBuf,
    cipher: OnceLock<Option<Cipher>>,
    /// Each terminal's text as last written or read, as a fingerprint: saving an unchanged text
    /// costs a hash, not a seal and a write.
    written: Mutex<HashMap<i64, u64>>,
}

impl Keeper {
    pub fn new(platform: Arc<dyn Platform>, dir: &Path) -> Self {
        Self { platform, dir: dir.to_path_buf(), cipher: OnceLock::new(), written: Mutex::default() }
    }

    pub fn platform(&self) -> &dyn Platform {
        self.platform.as_ref()
    }

    /// The cipher, once the key is at hand; `None` when it can't be (said once in the log).
    fn cipher(&self) -> Option<&Cipher> {
        self.cipher
            .get_or_init(|| match self.platform.text_key(&self.dir).and_then(|key| Cipher::new(&key)) {
                Ok(cipher) => Some(cipher),
                Err(err) => {
                    tracing::warn!("terminal text is not kept for a restart: {err:#}");
                    None
                }
            })
            .as_ref()
    }

    /// `text` sealed for terminal `tab_id`, or `None` when there is no key.
    pub fn seal(&self, tab_id: i64, text: &str) -> Result<Option<Vec<u8>>> {
        self.cipher().map(|cipher| cipher.seal(tab_id, text)).transpose()
    }

    /// The text of a record, remembered as written; `None` (and a warning, without the text) when it
    /// doesn't open: another key (the Keychain item was removed, a development build wrote it), or
    /// a changed record. The next save writes it again.
    pub fn open(&self, tab_id: i64, record: &[u8]) -> Option<String> {
        let cipher = self.cipher()?;
        match cipher.open(tab_id, record) {
            Ok(text) => {
                self.remember(tab_id, fingerprint(&text));
                Some(text)
            }
            Err(err) => {
                tracing::warn!("terminal {tab_id}'s saved text can't be read: {err:#}");
                None
            }
        }
    }

    /// Whether `print` is what was last written for `tab_id`.
    pub fn is_written(&self, tab_id: i64, print: u64) -> bool {
        self.written().get(&tab_id) == Some(&print)
    }

    pub fn remember(&self, tab_id: i64, print: u64) {
        self.written().insert(tab_id, print);
    }

    pub fn forget(&self, tab_id: i64) {
        self.written().remove(&tab_id);
    }

    /// After terminals were deleted: the next save writes every text again rather than trust a
    /// fingerprint of a terminal whose id may come back.
    pub fn forget_all(&self) {
        self.written().clear();
    }

    fn written(&self) -> std::sync::MutexGuard<'_, HashMap<i64, u64>> {
        // Only fingerprints: one a panic left half-written costs a write, nothing else.
        self.written.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// A 64-bit fingerprint of a text, kept in memory only to skip a save: a collision, one in 2^64,
/// would keep the older text until the next change.
pub(super) fn fingerprint(text: &str) -> u64 {
    let digest = Sha256::digest(text.as_bytes());
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(head)
}

/// Attach the saved text's file in `dir` to `conn` (creating it owner-only), and keep it and its
/// journal files out of backups. A file that isn't a database, or a damaged one, is deleted and
/// started fresh: it held nothing a minute's save doesn't write again.
pub(super) fn attach(conn: &Connection, dir: &Path, platform: &dyn Platform) -> Result<()> {
    let path = dir.join(FILE);
    match attach_file(conn, &path) {
        Err(err) if super::is_damaged(&err) => {
            tracing::warn!("{err:#}; starting the saved terminal text again in {}", path.display());
            // Detaching can fail when the attach itself did; either way the files go.
            let _ = conn.execute_batch("DETACH DATABASE saved");
            remove_files(&path)?;
            attach_file(conn, &path).context("starting the saved terminal text again")?;
        }
        result => result?,
    }
    for file in files(&path) {
        if file.exists()
            && let Err(err) = platform.exclude_from_backup(&file)
        {
            tracing::warn!("{err:#}");
        }
    }
    Ok(())
}

fn attach_file(conn: &Connection, path: &Path) -> Result<()> {
    // As the main database: owner-only before SQLite opens it, so its journal files follow.
    for (ix, file) in files(path).into_iter().enumerate() {
        let made = if ix == 0 {
            crate::fsutil::create_private_file(&file)
        } else {
            crate::fsutil::make_private(&file)
        };
        if let Err(err) = made {
            tracing::warn!("couldn't make {} owner-only: {err}", file.display());
        }
    }
    let name = path.to_str().with_context(|| format!("{} is not UTF-8", path.display()))?;
    conn.execute("ATTACH DATABASE ?1 AS saved", [name])
        .with_context(|| format!("opening {}", path.display()))?;
    let db = SCHEMA_NAME;
    conn.pragma_update(Some(db), "journal_mode", "WAL")?;
    conn.pragma_update(Some(db), "journal_size_limit", 8 * 1024 * 1024)?;
    // A closed terminal's text is overwritten, not left in free pages.
    conn.pragma_update(Some(db), "secure_delete", true)?;
    let verdict: String = conn.query_row("PRAGMA saved.quick_check", [], |row| row.get(0))?;
    if verdict != "ok" {
        return Err(super::Damaged(verdict).into());
    }
    let version: i64 = conn.query_row("PRAGMA saved.user_version", [], |row| row.get(0))?;
    if version != LAYOUT_VERSION {
        // A new file, or one another build laid out otherwise.
        conn.execute_batch("DROP TABLE IF EXISTS saved.scrollback")?;
        conn.execute_batch(SCHEMA).context("creating the saved terminal text's table")?;
        conn.pragma_update(Some(SCHEMA_NAME), "user_version", LAYOUT_VERSION)?;
    }
    Ok(())
}

/// The file and its journal files.
fn files(path: &Path) -> [std::path::PathBuf; 3] {
    [path.to_path_buf(), with_suffix(path, "-wal"), with_suffix(path, "-shm")]
}

fn remove_files(path: &Path) -> Result<()> {
    for file in files(path) {
        match std::fs::remove_file(&file) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                return Err(err).with_context(|| format!("removing {}", file.display()));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Text of terminals the main database no longer has (a crash between the two deletes).
pub(super) fn forget_orphans(conn: &Connection) -> Result<usize> {
    Ok(conn.execute("DELETE FROM saved.scrollback WHERE tab_id NOT IN (SELECT id FROM main.tabs)", [])?)
}

/// What the step to schema 9 set aside from the main database's own `scrollback` table (in
/// `temp.v9_text`, in memory) is sealed and moved into this file, inside the upgrade's transaction.
/// Without a key it is dropped, never moved unsealed. Nothing to do when no step put it there.
pub(super) fn take_moved(conn: &Connection, keeper: &Keeper) -> Result<()> {
    let set_aside: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM temp.sqlite_master WHERE type = 'table' AND name = 'v9_text'",
        [],
        |row| row.get(0),
    )?;
    if !set_aside {
        return Ok(());
    }
    let rows: Vec<(i64, String)> = conn
        .prepare("SELECT tab_id, text FROM temp.v9_text")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let (mut moved, mut dropped) = (0, 0);
    for (tab_id, text) in &rows {
        match keeper.seal(*tab_id, text)? {
            Some(sealed) => {
                conn.execute(
                    "INSERT OR REPLACE INTO saved.scrollback (tab_id, sealed) VALUES (?1, ?2)",
                    (tab_id, sealed),
                )
                .context("moving the saved terminal text to its own file")?;
                moved += 1;
            }
            None => dropped += 1,
        }
    }
    conn.execute_batch("DROP TABLE temp.v9_text")?;
    tracing::info!(
        "sealed and moved the saved text of {moved} terminals to {FILE} ({dropped} without a key not kept)"
    );
    Ok(())
}

/// Take the terminals' text out of a copy of the main database (the one kept before an upgrade):
/// the copy is there to go back to the windows and terminals as they were, and the text, the one
/// secret part, must not outlive the move to its own file in it. Deleted with `secure_delete`, the
/// journal kept in memory and the file rewritten, so none of it stays in free pages.
pub(super) fn strip_copy(copy: &Path) -> Result<()> {
    strip(copy).with_context(|| format!("taking the terminals' text out of {}", copy.display()))
}

fn strip(copy: &Path) -> Result<()> {
    let conn = Connection::open(copy).with_context(|| format!("opening {}", copy.display()))?;
    conn.pragma_update(None, "secure_delete", true)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "journal_mode", "MEMORY")?;
    let has_text: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = 'scrollback'",
        [],
        |row| row.get(0),
    )?;
    if has_text {
        // Emptied, not dropped: the build before expects the table.
        conn.execute("DELETE FROM scrollback", [])?;
        conn.execute_batch("VACUUM")?;
    }
    Ok(())
}
