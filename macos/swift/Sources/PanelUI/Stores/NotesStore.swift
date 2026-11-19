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
    /// The steps and notes whose editor is open, and the note last written in (⇧⌘E comes back to it).
    @ObservationIgnored private var writers: Set<UUID> = []
    @ObservationIgnored private var lastNote: Note.ID?
    @ObservationIgnored private weak var host: (any PanelHost)?

    public init(model: NotesModel = NotesModel(), host: (any PanelHost)? = nil) {
        self.model = model
        self.host = host
    }

    /// A next step goes to the terminal's prompt as it reads.
    public func send(_ task: PanelTask) { send(text: task.text, submit: false) }

    public func send(_ note: Note) { send(text: note.text, submit: false) }

    /// ⌘↵ in a note or step: `text` goes to the prompt, and with ⇧ the program is told to run it.
    public func send(text: String, submit: Bool) {
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        host?.send(text.trimmingCharacters(in: .whitespacesAndNewlines))
        // The terminal has the keys from here: the person reads what was pasted and goes on there.
        host?.focusTerminal()
        guard submit else { return }
        Task { [host] in
            // one-shot: the program takes the paste first; Return a moment later is what the person would press.
            try? await Task.sleep(for: .milliseconds(120))
            host?.submit()
        }
    }

    /// Another terminal was selected: an open draft row stays with the one it was started in.
    public func leaveDraft() { (drafting, draftAfter) = (false, nil) }

    /// A step or note of this terminal is being written: the notes.md merge waits for it (`api.rs` `being_written`).
    public var isWriting: Bool { drafting || !writers.isEmpty }

    /// A step's or a note's editor opened or closed.
    public func setWriting(_ id: UUID, _ on: Bool, note: Bool = false) {
        if on { writers.insert(id) } else { writers.remove(id) }
        if on, note { lastNote = id }
    }

    /// ⇧⌘E from the terminal: the note last worked on takes the keys, else the first, else a new one
    /// (`note_ops.rs` `focus_in`).
    public func focusIn() {
        model.notesCollapsed = false
        if let lastNote, model.notes.contains(where: { $0.id == lastNote }) {
            freshNote = lastNote
        } else if let first = model.notes.first {
            freshNote = first.id
        } else {
            newNote()
        }
    }

    /// Backspace in the empty step being written: it is dropped, and the step above it opens (or the keys go back).
    public func backOutOfDraft() {
        let above = draftAfter ?? model.tasks.items.last?.id
        drafting = false
        if let above { freshStep = above } else { leave() }
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
        // The note is not opened for writing: the keys stay in the terminal, to answer the agent's question.
        model.notes.append(Note(text: text))
        model.notesCollapsed = false
        host?.notify(panelText("notes-prompt-stashed", "Prompt stashed in a new note"))
        host?.focusTerminal()
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
