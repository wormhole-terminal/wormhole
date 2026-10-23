//! Two-way sync between one session's notes and tasks and a project's `notes.md`: a three-way merge
//! against the last state both agreed on (the base).
//!
//! The base is tied to the project it was agreed with and is kept across restarts. Without that,
//! the first sync after a launch merged against nothing and brought back everything deleted from
//! `notes.md` while the app was closed, and a session whose first tab moved to another project
//! merged that project's file against the old project's base, which wiped the session's notes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{ProjectError, ProjectFolder, parse_json, read_state_file, set_aside, write_atomic};
use super::{NotesDoc, merge_notes};
use crate::storage::files::{FileStamp, LinkPolicy};

/// The last notes and tasks a session and a `notes.md` agreed on, and the project root of that file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesBase {
    pub root: PathBuf,
    #[serde(flatten)]
    pub doc: NotesDoc,
}

/// What one sync step decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    /// Write this to `notes.md` (what the file holds differs).
    pub write: Option<NotesDoc>,
    /// Replace the session's notes and tasks with this.
    pub apply: Option<NotesDoc>,
    /// The base for the next step, once the write (if any) went through.
    pub base: NotesBase,
}

/// One sync step for a session whose notes sync with the `notes.md` of `root`: `local` is the session,
/// `theirs` the file (empty when there is none). `None` when there is nothing to do, and while
/// the session is `editing` a note or task: replacing the list under an open editor would move it
/// onto another item, so the merge waits for the edit and then includes it.
pub fn plan_sync(
    base: Option<&NotesBase>,
    root: &Path,
    local: &NotesDoc,
    theirs: &NotesDoc,
    editing: bool,
) -> Option<SyncPlan> {
    if editing {
        return None;
    }
    // A base agreed with another project's file says nothing about this one: the first sync
    // with a file keeps everything from both sides.
    let empty = NotesDoc::default();
    let base_doc = base.filter(|base| base.root == root).map_or(&empty, |base| &base.doc);
    if local == base_doc && theirs == base_doc {
        return None;
    }
    let merged = merge_notes(base_doc, local, theirs);
    Some(SyncPlan {
        write: (merged != *theirs).then(|| merged.clone()),
        apply: (merged != *local).then(|| merged.clone()),
        base: NotesBase { root: root.to_path_buf(), doc: merged },
    })
}

/// Every session's merge base, stored as `notes-sync.json` in the app's data dir so a restart keeps
/// them. Keyed by session id. Mutations are in memory; [`Self::save`] persists.
///
/// Session ids are not unique over time: a launch hands out ids from one past the highest saved one,
/// so the id of a session that was closed last (its base kept because the app was quitting), of one
/// that never reached the database, or of every session in a database that was moved aside comes
/// back for a new session. That session must not merge against the old base: an empty session against a
/// base full of notes reads as "deleted them all", and the merge removes them from `notes.md`.
/// So every id from the first unused one on is forgotten at launch ([`Self::forget_from`]), and
/// an id handed out later is forgotten again when it is ([`Self::remove`]).
#[derive(Debug, Default)]
pub struct NotesBaseStore {
    bases: BTreeMap<i64, NotesBase>,
    path: PathBuf,
    dirty: bool,
}

impl NotesBaseStore {
    pub const FILE_NAME: &'static str = "notes-sync.json";

    pub fn load(data_dir: &Path) -> Result<Self, ProjectError> {
        let path = data_dir.join(Self::FILE_NAME);
        let bases = match read_state_file(&path)? {
            Some(text) => parse_json(&path, &text)?,
            None => BTreeMap::new(),
        };
        Ok(Self { bases, path, dirty: false })
    }

    /// [`Self::load`]; a file that doesn't load is moved aside (`notes-sync.json.bad`) and an
    /// empty store returned with the error, so the next save can't destroy it unseen.
    pub fn load_or_recover(data_dir: &Path) -> (Self, Option<ProjectError>) {
        match Self::load(data_dir) {
            Ok(store) => (store, None),
            Err(err) => {
                let path = data_dir.join(Self::FILE_NAME);
                set_aside(&path);
                (Self { path, ..Self::default() }, Some(err))
            }
        }
    }

    pub fn get(&self, session: i64) -> Option<&NotesBase> {
        self.bases.get(&session)
    }

