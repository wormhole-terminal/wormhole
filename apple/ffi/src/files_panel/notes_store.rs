//! A terminal's notes and tasks on disk: the Notes tab's persistence, in the same database and tables the
//! app uses, written without the rest of the terminal's record (core's `Store::save_session_notes`). A blank
//! note or task is one still being written and is never saved.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use wormhole_core::storage::store::{NoteRecord, SessionNotes, Store, TaskRecord};

use crate::error::FfiError;
use crate::isolation;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct NoteSaved {
    pub body: String,
    pub collapsed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct TaskSaved {
    pub text: String,
    pub done: bool,
}

/// What the Notes tab holds for one terminal.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct NotesSaved {
    pub notes: Vec<NoteSaved>,
    pub tasks: Vec<TaskSaved>,
    pub tasks_collapsed: bool,
    pub notes_collapsed: bool,
}

impl From<SessionNotes> for NotesSaved {
    fn from(saved: SessionNotes) -> Self {
        Self {
            notes: saved
                .notes
                .into_iter()
                .map(|n| NoteSaved { body: n.body, collapsed: n.collapsed })
                .collect(),
            tasks: saved.tasks.into_iter().map(|t| TaskSaved { text: t.text, done: t.done }).collect(),
            tasks_collapsed: saved.tasks_collapsed,
            notes_collapsed: saved.notes_collapsed,
        }
    }
}

impl From<NotesSaved> for SessionNotes {
    fn from(saved: NotesSaved) -> Self {
        Self {
            notes: saved
                .notes
                .into_iter()
                .map(|n| NoteRecord { body: n.body, collapsed: n.collapsed })
                .collect(),
            tasks: saved.tasks.into_iter().map(|t| TaskRecord { text: t.text, done: t.done }).collect(),
            tasks_collapsed: saved.tasks_collapsed,
            notes_collapsed: saved.notes_collapsed,
        }
    }
}

/// The notes and tasks of every terminal, by its id.
#[derive(uniffi::Object)]
pub struct SessionNotesStore {
    store: Mutex<Store>,
}

impl SessionNotesStore {
    pub(crate) fn in_dir(dir: &Path) -> Result<Arc<Self>, FfiError> {
        let store = Store::open_at(dir).map_err(|err| FfiError::Start { reason: format!("{err:#}") })?;
        Ok(Arc::new(Self { store: Mutex::new(store) }))
    }
}

#[uniffi::export]
impl SessionNotesStore {
    /// Fails unless this is an isolated instance of the experiment, or the database can't be opened.
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, FfiError> {
        isolation::require()?;
        Self::in_dir(&wormhole_core::storage::paths::data_dir())
    }

    /// What was saved for terminal `session` (nothing: empty).
    pub fn load(&self, session: i64) -> NotesSaved {
        let store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        store
            .load_session_notes(session)
            .inspect_err(|err| tracing::warn!("reading the notes of terminal {session}: {err:#}"))
            .unwrap_or_default()
            .into()
    }

    /// Replaces what is saved for terminal `session`. Rows already equal to what is given are not
    /// rewritten, so saving after every keystroke costs one comparison. False when the write failed.
    pub fn save(&self, session: i64, saved: NotesSaved) -> bool {
        let mut store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        match store.save_session_notes(session, &saved.into()) {
            Ok(()) => true,
            Err(err) => {
                tracing::warn!("saving the notes of terminal {session}: {err:#}");
                false
            }
        }
    }
}
