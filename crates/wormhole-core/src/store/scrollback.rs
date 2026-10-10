//! Terminal scrollback, one per terminal (a `tabs` row, which has its session's id), kept apart from
//! the rest of the row so saving a window never touches it.

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use super::Store;

impl Store {
    /// Save one terminal's scrollback (`tab_id`: its id, its session's, what hooks report). Prefer
    /// `save_scrollbacks` for more than one: each call here is a transaction of its own.
    pub fn save_scrollback(&self, tab_id: i64, text: &str) -> Result<()> {
        write_scrollback(&self.conn, tab_id, text)?;
        Ok(())
    }

    /// Save the scrollback of several panes in one transaction (all or none), and only for
    /// those whose text differs from what is stored: a pane nothing was printed in since the
    /// last save costs a hash and a one-row read, not a write. Panes the store has no row for
    /// (opened since the last window save) are skipped, as `save_scrollback` always did.
    /// Returns how many were written.
    pub fn save_scrollbacks<S: AsRef<str>>(&mut self, items: &[(i64, S)]) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction()?;
        let mut written = 0;
        for (tab_id, text) in items {
            written += usize::from(write_scrollback(&tx, *tab_id, text.as_ref())?);
        }
        tx.commit()?;
        Ok(written)
    }
}

/// Whether anything was written. The caller provides the transaction (or the autocommit).
fn write_scrollback(conn: &Connection, tab_id: i64, text: &str) -> Result<bool> {
    let hash = content_hash(text);
    let stored: Option<Option<i64>> = conn
        .prepare_cached("SELECT hash FROM scrollback WHERE tab_id = ?")?
        .query_row([tab_id], |row| row.get(0))
        .optional()?;
    if stored == Some(Some(hash)) {
        return Ok(false);
    }
    // `WHERE EXISTS` is the pane check (the foreign key would make a missing pane an error) and
    // also what lets SQLite parse an upsert on an INSERT ... SELECT.
    let changed = conn
        .prepare_cached(
            "INSERT INTO scrollback (tab_id, hash, text)
             SELECT ?1, ?2, ?3 WHERE EXISTS (SELECT 1 FROM tabs WHERE id = ?1)
             ON CONFLICT(tab_id) DO UPDATE SET hash = excluded.hash, text = excluded.text",
        )?
        .execute((tab_id, hash, text))?;
    Ok(changed > 0)
}

/// A stable 64-bit fingerprint of a text (it is stored, so it must not change between Rust
/// releases, which rules out `DefaultHasher`). Only used to skip a save: a collision, one in
/// 2^64, would keep the older text until the next change.
fn content_hash(text: &str) -> i64 {
    let digest = Sha256::digest(text.as_bytes());
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest[..8]);
    i64::from_le_bytes(head)
}
