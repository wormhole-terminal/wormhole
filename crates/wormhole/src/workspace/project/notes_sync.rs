//! Two-way sync of each session's notes and tasks with its project's `notes.md` (the merge itself
//! is in core: `wormhole_core::project::plan_sync`).

use wormhole_core::project::{self as proj, Feature, NotesBase, NotesDoc, NotesFile};

use super::*;

impl Workspace {
    /// Two-way sync between each session's notes/tasks and its project's `notes.md`, as a three-way
    /// merge against the last state both agreed on (kept per session and project across restarts).
    pub(in crate::workspace) fn sync_project_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // It runs more often than `refresh_projects`, so the `notes_root`s may not be cleared yet
        // when the project-folder flag goes off: check the flag itself before touching notes.md.
        if !project_folders_on(cx) || !super::super::notes::notes_on(cx) {
            return;
        }
        for ix in 0..self.sessions.len() {
            let session_id = self.sessions[ix].id;
            let Some(root) = self.sessions[ix].notes_root.clone() else { continue };
            let hub = &app(cx).hub;
            let Some(project) = hub.projects.get(&root) else { continue };
            // Syncs when switched on and the project has a notes.md, or when the user
            // explicitly opted in for this project (which creates the file). The Project tab's
            // switch asks the same question, so it shows on only when this runs.
            if !hub.prefs.notes_sync(&root, project.folder.has(Feature::Notes)) {
                // Switched off: switching on again merges both sides afresh.
                self.sessions[ix].notes_base = None;
                app_mut(cx).hub.notes_bases.remove(session_id);
                continue;
            }
            let folder = project.folder.clone();
            let local = self.session_doc(ix, cx);
            let editing = self.editing_notes_of(ix);
            // `notes_base` is this run's copy of the base (empty after a launch); the hub knows
            // which project it belongs to and keeps it for the next launch.
            let session_base = self.sessions[ix].notes_base.clone();
            let hub = &mut app_mut(cx).hub;
            let base = hub.notes_bases.get(session_id).map(|saved| NotesBase {
                root: saved.root.clone(),
                doc: session_base.unwrap_or_else(|| saved.doc.clone()),
            });
            let file = hub.notes_files.entry(root.clone()).or_insert_with(|| NotesFile::new(folder));
            let plan = match file.read() {
                (Ok(theirs), _) => {
                    let empty = NotesDoc::default();
                    proj::plan_sync(base.as_ref(), &root, &local, theirs.as_ref().unwrap_or(&empty), editing)
                }
                (Err(err), fresh) => {
                    if fresh {
                        tracing::warn!("notes.md: {err}");
                    }
                    hub.notes_errors.insert(root, err.to_string());
                    continue;
                }
            };
            let Some(plan) = plan else { continue };
            if let Some(doc) = &plan.write
                && let Err(err) = file.write(doc)
            {
                let err = err.to_string();
                if hub.notes_errors.get(&root) != Some(&err) {
                    tracing::warn!("writing notes.md: {err}");
                }
                hub.notes_errors.insert(root, err);
                continue;
            }
            hub.notes_errors.remove(&root);
            hub.notes_bases.set(session_id, plan.base.clone());
            if let Some(doc) = &plan.apply {
                self.apply_doc(ix, doc, window, cx);
            }
            self.sessions[ix].notes_base = Some(plan.base.doc);
        }
        app_mut(cx).hub.save_notes_bases();
    }

    /// Whether a task or note of session `ix` has an editor open (the sync waits for it).
    fn editing_notes_of(&self, ix: usize) -> bool {
        let session = &self.sessions[ix];
        session.task_edit.is_some()
            || self
                .editing_note
                .is_some_and(|id| session.notes.iter().any(|note| note.state.entity_id() == id))
    }

    fn session_doc(&self, ix: usize, cx: &App) -> NotesDoc {
        let session = &self.sessions[ix];
        NotesDoc {
            tasks: session
                .tasks
                .iter()
                .map(|t| proj::TaskItem { text: t.text.to_string(), done: t.done })
                .collect(),
            notes: session
                .notes
                .iter()
                .map(|n| n.state.read(cx).value().trim().to_string())
                .filter(|text| !text.is_empty())
                .collect(),
        }
    }

    /// Replace a session's tasks and (non-empty) notes with `doc`. A note keeps its editor (folding,
    /// cursor, undo) when its text is still there, wherever it moved.
    fn apply_doc(&mut self, ix: usize, doc: &NotesDoc, window: &mut Window, cx: &mut Context<Self>) {
        let session = &mut self.sessions[ix];
        session.tasks =
            doc.tasks.iter().map(|t| TaskItem { text: t.text.clone().into(), done: t.done }).collect();
        // The sync waits while an editor is open; this is the backstop for one that isn't.
        if session.task_edit.as_ref().is_some_and(|edit| !edit.new && edit.ix >= session.tasks.len()) {
            session.task_edit = None;
        }

        let (filled, blank): (Vec<Note>, Vec<Note>) = std::mem::take(&mut session.notes)
            .into_iter()
            .partition(|note| !note.state.read(cx).value().trim().is_empty());
        let texts: Vec<String> =
            filled.iter().map(|note| note.state.read(cx).value().trim().to_string()).collect();
        let slots = note_slots(&texts, &doc.notes);
        let mut filled: Vec<Option<Note>> = filled.into_iter().map(Some).collect();
        for (text, slot) in doc.notes.iter().zip(slots) {
            match slot.and_then(|i| Some((i, filled[i].take()?))) {
                Some((i, note)) => {
                    if texts[i] != text.trim() {
                        let text = SharedString::from(text.clone());
                        note.state.update(cx, |n, cx| n.set_value(text, window, cx));
                    }
                    self.sessions[ix].notes.push(note);
                }
                None => {
                    self.add_note(ix, Some(text.clone()), false, window, cx);
                }
            }
        }
        // Notes still being written (empty) stay, after the others.
        self.sessions[ix].notes.extend(blank);
        self.focused_note = None;
        self.mark_dirty(cx);
    }
}

