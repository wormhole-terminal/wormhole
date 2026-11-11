// A terminal's notes and tasks (`NotesModel`) as the records Rust saves and merges, and back. The merge with
// `notes.md` is core's (`plan_sync`); what this adds is only keeping a note's identity (its editor, its fold)
// across a merge, by the slots Rust works out (`note_slots`).

import Foundation
import PanelModel
import WormholeBindings

extension NotesModel {
    /// What was saved for a terminal: the folds with the notes and steps.
    init(saved: NotesSaved) {
        self.init(
            notes: saved.notes.map { Note(text: $0.body, collapsed: $0.collapsed) },
            tasks: TaskList(saved.tasks.map { PanelTask(text: $0.text, done: $0.done) }))
        stepsCollapsed = saved.tasksCollapsed
        notesCollapsed = saved.notesCollapsed
    }

    var saved: NotesSaved {
        NotesSaved(
            notes: notes.map { NoteSaved(body: $0.text, collapsed: $0.collapsed) },
            tasks: tasks.items.map { TaskSaved(text: $0.text, done: $0.done) },
            tasksCollapsed: stepsCollapsed, notesCollapsed: notesCollapsed)
    }

    /// The document the sync compares: the steps, and the notes with something in them (trimmed), in order.
    var document: NotesDocRecord {
        NotesDocRecord(
            tasks: tasks.items.map { TaskDocRecord(text: $0.text, done: $0.done) },
            notes: filledNotes.map { $0.text.trimmingCharacters(in: .whitespacesAndNewlines) })
    }

    private var filledNotes: [Note] {
        notes.filter { !$0.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    }

    /// This model with a merged document laid over it. A note keeps its identity (and fold) when its text is still
    /// there, wherever it moved, or when it was edited in place (`slots`); notes still being written (empty) stay,
    /// after the others.
    func applying(_ outcome: NotesSyncOutcome) -> NotesModel? {
        guard let merged = outcome.apply else { return nil }
        let filled = filledNotes
        var result = self
        result.tasks = TaskList(merged.tasks.map { PanelTask(text: $0.text, done: $0.done) })
        result.notes = zip(merged.notes, outcome.noteSlots).map { text, slot in
            guard let slot, filled.indices.contains(Int(slot)) else { return Note(text: text) }
            var kept = filled[Int(slot)]
            if kept.text.trimmingCharacters(in: .whitespacesAndNewlines) != text { kept.text = text }
            return kept
        }
        result.notes += notes.filter { $0.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
        return result
    }
}
