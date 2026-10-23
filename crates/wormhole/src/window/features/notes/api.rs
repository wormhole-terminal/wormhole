//! What the project folder may do with a terminal's notes and tasks: read them as a document, replace
//! them with a merged one, ask whether one is being written (the `notes.md` sync waits for the writer),
//! and add a note from a prompt. The project feature's `notes_sync` is the one file that uses it; the
//! sync's own decisions are core's (`project::plan_sync`).
use gpui_kit::*;
use wormhole_core::project::{NotesDoc, TaskItem as DocTask};
use wormhole_core::view::notes::paragraphs::note_slots;
use wormhole_core::view::notes::tasks::Task;

use super::Notes;

impl Notes {
    /// A note with `text` on the terminal, selected and given the keyboard, as the key for a new note
    /// does. `panel` is the panel's width, for the hint of an empty note.
    pub(in crate::window) fn note_from(
        &mut self,
        session: i64,
        text: String,
        panel: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_note(session, Some(text), panel, window, cx);
    }

    /// The terminal's tasks and its notes with something in them, as the document the sync compares.
    pub(in crate::window) fn snapshot(&self, session: i64, cx: &App) -> NotesDoc {
        let Some(of) = self.of.get(&session) else { return NotesDoc::default() };
        NotesDoc {
            tasks: of
                .tasks
                .items()
                .iter()
                .map(|t| DocTask { text: t.text.to_string(), done: t.done })
                .collect(),
            notes: of
                .notes
                .iter()
                .map(|n| n.state.read(cx).value().trim().to_string())
                .filter(|text| !text.is_empty())
                .collect(),
        }
    }

    /// Whether a task or a note of the terminal has an editor open.
    pub(in crate::window) fn being_written(&self, session: i64) -> bool {
        let Some(of) = self.of.get(&session) else { return false };
        of.edit.is_some()
            || self.editing.is_some_and(|id| of.notes.iter().any(|note| note.state.entity_id() == id))
    }

    /// Replace the terminal's tasks and (non-empty) notes with `doc`. A note keeps its editor (folding,
    /// cursor, undo) when its text is still there, wherever it moved.
    pub(in crate::window) fn apply(
        &mut self,
        session: i64,
        doc: &NotesDoc,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel = self.panel;
        let Some(of) = self.of_mut(session) else { return };
        of.tasks.replace(doc.tasks.iter().map(|t| Task { text: t.text.as_str().into(), done: t.done }));
        // The sync waits while an editor is open; this is the backstop for one that isn't.
        if of.edit.as_ref().is_some_and(|edit| !edit.edit.new && edit.edit.ix >= of.tasks.len()) {
            of.edit = None;
        }

        let (filled, blank): (Vec<_>, Vec<_>) = std::mem::take(&mut of.notes)
            .into_iter()
            .partition(|note| !note.state.read(cx).value().trim().is_empty());
        let texts: Vec<String> =
            filled.iter().map(|note| note.state.read(cx).value().trim().to_string()).collect();
        let slots = note_slots(&texts, &doc.notes);
        let mut filled: Vec<Option<_>> = filled.into_iter().map(Some).collect();
        for (text, slot) in doc.notes.iter().zip(slots) {
            match slot.and_then(|i| Some((i, filled[i].take()?))) {
                Some((i, note)) => {
                    let Some(of) = self.of_mut(session) else { continue };
                    of.notes.push(note);
                    if texts[i] != text.trim() {
                        let at = of.notes.len() - 1;
                        self.set_note_text(session, at, SharedString::from(text.clone()), window, cx);
                    }
                }
                None => {
                    self.add_note(session, Some(text.clone()), false, panel, window, cx);
                }
            }
        }
        // Notes still being written (empty) stay, after the others.
        if let Some(of) = self.of_mut(session) {
            of.notes.extend(blank);
            of.recount(cx);
        }
        self.focused = None;
        self.dirty(cx);
    }
}
