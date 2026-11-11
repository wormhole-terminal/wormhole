//! The Notes tab's persistence: the app's own tables, written without the rest of a terminal's record.

use wormhole_core::storage::store::{SessionRecord, Store};

use crate::files_panel::{NoteSaved, NotesSaved, SessionNotesStore, TaskSaved};

fn saved(notes: &[(&str, bool)], tasks: &[(&str, bool)], folds: (bool, bool)) -> NotesSaved {
    NotesSaved {
        notes: notes
            .iter()
            .map(|(body, collapsed)| NoteSaved { body: body.to_string(), collapsed: *collapsed })
            .collect(),
        tasks: tasks.iter().map(|(text, done)| TaskSaved { text: text.to_string(), done: *done }).collect(),
        tasks_collapsed: folds.0,
        notes_collapsed: folds.1,
    }
}

#[test]
fn notes_and_tasks_come_back_as_saved_and_blank_ones_are_never_kept() {
    let dir = tempfile::tempdir().expect("data dir");
    let store = SessionNotesStore::in_dir(dir.path()).expect("opens");
    assert_eq!(store.load(7), NotesSaved::default(), "nothing saved: empty");
    let notes = saved(
        &[("first\nline two", true), ("  ", false), ("second", false)],
        &[("ship", true), ("", false)],
        (false, false),
    );
    assert!(store.save(7, notes));
    let back = store.load(7);
    assert_eq!(
        back.notes,
        [
            NoteSaved { body: "first\nline two".into(), collapsed: true },
            NoteSaved { body: "second".into(), collapsed: false }
        ]
    );
    assert_eq!(back.tasks, [TaskSaved { text: "ship".into(), done: true }]);
    assert_eq!(store.load(8), NotesSaved::default(), "another terminal's are its own");
    drop(store);
    let reopened = SessionNotesStore::in_dir(dir.path()).expect("opens again");
    assert_eq!(reopened.load(7), back, "kept on disk");
}

#[test]
fn the_folds_belong_to_the_terminals_own_row_and_nothing_else_of_it_changes() {
    let dir = tempfile::tempdir().expect("data dir");
    let mut core = Store::open_at(dir.path()).expect("opens");
    let mut session = SessionRecord::new(3, Some("build".into()), Some("/work".into()));
    session.pinned = true;
    core.save_session(1, &session).expect("the window saved the terminal");
    drop(core);
    let store = SessionNotesStore::in_dir(dir.path()).expect("opens");
    assert!(store.save(3, saved(&[("n", false)], &[("t", false)], (true, true))));
    let back = store.load(3);
    assert!(back.tasks_collapsed && back.notes_collapsed);
    drop(store);
    let loaded = Store::open_at(dir.path()).expect("opens").load().expect("loads");
    let record = &loaded[0].sessions[0];
    assert_eq!(
        (record.name.as_deref(), record.cwd.as_deref(), record.pinned),
        (Some("build"), Some("/work"), true)
    );
    assert_eq!((record.notes.len(), record.tasks.len(), record.notes_collapsed), (1, 1, true));
}
