//! Writing one session: its row, terminal, notes and tasks.
//!
//! Everything goes through the connection's statement cache, and a row that already holds
//! what is being saved is left alone (the upserts carry a `WHERE ... IS NOT` guard; notes and
//! tasks are compared before they are replaced), so saving a window in which one note changed
//! writes that session's notes and nothing else.

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

use super::records::{NoteRecord, SessionRecord, TaskRecord, is_blank};
use super::{Store, exec};

impl Store {
    /// Save one session of `window_id` (its row, terminal, notes and tasks) and nothing else: other
    /// sessions, the window's own row and its splits (the layouts) are untouched. A session the
    /// store has not seen goes last in the window, one it has keeps its place (see
    /// `order_sessions`); a session saved under a different window than before moves. Panes the
    /// session no longer lists are forgotten with their scrollback; scrollback itself is saved
    /// with `save_scrollbacks`.
    pub fn save_session(&mut self, window_id: i64, session: &SessionRecord) -> Result<()> {
        let tx = self.conn.transaction()?;
        write_session(&tx, window_id, None, session)?;
        tx.commit()?;
        Ok(())
    }

    /// Save one session right now, in a transaction of its own: what the app calls when it has just
    /// started a session host for a terminal, so the row that names the host (`host_id`) is on disk
    /// before anything can go wrong, not on the next window save. It is [`Store::save_session`]
    /// (same row, same guards); it only sets `host_id`, so the rest of the record should be the
    /// session as it is.
    pub fn save_session_host(
        &mut self,
        window_id: i64,
        session: &SessionRecord,
        host_id: &str,
    ) -> Result<()> {
        let mut record = session.clone();
        record.host_id = Some(host_id.to_string());
        self.save_session(window_id, &record)
    }