    pub fn set(&mut self, session: i64, base: NotesBase) {
        if self.bases.get(&session) != Some(&base) {
            self.bases.insert(session, base);
            self.dirty = true;
        }
    }

    pub fn remove(&mut self, session: i64) {
        self.dirty |= self.bases.remove(&session).is_some();
    }

    /// Forget the bases of every id from `first_free` on: the first id the app will hand out to a
    /// new session or tab, one past the highest the database holds. No saved session has such an id, so
    /// these bases are left from sessions that are gone, and a new session would inherit one.
    pub fn forget_from(&mut self, first_free: i64) {
        self.retain(|session| session < first_free);
    }

    /// Forget the sessions `keep` says no to (closed ones).
    pub fn retain(&mut self, keep: impl Fn(i64) -> bool) {
        let before = self.bases.len();
        self.bases.retain(|session, _| keep(*session));
        self.dirty |= self.bases.len() != before;
    }

    /// Changed since the last load or save.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn save(&mut self) -> Result<(), ProjectError> {
        let json = serde_json::to_string_pretty(&self.bases)
            .map_err(|e| ProjectError::invalid(&self.path, e.to_string()))?;
        // The user's own data dir: a symlinked file there is their dotfiles setup.
        write_atomic(&self.path, json.as_bytes(), LinkPolicy::Follow)?;
        self.dirty = false;
        Ok(())
    }
}

/// A project's `notes.md`, read and parsed again only when it changed: the sync looks at it
/// every 1.2 s for every session.
#[derive(Debug)]
pub struct NotesFile {
    folder: ProjectFolder,
    /// The stamp taken before the last read, and what that read gave.
    last: Option<(Option<FileStamp>, NotesRead)>,
}

/// What reading `notes.md` gave: its notes (`None` without a file) or why it can't be read.
pub(super) type NotesRead = Result<Option<NotesDoc>, ProjectError>;

impl NotesFile {
    pub fn new(folder: ProjectFolder) -> Self {
        Self { folder, last: None }
    }

    pub fn folder(&self) -> &ProjectFolder {
        &self.folder
    }

    /// The file's notes (`None` without a notes.md) or why it can't be read, and whether it was
    /// read just now (so a broken file is reported once, not on every look).
    pub fn read(&mut self) -> (&NotesRead, bool) {
        let stamp = FileStamp::of(&self.folder.notes_path());
        let fresh = self.last.as_ref().is_none_or(|(last, _)| *last != stamp);
        if fresh {
            self.last = None;
        }
        let folder = &self.folder;
        let (_, result) = self.last.get_or_insert_with(|| (stamp, folder.notes()));
        (result, fresh)
    }

