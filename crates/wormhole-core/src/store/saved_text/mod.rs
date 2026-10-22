//! The terminals' saved text (what comes back after a restart), in a database file of its own
//! beside the profile's: `scrollback.db`, attached to the store's connection as `saved`.
//!
//! Apart because it is the one sensitive, large and churning part: the file is kept out of backups
//! (Time Machine) while the windows, notes and tasks in `wormhole.db` stay in them, a minute's save
//! no longer grows the main database's WAL, and erasing all of it touches nothing else. Nothing
//! but this text is in it, so a damaged file is simply started again.
//!
//! One connection serves both files, so a statement can name `main.tabs` and `saved.scrollback`
//! together; a foreign key can't reach across files, so every delete of a terminal deletes its text
//! here too, and rows left behind (a crash between the two) go at the next open.

pub(crate) mod platform;

use std::path::Path;

use anyhow::{Context as _, Result};
use rusqlite::Connection;

use self::platform::Platform;
use super::with_suffix;

/// The file, in the profile's folder.
pub(super) const FILE: &str = "scrollback.db";
/// The name the file is attached under.
const SCHEMA_NAME: &str = "saved";

/// `hash` comes before `text` on purpose: a column's bytes are reached by walking the record, so
/// reading the hash after a 300 KB text would walk the whole overflow chain.
const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS saved.scrollback (
        tab_id INTEGER PRIMARY KEY,
        hash INTEGER,
        text TEXT NOT NULL
    );
";

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
    conn.execute_batch(SCHEMA).context("creating the saved terminal text's table")?;
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
/// `temp.v9_text`, in memory) moves into this file, inside the upgrade's transaction. Nothing to do
/// when no step put it there.
pub(super) fn take_moved(conn: &Connection) -> Result<()> {
    let set_aside: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM temp.sqlite_master WHERE type = 'table' AND name = 'v9_text'",
        [],
        |row| row.get(0),
    )?;
    if !set_aside {
        return Ok(());
    }
    let moved = conn
        .execute(
            "INSERT OR REPLACE INTO saved.scrollback (tab_id, hash, text) SELECT tab_id, NULL, text FROM temp.v9_text",
            [],
        )
        .context("moving the saved terminal text to its own file")?;
    conn.execute_batch("DROP TABLE temp.v9_text")?;
    tracing::info!("moved the saved text of {moved} terminals to {FILE}");
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
