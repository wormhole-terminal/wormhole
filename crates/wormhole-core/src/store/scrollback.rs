//! Terminal scrollback, one per terminal (a `tabs` row, which has its session's id), kept sealed in a
//! file of its own (`saved_text`) so saving a window never touches it.

use anyhow::{Context as _, Result};
use rusqlite::Connection;

use super::Store;
use super::saved_text::{Keeper, fingerprint};

impl Store {
    /// Save one terminal's scrollback (`tab_id`: its id, its session's, what hooks report). Prefer
    /// `save_scrollbacks` for more than one: each call here is a transaction of its own.
    pub fn save_scrollback(&self, tab_id: i64, text: &str) -> Result<()> {
        if let Some(print) = write_scrollback(&self.conn, &self.saved, tab_id, text)? {
            self.saved.remember(tab_id, print);
        }
        Ok(())
    }

    /// Save the scrollback of several panes in one transaction (all or none), and only for
    /// those whose text differs from what was last written: a pane nothing was printed in since the
    /// last save costs a hash. Panes the store has no row for (opened since the last window save)
    /// are skipped, as `save_scrollback` always did, and so is everything when there is no key
    /// (`saved_text`). Returns how many were written.
    pub fn save_scrollbacks<S: AsRef<str>>(&mut self, items: &[(i64, S)]) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction()?;
        let mut written = Vec::new();
        for (tab_id, text) in items {
            if let Some(print) = write_scrollback(&tx, &self.saved, *tab_id, text.as_ref())? {
                written.push((*tab_id, print));
            }
        }
        tx.commit()?;
        // Only once it is on disk: a batch that failed is written again next time.
        for (tab_id, print) in &written {
            self.saved.remember(*tab_id, *print);
        }
        Ok(written.len())
    }

    /// Forget one terminal's saved text now (the user cleared it with ⌘K), and take it off the disk
    /// (`secure_delete`, then a checkpoint) rather than leave it until the next minute's save
    /// overwrites it. The next save stores what the terminal shows from then on.
    pub fn clear_scrollback(&self, tab_id: i64) -> Result<()> {
        self.saved.forget(tab_id);
        let removed = super::exec(&self.conn, "DELETE FROM saved.scrollback WHERE tab_id = ?1", [tab_id])?;
        if removed > 0 {
            self.checkpoint_after_delete();
        }
        Ok(())
    }

    /// Erase every terminal's saved text (Restore terminal text after restart turned off): the rows
    /// go with `secure_delete`, the file is rewritten without its free pages and its WAL emptied.
    /// Returns how many terminals had text.
    pub fn erase_saved_text(&self) -> Result<usize> {
        self.saved.forget_all();
        let removed = super::exec(&self.conn, "DELETE FROM saved.scrollback", [])?;
        if removed > 0 {
            self.conn.execute_batch("VACUUM saved").context("rewriting the saved text's file")?;
            self.checkpoint()?;
        }
        Ok(removed)
    }

    /// The text of a terminal's sealed record, as `load` gives it.
    pub(super) fn open_scrollback(&self, tab_id: i64, sealed: Option<Vec<u8>>) -> Option<String> {
        self.saved.open(tab_id, &sealed?)
    }
}

/// The fingerprint of what was written, `None` when nothing was (unchanged, no row for the terminal,
/// no key). The caller provides the transaction (or the autocommit) and remembers the fingerprint
/// once it is committed.
fn write_scrollback(conn: &Connection, saved: &Keeper, tab_id: i64, text: &str) -> Result<Option<u64>> {
    let print = fingerprint(text);
    if saved.is_written(tab_id, print) {
        return Ok(None);
    }
    let Some(sealed) = saved.seal(tab_id, text)? else { return Ok(None) };
    // `WHERE EXISTS` is the pane check (no foreign key reaches across files), and keeps out a
    // terminal whose text the person asked not to keep; it is also what lets SQLite parse an upsert
    // on an INSERT ... SELECT.
    let changed = conn
        .prepare_cached(
            "INSERT INTO saved.scrollback (tab_id, sealed)
             SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM main.tabs t JOIN main.sessions s ON s.id = t.session_id
                                         WHERE t.id = ?1 AND NOT s.forget_text)
             ON CONFLICT(tab_id) DO UPDATE SET sealed = excluded.sealed",
        )?
        .execute((tab_id, sealed))?;
    Ok((changed > 0).then_some(print))
}
