// The Notes tab's state and what it does to the terminal: sending a step or a note to the prompt, stashing the
// prompt into a new note, moving steps and notes by hand. The text itself is `NotesModel`, a value.

import Foundation
import Observation
import PanelModel

@MainActor @Observable
public final class NotesStore {
    public var model: NotesModel
    /// The note the stash just made, which opens for writing at once.
    public private(set) var freshNote: Note.ID?
    @ObservationIgnored private weak var host: (any PanelHost)?

    public init(model: NotesModel = NotesModel(), host: (any PanelHost)? = nil) {
        self.model = model
        self.host = host
    }

    /// A next step goes to the terminal's prompt as it reads.
    public func send(_ task: PanelTask) { host?.send(task.text) }

    public func send(_ note: Note) { host?.send(note.text) }

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
