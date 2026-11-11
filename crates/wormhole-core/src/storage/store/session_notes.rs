//! One session's notes and tasks on their own: read and written without the rest of the session's
//! record, for a host that keeps notes apart from the window it saves (`save_session` writes a whole
//! session, its folder and shell included, which a caller that only edited a note does not have).

use anyhow::Result;

use super::records::{NoteRecord, TaskRecord, is_blank};
use super::sessions::{replace_notes, replace_tasks};
use super::{Store, exec};

/// What the Notes tab holds for one terminal.
#[derive(Debug, Default, Clone)]
pub struct SessionNotes {
    pub notes: Vec<NoteRecord>,
    pub tasks: Vec<TaskRecord>,
    pub tasks_collapsed: bool,
    pub notes_collapsed: bool,
}

impl Store {
    /// The notes and tasks saved for session `id` (empty when there are none), blank entries left out as
    /// [`Store::load`] leaves them.
    pub fn load_session_notes(&self, id: i64) -> Result<SessionNotes> {
        let conn = &self.conn;
        let notes = conn
            .prepare_cached("SELECT body, collapsed FROM notes WHERE session_id = ? ORDER BY position")?
            .query_map([id], |row| Ok(NoteRecord { body: row.get(0)?, collapsed: row.get(1)? }))?
            .filter(|note| !matches!(note, Ok(n) if is_blank(&n.body)))
            .collect::<rusqlite::Result<_>>()?;
        let tasks = conn
            .prepare_cached("SELECT text, done FROM tasks WHERE session_id = ? ORDER BY position")?
            .query_map([id], |row| Ok(TaskRecord { text: row.get(0)?, done: row.get(1)? }))?
            .filter(|task| !matches!(task, Ok(t) if is_blank(&t.text)))
            .collect::<rusqlite::Result<_>>()?;
        let folds: Option<(bool, bool)> = conn
            .prepare_cached("SELECT tasks_collapsed, notes_collapsed FROM sessions WHERE id = ?")?
            .query_map([id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .next()
            .transpose()?;
        let (tasks_collapsed, notes_collapsed) = folds.unwrap_or_default();
        Ok(SessionNotes { notes, tasks, tasks_collapsed, notes_collapsed })
    }

    /// Replaces session `id`'s notes and tasks with `saved` (blank entries are never kept), in one
    /// transaction. The folds belong to the session's own row: they are written only when the window has
    /// saved that row, and nothing else of it changes.
    pub fn save_session_notes(&mut self, id: i64, saved: &SessionNotes) -> Result<()> {
        let tx = self.conn.transaction()?;
        replace_notes(&tx, id, saved.notes.iter().filter(|n| !is_blank(&n.body)).collect())?;
        replace_tasks(&tx, id, saved.tasks.iter().filter(|t| !is_blank(&t.text)).collect())?;
        exec(
            &tx,
            "UPDATE sessions SET tasks_collapsed = ?2, notes_collapsed = ?3
             WHERE id = ?1 AND (tasks_collapsed IS NOT ?2 OR notes_collapsed IS NOT ?3)",
            rusqlite::params![id, saved.tasks_collapsed, saved.notes_collapsed],
        )?;
        tx.commit()?;
        Ok(())
    }
}