/// For each merged note, the existing note (index into `existing`) that shows it from now on: one
/// with exactly that text if there is one, else the next unused one in order (an edited note),
/// else none (a new note). Positions alone would hand a note's editor to its neighbour whenever a
/// note is added or removed above it.
fn note_slots(existing: &[String], merged: &[String]) -> Vec<Option<usize>> {
    let mut used = vec![false; existing.len()];
    let mut slots: Vec<Option<usize>> = merged
        .iter()
        .map(|text| {
            let found = (0..existing.len()).find(|&i| !used[i] && existing[i] == text.trim());
            if let Some(i) = found {
                used[i] = true;
            }
            found
        })
        .collect();
    let mut free = (0..existing.len()).filter(|&i| !used[i]).collect::<Vec<_>>().into_iter();
    for slot in slots.iter_mut().filter(|slot| slot.is_none()) {
        *slot = free.next();
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::note_slots;

    fn texts(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn notes_keep_their_editors_by_text() {
        let existing = texts(&["a", "b", "c"]);
        // An external insert above: every note keeps its own editor.
        assert_eq!(note_slots(&existing, &texts(&["new", "a", "b", "c"])), [None, Some(0), Some(1), Some(2)]);
        // A removal and a reorder.
        assert_eq!(note_slots(&existing, &texts(&["c", "a"])), [Some(2), Some(0)]);
        // An edited note takes the editor that is left over.
        assert_eq!(note_slots(&existing, &texts(&["a", "b, edited", "c"])), [Some(0), Some(1), Some(2)]);
        // Duplicates are matched one to one.
        assert_eq!(note_slots(&texts(&["x", "x"]), &texts(&["x", "x", "x"])), [Some(0), Some(1), None]);
    }
}