    /// Every terminal that names a session host: `(session id, host id)`, for matching the hosts
    /// found on disk against the rows (a live host nobody names is an orphan).
    pub fn host_ids(&self) -> Result<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT session_id, host_id FROM tabs WHERE host_id IS NOT NULL ORDER BY session_id",
        )?;
        let rows =
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// Put the window's sessions in this order (`order[0]` first). Ids that are not the window's
    /// are ignored, and sessions not listed keep their old position numbers, so list every session.
    pub fn order_sessions(&mut self, window_id: i64, order: &[i64]) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (position, id) in order.iter().enumerate() {
            exec(
                &tx,
                "UPDATE sessions SET position = ?1 WHERE id = ?2 AND window_id = ?3 AND position IS NOT ?1",
                params![position as i64, id, window_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

/// `position`: where the session goes, or `None` to keep the place it has in this window (a new
/// session goes last).
pub(super) fn write_session(
    conn: &Connection,
    window_id: i64,
    position: Option<usize>,
    session: &SessionRecord,
) -> Result<()> {
    let position = match position {
        Some(position) => position as i64,
        None => existing_or_next_position(conn, window_id, session.id)?,
    };
    // `active_tab` is a column a later step drops: a session never selects a tab now.
    exec(
        conn,
        "INSERT INTO sessions (id, window_id, position, name, tasks_collapsed, notes_collapsed, pinned, split_id,
                               forget_text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET window_id = ?2, position = ?3, name = ?4, tasks_collapsed = ?5,
                                       notes_collapsed = ?6, pinned = ?7, split_id = ?8, forget_text = ?9
         WHERE window_id IS NOT ?2 OR position IS NOT ?3 OR name IS NOT ?4 OR tasks_collapsed IS NOT ?5
            OR notes_collapsed IS NOT ?6 OR pinned IS NOT ?7 OR split_id IS NOT ?8 OR forget_text IS NOT ?9",
        params![
            session.id,
            window_id,
            position,
            session.name,
            session.tasks_collapsed,
            session.notes_collapsed,
            session.pinned,
            session.split_id,
            session.forget_text,
        ],
    )?;
    if session.forget_text {
        exec(conn, "DELETE FROM saved.scrollback WHERE tab_id = ?1", [session.id])?;
    }
    write_terminal(conn, session)?;
    // A blank note or task is one still being written (or emptied to be thrown
    // away): it is never saved, so it can't come back as an empty entry.
    replace_notes(conn, session.id, session.notes.iter().filter(|n| !is_blank(&n.body)).collect())?;
    replace_tasks(conn, session.id, session.tasks.iter().filter(|t| !is_blank(&t.text)).collect())?;
    Ok(())
}

/// Forget a session with everything that belongs to it, its saved text in the other file too.
pub(super) fn delete_session_rows(conn: &Connection, id: i64) -> Result<()> {
    exec(
        conn,
        "DELETE FROM saved.scrollback WHERE tab_id IN (SELECT id FROM main.tabs WHERE session_id = ?1)",
        [id],
    )?;
    exec(conn, "DELETE FROM notes WHERE session_id = ?1", [id])?;
    exec(conn, "DELETE FROM tasks WHERE session_id = ?1", [id])?;
    exec(conn, "DELETE FROM row_claims WHERE session_id = ?1", [id])?;
    exec(conn, "DELETE FROM tabs WHERE session_id = ?1", [id])?;
    exec(conn, "DELETE FROM sessions WHERE id = ?1", [id])?;
    Ok(())
}

fn existing_or_next_position(conn: &Connection, window_id: i64, session_id: i64) -> Result<i64> {
    let existing: Option<(i64, i64)> = conn
        .prepare_cached("SELECT window_id, position FROM sessions WHERE id = ?")?
        .query_row([session_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()?;
    if let Some((window, position)) = existing
        && window == window_id
    {
        return Ok(position);
    }
    Ok(conn
        .prepare_cached("SELECT COALESCE(MAX(position) + 1, 0) FROM sessions WHERE window_id = ?")?
        .query_row([window_id], |row| row.get(0))?)
}

/// The session's terminal: one `tabs` row with the session's id. Any other row the session has
/// (from before it was one terminal) is deleted, its scrollback with it.
fn write_terminal(conn: &Connection, session: &SessionRecord) -> Result<()> {
    exec(
        conn,
        "DELETE FROM saved.scrollback
         WHERE tab_id IN (SELECT id FROM main.tabs WHERE session_id = ?1 AND id <> ?1)",
        [session.id],
    )?;
    exec(conn, "DELETE FROM tabs WHERE session_id = ?1 AND id <> ?1", [session.id])?;
    exec(
        conn,
        "INSERT INTO tabs (id, session_id, tab_id, position, cwd, shell, layout, host_id)
         VALUES (?1, ?1, ?1, 0, ?2, ?3, NULL, ?4)
         ON CONFLICT(id) DO UPDATE SET session_id = ?1, tab_id = ?1, position = 0, cwd = ?2, shell = ?3,
                                       layout = NULL, host_id = ?4
         WHERE session_id IS NOT ?1 OR tab_id IS NOT ?1 OR position IS NOT 0 OR cwd IS NOT ?2
            OR shell IS NOT ?3 OR layout IS NOT NULL OR host_id IS NOT ?4",
        params![session.id, session.cwd, session.shell, session.host_id],
    )?;
    Ok(())
}

/// Whether the rows `sql` returns (in order) are exactly `items`, judged by `same`.
fn stored_equals<T>(
    conn: &Connection,
    sql: &str,
    session_id: i64,
    items: &[T],
    same: impl Fn(&rusqlite::Row<'_>, &T) -> Result<bool>,
) -> Result<bool> {
    let mut stmt = conn.prepare_cached(sql)?;
    let mut rows = stmt.query([session_id])?;
    for item in items {
        match rows.next()? {
            Some(row) if same(row, item)? => {}
            _ => return Ok(false),
        }
    }
    Ok(rows.next()?.is_none())
}

fn replace_notes(conn: &Connection, session_id: i64, notes: Vec<&NoteRecord>) -> Result<()> {
    let sql = "SELECT body, collapsed FROM notes WHERE session_id = ? ORDER BY position";
    let same = |row: &rusqlite::Row<'_>, note: &&NoteRecord| -> Result<bool> {
        Ok(row.get_ref(0)?.as_str()? == note.body && row.get::<_, bool>(1)? == note.collapsed)
    };
    if stored_equals(conn, sql, session_id, &notes, same)? {
        return Ok(());
    }
    exec(conn, "DELETE FROM notes WHERE session_id = ?1", [session_id])?;
    for (position, note) in notes.iter().enumerate() {
        exec(
            conn,
            "INSERT INTO notes (session_id, position, body, collapsed) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, position as i64, note.body, note.collapsed],
        )?;
    }
    Ok(())
}

fn replace_tasks(conn: &Connection, session_id: i64, tasks: Vec<&TaskRecord>) -> Result<()> {
    let sql = "SELECT text, done FROM tasks WHERE session_id = ? ORDER BY position";
    let same = |row: &rusqlite::Row<'_>, task: &&TaskRecord| -> Result<bool> {
        Ok(row.get_ref(0)?.as_str()? == task.text && row.get::<_, bool>(1)? == task.done)
    };
    if stored_equals(conn, sql, session_id, &tasks, same)? {
        return Ok(());
    }
    exec(conn, "DELETE FROM tasks WHERE session_id = ?1", [session_id])?;
    for (position, task) in tasks.iter().enumerate() {
        exec(
            conn,
            "INSERT INTO tasks (session_id, position, text, done) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, position as i64, task.text, task.done],
        )?;
    }
    Ok(())
}
