// The Notes tab's state and what it does to the terminal: sending a step or a note to the prompt, stashing the
// prompt into a new note, moving steps and notes by hand. The text itself is `NotesModel`, a value.

import Foundation
import Observation
import PanelModel

@MainActor @Observable
public final class NotesStore {
    public var model: NotesModel
    /// The row the keys are on (`PanelWalking`).
    public var cursor: String?
    /// The note that opens for writing at once (a new one, the stash's, the one above a deleted empty one).
    public internal(set) var freshNote: Note.ID?
    /// The same for a next step: the row that was asked to open its editor.
    public internal(set) var freshStep: PanelTask.ID?
    /// A new, empty step is being written, and where: after this step, or at the end when nil (↵ in a step opens the next one below).
    public var draftAfter: PanelTask.ID?
    public var drafting = false
    @ObservationIgnored private weak var host: (any PanelHost)?

    public init(model: NotesModel = NotesModel(), host: (any PanelHost)? = nil) {
        self.model = model
        self.host = host
    }

    /// A next step goes to the terminal's prompt as it reads.
    public func send(_ task: PanelTask) { host?.send(task.text) }

    public func send(_ note: Note) { host?.send(note.text) }

    /// ⌘↵ in a note or step: `text` goes to the prompt, and with ⇧ the program is told to run it.
    public func send(text: String, submit: Bool) {
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        host?.send(text.trimmingCharacters(in: .whitespacesAndNewlines))
        if submit { host?.submit() }
    }

    /// Esc, or ↵ on an empty field: the person is done writing and the keys go back to the terminal.
    public func leave() { host?.focusTerminal() }

    /// A new, empty note, open for writing (the band's +, ⌥⌘N).
    public func newNote() { add(note: "") }

    /// Backspace in an empty note: it goes, and the one above opens (or the keys go back when it was the first).
    public func removeEmpty(_ note: Note) {
        guard let at = model.notes.firstIndex(where: { $0.id == note.id }) else { return }
        model.notes.remove(at: at)
        if at > 0 { freshNote = model.notes[at - 1].id } else { leave() }
    }

    /// A note's writing ended: one with nothing in it is thrown away.
    public func finishWriting(_ note: Note) {
        if note.isBlank { model.notes.removeAll { $0.id == note.id } }
    }

    /// Backspace in an empty step: it goes, and the one above opens.
    public func removeEmpty(_ task: PanelTask) {
        let items = model.tasks.items
        guard let at = items.firstIndex(where: { $0.id == task.id }) else { return }
        model.tasks.delete(task.id)
        if at > 0 { freshStep = items[at - 1].id } else { leave() }
    }

    public func consumeFreshStep() { freshStep = nil }

    /// What is typed in the terminal's input box (an agent's, as core reads it) becomes a new note, open for writing.
    /// Nothing there, nothing happens.
    @discardableResult
    public func stashPrompt() -> Bool {
        guard let text = host?.takePrompt(), !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return false }
        add(note: text)
        host?.notify("Prompt stashed in a new note")
        return true
    }

    /// A new note with `text` in it, open for writing.
    public func add(note text: String) {
        let note = Note(text: text)
        model.notes.append(note)
        model.notesCollapsed = false
        freshNote = note.id
    }

    public func moveTask(from: Int, to: Int) { model.tasks.move(from: from, to: to) }

    public func moveNote(from: Int, to: Int) { Reorder.move(&model.notes, from: from, to: to) }

    public func consumeFresh() { freshNote = nil }
}