    /// Write `doc` (see [`ProjectFolder::write_notes`]); the next read looks at the file again.
    pub fn write(&mut self, doc: &NotesDoc) -> Result<bool, ProjectError> {
        self.last = None;
        self.folder.write_notes(doc)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::tests::doc;
    use super::super::{NotesDoc, TaskItem};
    use super::*;

    fn sync_loop(
        base: &mut Option<NotesBase>,
        root: &Path,
        local: &mut NotesDoc,
        file: &mut NotesDoc,
        editing: bool,
    ) {
        if let Some(plan) = plan_sync(base.as_ref(), root, local, file, editing) {
            if let Some(write) = plan.write {
                *file = write;
            }
            if let Some(apply) = plan.apply {
                *local = apply;
            }
            *base = Some(plan.base);
        }
    }

    /// Deleted from notes.md (say by `git pull`) while the app was closed: after the
    /// restart the deletion must stick, and an edited note must not come back twice.
    #[test]
    fn deletions_while_closed_stay_deleted_after_a_restart() {
        let data = tempfile::TempDir::new().expect("tempdir");
        let root = Path::new("/repo");
        let mut local = doc(&[("keep", false), ("drop", false)], &["note one", "an old note about it"]);
        let mut file = local.clone();
        let mut base = None;
        sync_loop(&mut base, root, &mut local, &mut file, false);
        let mut store = NotesBaseStore::load(data.path()).expect("load");
        store.set(7, base.clone().expect("base"));
        store.save().expect("save");

        // Closed; the file loses a task and a note, and another note is edited.
        file = doc(&[("keep", false)], &["note one", "an old note about it, now longer"]);
        // Launch: the session comes back from the database as it was.
        let mut base = NotesBaseStore::load(data.path()).expect("load").get(7).cloned();
        sync_loop(&mut base, root, &mut local, &mut file, false);
        let expected = doc(&[("keep", false)], &["note one", "an old note about it, now longer"]);
        assert_eq!(local, expected);
        assert_eq!(file, expected);
    }

    /// The first tab moves from project A to B. A's base must not be B's: merge(A, A, B)
    /// is just B and wiped the session's notes.
    #[test]
    fn a_base_from_another_project_is_no_base() {
        let (a, b) = (Path::new("/a"), Path::new("/b"));
        let mut local = doc(&[("mine", false)], &["session note"]);
        let mut file_a = local.clone();
        let mut base = None;
        sync_loop(&mut base, a, &mut local, &mut file_a, false);
        assert_eq!(base.as_ref().map(|b| b.root.as_path()), Some(a));

        let mut file_b = doc(&[("theirs", false)], &["project b note"]);
        sync_loop(&mut base, b, &mut local, &mut file_b, false);
        // Nothing wiped: the session's note stays, the file's comes first (it has no predecessor).
        assert_eq!(local.notes, ["project b note", "session note"]);
        assert_eq!(local.tasks.len(), 2);
        assert_eq!(file_b, local);
        assert_eq!(base.as_ref().map(|b| b.root.as_path()), Some(b));
    }

    #[test]
    fn nothing_to_do_when_both_sides_match_the_base() {
        let root = Path::new("/repo");
        let same = doc(&[("t", false)], &["n"]);
        let base = NotesBase { root: root.to_path_buf(), doc: same.clone() };
        assert_eq!(plan_sync(Some(&base), root, &same, &same, false), None);
        // The first sync with an empty session and no file writes nothing either.
        assert_eq!(plan_sync(None, root, &NotesDoc::default(), &NotesDoc::default(), false), None);
        // A file that says the same as the session only needs a base.
        let plan = plan_sync(None, root, &same, &same, false).expect("plan");
        assert_eq!((plan.write, plan.apply), (None, None));
        assert_eq!(plan.base.doc, same);
    }

    /// While a task is being edited, applying the file moved the editor onto another task
    /// (rows are by index), and Enter then overwrote that task. The sync waits for the edit.
    #[test]
    fn a_session_being_edited_syncs_after_the_edit() {
        let root = Path::new("/repo");
        let mut local = doc(&[("first", false), ("second", false)], &[]);
        let mut file = local.clone();
        let mut base = None;
        sync_loop(&mut base, root, &mut local, &mut file, false);

        // An agent adds a task at the top of notes.md while "second" is open in the editor.
        file.tasks.insert(0, TaskItem { text: "from the agent".into(), done: false });
        assert_eq!(plan_sync(base.as_ref(), root, &local, &file, true), None, "deferred");
        // The edit finishes on the task it started on.
        local.tasks[1].text = "second, edited".into();
        sync_loop(&mut base, root, &mut local, &mut file, false);
        let texts: Vec<_> = local.tasks.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["from the agent", "first", "second, edited"]);
        assert_eq!(file, local);
    }

    #[test]
    fn the_store_round_trips_and_forgets() {
        let data = tempfile::TempDir::new().expect("tempdir");
        let mut store = NotesBaseStore::load(data.path()).expect("load");
        assert!(!store.is_dirty());
        let base = NotesBase { root: "/repo".into(), doc: doc(&[("t\n---", true)], &["## Todo\nx"]) };
        store.set(1, base.clone());
        store.set(2, base.clone());
        assert!(store.is_dirty());
        store.save().expect("save");
        assert!(!store.is_dirty());
        store.set(2, base.clone());
        assert!(!store.is_dirty(), "unchanged");
        let mut store = NotesBaseStore::load(data.path()).expect("load");
        assert_eq!(store.get(1), Some(&base));
        store.retain(|session| session == 2);
        assert_eq!(store.get(1), None);
        assert!(store.is_dirty());
        store.remove(2);
        store.save().expect("save");
        assert_eq!(NotesBaseStore::load(data.path()).expect("load").get(2), None);
    }

