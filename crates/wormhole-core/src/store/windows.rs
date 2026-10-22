//! Loading and saving windows (and, through `sessions`, what is in them).

use std::collections::HashSet;

use anyhow::Result;
use rusqlite::{Connection, params};

use super::records::{NoteRecord, SessionRecord, SplitRecord, TaskRecord, WindowRecord, is_blank};
use super::sessions::{delete_session_rows, write_session};
use super::{Store, exec};

impl Store {
    /// Every saved window, in the order they were first opened.
    pub fn load(&self) -> Result<Vec<WindowRecord>> {
        let conn = &self.conn;
        let mut windows = conn
            .prepare_cached("SELECT id, active, layout, bounds FROM windows ORDER BY position")?
            .query_map([], |row| {
                Ok(WindowRecord {
                    id: row.get(0)?,
                    active: row.get::<_, i64>(1)?.max(0) as usize,
                    layout: row.get(2)?,
                    bounds: row.get(3)?,
                    sessions: Vec::new(),
                    splits: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut sessions = conn.prepare_cached(
            "SELECT id, name, tasks_collapsed, notes_collapsed, pinned, split_id FROM sessions
             WHERE window_id = ? ORDER BY position",
        )?;
        let mut splits =
            conn.prepare_cached("SELECT id, layout FROM splits WHERE window_id = ? ORDER BY id")?;
        // The terminal is the row with the session's id (the only one, since schema 7).
        let mut terminal = conn.prepare_cached(
            "SELECT t.cwd, t.shell, s.text, t.host_id
             FROM main.tabs t LEFT JOIN saved.scrollback s ON s.tab_id = t.id
             WHERE t.session_id = ?1 ORDER BY t.id = ?1 DESC, t.position LIMIT 1",
        )?;
        let mut notes =
            conn.prepare_cached("SELECT body, collapsed FROM notes WHERE session_id = ? ORDER BY position")?;
        let mut tasks =
            conn.prepare_cached("SELECT text, done FROM tasks WHERE session_id = ? ORDER BY position")?;
        for window in &mut windows {
            window.sessions = sessions
                .query_map([window.id], |row| {
                    Ok(SessionRecord {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        tasks_collapsed: row.get(2)?,
                        notes_collapsed: row.get(3)?,
                        pinned: row.get(4)?,
                        split_id: row.get(5)?,
                        ..Default::default()
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for session in &mut window.sessions {
                let found = terminal
                    .query_map([session.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
                    .next()
                    .transpose()?;
                if let Some((cwd, shell, scrollback, host_id)) = found {
                    session.cwd = cwd;
                    session.shell = shell;
                    session.scrollback = scrollback;
                    session.host_id = host_id;
                }
                // Blank ones saved by an older version are left behind.
                session.notes = notes
                    .query_map([session.id], |row| {
                        Ok(NoteRecord { body: row.get(0)?, collapsed: row.get(1)? })
                    })?
                    .filter(|note| !matches!(note, Ok(n) if is_blank(&n.body)))
                    .collect::<rusqlite::Result<_>>()?;
                session.tasks = tasks
                    .query_map([session.id], |row| Ok(TaskRecord { text: row.get(0)?, done: row.get(1)? }))?
                    .filter(|task| !matches!(task, Ok(t) if is_blank(&t.text)))
                    .collect::<rusqlite::Result<_>>()?;
            }
            window.splits = splits
                .query_map([window.id], |row| Ok(SplitRecord { id: row.get(0)?, layout: row.get(1)? }))?
                .collect::<rusqlite::Result<_>>()?;
        }
        Ok(windows)
    }

    /// Replace one window's sessions and splits (and the window's own row); other windows'
    /// rows are left alone. Scrollback is not touched (see `save_scrollbacks`).
    /// Rows that are already as given are not written, so saving an unchanged
    /// window costs reads only. To save just what changed, see `save_session`.
    pub fn save_window(&mut self, window: &WindowRecord) -> Result<()> {
        let tx = self.conn.transaction()?;
        write_window_row(&tx, window)?;
        let keep: HashSet<i64> = window.sessions.iter().map(|s| s.id).collect();
        let forgotten = delete_sessions_except(&tx, window.id, &keep)?;
        for split in &window.splits {
            write_split(&tx, window.id, split)?;
        }
        for (position, session) in window.sessions.iter().enumerate() {
            write_session(&tx, window.id, Some(position), session)?;
        }
        forget_empty_splits(&tx, window.id)?;
        tx.commit()?;
        if forgotten > 0 {
            self.checkpoint_after_delete();
        }
        Ok(())
    }

    /// Only the window's own row (selected session, layout, bounds); `window.sessions` is ignored.
    pub fn save_window_meta(&self, window: &WindowRecord) -> Result<()> {
        write_window_row(&self.conn, window)
    }

    /// Forget a window and everything in it (the user closed it).
    pub fn delete_window(&mut self, id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        delete_sessions_except(&tx, id, &HashSet::new())?;
        exec(&tx, "DELETE FROM splits WHERE window_id = ?1", [id])?;
        exec(&tx, "DELETE FROM windows WHERE id = ?1", [id])?;
        tx.commit()?;
        self.checkpoint_after_delete();
        Ok(())
    }

    /// Forget this window's sessions that are not in `keep` (closed sessions), with their notes,
    /// tasks, terminal and scrollback, and the splits left without a member. Other windows'
    /// sessions are never touched.
    pub fn delete_sessions_except(&mut self, window_id: i64, keep: &[i64]) -> Result<()> {
        let tx = self.conn.transaction()?;
        let forgotten = delete_sessions_except(&tx, window_id, &keep.iter().copied().collect())?;
        forget_empty_splits(&tx, window_id)?;
        tx.commit()?;
        if forgotten > 0 {
            self.checkpoint_after_delete();
        }
        Ok(())
    }

    /// Ids for new sessions, splits and windows must not collide with saved ones: one counter for
    /// sessions, terminals and splits, one for windows. A tab's id (schema 6) outlives the pane it
    /// was made with, so those group keys count too.
    pub fn next_ids(&self) -> Result<(i64, i64)> {
        let max = |sql: &str| -> Result<i64> {
            Ok(self.conn.query_row(sql, [], |row| row.get::<_, Option<i64>>(0))?.unwrap_or(0))
        };
        let terminal =
            max("SELECT MAX(m) FROM (SELECT MAX(id) AS m FROM sessions UNION ALL SELECT MAX(id) FROM tabs
                                 UNION ALL SELECT MAX(tab_id) FROM tabs
                                 UNION ALL SELECT MAX(id) FROM splits)")?;
        let window = max("SELECT MAX(id) FROM windows")?;
        Ok((terminal + 1, window + 1))
    }
}

/// A new window goes last; an existing one keeps its place. An unchanged row is not rewritten.
fn write_window_row(conn: &Connection, window: &WindowRecord) -> Result<()> {
    exec(
        conn,
        "INSERT INTO windows (id, position, active, layout, bounds)
         VALUES (?1, (SELECT COALESCE(MAX(position) + 1, 0) FROM windows), ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET active = ?2, layout = ?3, bounds = ?4
         WHERE active IS NOT ?2 OR layout IS NOT ?3 OR bounds IS NOT ?4",
        params![window.id, window.active as i64, window.layout, window.bounds],
    )?;
    Ok(())
}

/// A split is not rewritten when it is already as given.
fn write_split(conn: &Connection, window_id: i64, split: &SplitRecord) -> Result<()> {
    exec(
        conn,
        "INSERT INTO splits (id, window_id, layout) VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET window_id = ?2, layout = ?3
         WHERE window_id IS NOT ?2 OR layout IS NOT ?3",
        params![split.id, window_id, split.layout],
    )?;
    Ok(())
}

/// A split no session is a member of any more (its terminals were closed, or ungrouped) is nothing.
fn forget_empty_splits(conn: &Connection, window_id: i64) -> Result<()> {
    exec(
        conn,
        "DELETE FROM splits WHERE window_id = ?1
            AND id NOT IN (SELECT split_id FROM sessions WHERE split_id IS NOT NULL)",
        [window_id],
    )?;
    Ok(())
}

/// How many sessions were forgotten.
fn delete_sessions_except(conn: &Connection, window_id: i64, keep: &HashSet<i64>) -> Result<usize> {
    let own: Vec<i64> = conn
        .prepare_cached("SELECT id FROM sessions WHERE window_id = ?")?
        .query_map([window_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut forgotten = 0;
    for id in own.into_iter().filter(|id| !keep.contains(id)) {
        delete_session_rows(conn, id)?;
        forgotten += 1;
    }
    Ok(forgotten)
}
