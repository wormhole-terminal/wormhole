//! Two-way sync between one terminal's notes and tasks and its project's `notes.md`: a three-way merge
//! against the last state both agreed on (core's `plan_sync`), kept per terminal across restarts.
//!
//! `notes.md` comes with the repository, which may be anyone's, and it is merged into the terminal's notes
//! and written back, so an untrusted project's file is neither read nor written: the sync waits for trust
//! and says so in the Project tab. It runs only where the person wants it (the switch is on and the file is
//! there, or the person switched it on, which makes the file).

use std::path::Path;

use wormhole_core::project::{self as proj, Feature, NotesBase, NotesDoc, NotesFile, TaskItem, TrustStatus};
use wormhole_core::view::notes::paragraphs::note_slots;

use super::hub::Hub;
use super::service::{ProjectService, open};
use super::stores;

/// Why a project's notes.md does not sync while the person wants it to and the project is not trusted
/// (shown in the Project tab after "Notes sync: ").
const NEEDS_TRUST: &str =
    "waits for the project to be trusted; its notes.md is neither read nor written until then";

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct TaskDocRecord {
    pub text: String,
    pub done: bool,
}

/// A terminal's tasks and its notes with something in them: the document the sync compares.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct NotesDocRecord {
    pub tasks: Vec<TaskDocRecord>,
    pub notes: Vec<String>,
}

impl From<NotesDocRecord> for NotesDoc {
    fn from(doc: NotesDocRecord) -> Self {
        Self {
            tasks: doc.tasks.into_iter().map(|t| TaskItem { text: t.text, done: t.done }).collect(),
            notes: doc.notes.into_iter().map(|note| note.trim().to_string()).collect(),
        }
    }
}

impl From<&NotesDoc> for NotesDocRecord {
    fn from(doc: &NotesDoc) -> Self {
        Self {
            tasks: doc.tasks.iter().map(|t| TaskDocRecord { text: t.text.clone(), done: t.done }).collect(),
            notes: doc.notes.clone(),
        }
    }
}

/// What one sync step decided for the terminal.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct NotesSyncOutcome {
    /// Replace the terminal's tasks and (non-empty) notes with this; `None` when they stand.
    pub apply: Option<NotesDocRecord>,
    /// For each note of `apply`, the note of the document passed in that keeps its editor (its folds,
    /// its cursor): the one with exactly that text, else the next unused one, else none (a new note).
    pub note_slots: Vec<Option<u32>>,
}

#[uniffi::export]
impl ProjectService {
    /// One sync step for the terminal `session`, whose notes are `local`, with the `notes.md` of the project
    /// at `root`. `editing`: a note or task has an open editor, so the merge waits for the edit. Cheap
    /// when nothing changed (a file stamp), so it can run every second or so while the Notes tab is up.
    pub fn sync_notes(
        &self,
        session: i64,
        root: String,
        local: NotesDocRecord,
        editing: bool,
    ) -> NotesSyncOutcome {
        // With project folders off no notes.md is read or written.
        if self.off() {
            return NotesSyncOutcome::default();
        }
        let Some(folder) = open(&root) else { return NotesSyncOutcome::default() };
        let local = NotesDoc::from(local);
        let mut hub = self.lock();
        let snapshot = hub.snapshot(&folder);
        // The profile's switch and approvals, held from here to the write: once another window has switched the sync off
        // or revoked the project, the file is neither read nor written here. And the file's own lock, held from its read
        // to its write: two windows merging terminals of this project take turns, each from what the other wrote.
        let profile = hub.stores();
        let prefs = stores::now(&profile.prefs);
        let trust = stores::now(&profile.trust);
        let wanted = prefs.notes_sync(&folder.root, folder.has(Feature::Notes));
        let trusted = trust.status_of(&snapshot) == TrustStatus::Trusted;
        let (outcome, told) = if wanted && trusted {
            let file = stores::notes_file(&folder);
            let mut file = stores::lock(&file);
            step(&mut hub, &mut file, session, &folder.root, &local, editing)
        } else {
            // Switched off: switching on merges both sides afresh. Waiting for trust keeps the base, so
            // what was deleted on either side before the wait stays deleted once the project is trusted.
            if !wanted {
                hub.bases().remove(session);
            }
            let told = hub.set_notes_error(&folder.root, (wanted).then(|| NEEDS_TRUST.to_string()));
            (NotesSyncOutcome::default(), told)
        };
        drop((trust, prefs));
        hub.save_notes_bases();
        drop(hub);
        if told {
            self.changed();
        }
        outcome
    }

    /// The terminal `session` is closed: the base it merged against goes with it, so a new terminal that
    /// is given its id does not read as having deleted every note.
    pub fn notes_session_closed(&self, session: i64) {
        let mut hub = self.lock();
        hub.bases().remove(session);
        hub.save_notes_bases();
    }

    /// Ids from `first_free` on are handed out to new terminals from now on (call it at launch with one past
    /// the highest id in use): bases left from terminals that are gone are forgotten. The bases are the profile's,
    /// every window's: a window's own highest id is not past another window's, so the app's counter decides once
    /// there is one (the GPUI app's `ids_start_at(app.next_id())`).
    pub fn notes_ids_start_at(&self, first_free: i64) {
        let first_free = crate::windows::next_terminal_id().map_or(first_free, |next| next.max(first_free));
        let mut hub = self.lock();
        hub.bases().forget_from(first_free);
        hub.save_notes_bases();
    }
}

/// The merge itself, for a project that is wanted and trusted. Returns what to apply and whether what the
/// Project tab says of the sync changed.
fn step(
    hub: &mut Hub,
    file: &mut NotesFile,
    session: i64,
    root: &Path,
    local: &NotesDoc,
    editing: bool,
) -> (NotesSyncOutcome, bool) {
    let base = hub.bases().get(session).cloned();
    let plan = match file.read() {
        (Ok(theirs), _) => {
            let empty = NotesDoc::default();
            proj::plan_sync(base.as_ref(), root, local, theirs.as_ref().unwrap_or(&empty), editing)
        }
        (Err(err), fresh) => {
            if fresh {
                tracing::warn!("notes.md: {err}");
            }
            let message = err.to_string();
            return (NotesSyncOutcome::default(), hub.set_notes_error(root, Some(message)));
        }
    };
    let Some(plan) = plan else { return (NotesSyncOutcome::default(), false) };
    if let Some(doc) = &plan.write
        && let Err(err) = file.write(doc)
    {
        let message = err.to_string();
        let changed = hub.set_notes_error(root, Some(message.clone()));
        if changed {
            tracing::warn!("writing notes.md: {message}");
        }
        return (NotesSyncOutcome::default(), changed);
    }
    let told = hub.set_notes_error(root, None);
    hub.bases().set(session, NotesBase { root: root.to_path_buf(), doc: plan.base.doc.clone() });
    let outcome = match &plan.apply {
        Some(doc) => NotesSyncOutcome {
            apply: Some(NotesDocRecord::from(doc)),
            note_slots: note_slots(&local.notes, &doc.notes)
                .into_iter()
                .map(|slot| slot.map(|i| i as u32))
                .collect(),
        },
        None => NotesSyncOutcome::default(),
    };
    (outcome, told)
}