    /// The bug: session 7 synced its notes and was closed while the app quit (no pruning then). The
    /// next launch handed id 7 to a new, empty session, which merged against session 7's base: "every
    /// note deleted", and they were removed from notes.md.
    #[test]
    fn a_new_session_with_a_reused_id_deletes_nothing_from_notes_md() {
        let data = tempfile::TempDir::new().expect("tempdir");
        let root = Path::new("/repo");
        let mut old_session = doc(&[("ship it", false)], &["design notes", "meeting notes"]);
        let mut file = NotesDoc::default();
        let mut base = None;
        sync_loop(&mut base, root, &mut old_session, &mut file, false);
        assert_eq!(file, old_session);
        let mut store = NotesBaseStore::load(data.path()).expect("load");
        store.set(6, base.clone().expect("base"));
        store.set(7, base.expect("base"));
        store.save().expect("save");

        // What the stale base would do to the new session 7.
        let stale = NotesBaseStore::load(data.path()).expect("load").get(7).cloned();
        let plan = plan_sync(stale.as_ref(), root, &NotesDoc::default(), &file, false).expect("plan");
        assert_eq!(plan.write, Some(NotesDoc::default()), "the mechanism: everything deleted");

        // Launch: the database's highest id is 6, so 7 is the first one handed out.
        let mut store = NotesBaseStore::load(data.path()).expect("load");
        store.forget_from(7);
        assert!(store.get(6).is_some(), "a saved session keeps its base");
        let mut new_session = NotesDoc::default();
        let mut base = store.get(7).cloned();
        assert_eq!(base, None);
        sync_loop(&mut base, root, &mut new_session, &mut file, false);
        assert_eq!(file, old_session, "nothing deleted from notes.md");
        assert_eq!(new_session, old_session, "the new session picks the file's notes up");

        // A database moved aside starts again at 1: no base survives.
        store.forget_from(1);
        assert_eq!((store.get(6), store.get(7)), (None, None));
        assert!(store.is_dirty());
    }

    /// A broken file is set aside, not silently replaced: it holds the user's merge bases.
    #[test]
    fn a_corrupt_store_is_kept_aside() {
        let data = tempfile::TempDir::new().expect("tempdir");
        std::fs::write(data.path().join(NotesBaseStore::FILE_NAME), "{ nope").expect("write");
        let (store, err) = NotesBaseStore::load_or_recover(data.path());
        assert!(err.is_some());
        assert_eq!(store.get(1), None);
        let aside = data.path().join(format!("{}.bad", NotesBaseStore::FILE_NAME));
        assert_eq!(std::fs::read_to_string(aside).expect("kept"), "{ nope");
    }

    /// The mechanism of the bug: with no base (every launch) the merge treats what was deleted
    /// from the file as added in the app, and keeps the old version of an edited note as well.
    #[test]
    fn merging_against_an_empty_base_brings_deletions_back() {
        let local = doc(&[("keep", false), ("drop", false)], &["note one", "an old note about it"]);
        let file = doc(&[("keep", false)], &["note one", "an old note about it, now longer"]);
        let merged = merge_notes(&NotesDoc::default(), &local, &file);
        assert!(merged.tasks.iter().any(|t| t.text == "drop"));
        assert!(merged.notes.contains(&"an old note about it".to_string()));
    }

    #[test]
    fn the_file_is_read_again_only_after_a_change() {
        use super::super::super::tests::{project, reads_during};
        let (_tmp, p) = project(&[]);
        let mut file = NotesFile::new(p.clone());
        assert!(matches!(file.read(), (Ok(None), true)));
        assert!(matches!(file.read(), (Ok(None), false)));
        let ours = doc(&[("t", false)], &["n"]);
        assert!(file.write(&ours).expect("write"));
        let (read, fresh) = file.read();
        assert_eq!((read.as_ref().ok().cloned().flatten(), fresh), (Some(ours.clone()), true));
        assert_eq!(reads_during(|| assert!(!file.read().1)), 0);
        std::fs::write(p.notes_path(), "- [ ] edited elsewhere\n").expect("write");
        let (read, fresh) = file.read();
        assert!(fresh);
        assert_eq!(read.as_ref().ok().cloned().flatten().map(|d| d.tasks.len()), Some(1));
        // A broken file is reported once (when read), not on every look.
        std::fs::write(p.notes_path(), [0xff, 0xfe]).expect("write");
        assert!(matches!(file.read(), (Err(_), true)));
        assert!(matches!(file.read(), (Err(_), false)));
    }
}
