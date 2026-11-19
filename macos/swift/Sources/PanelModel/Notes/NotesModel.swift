// What the Notes tab shows for one terminal: the Next steps card and the Notes card, each folded or not.

import Foundation
import WordsModel

public struct Note: Identifiable, Hashable, Sendable {
    public let id: UUID
    public var text: String
    public var collapsed = false

    public init(id: UUID = UUID(), text: String, collapsed: Bool = false) {
        self.id = id
        self.text = text
        self.collapsed = collapsed
    }

    /// The first line without a list's or a heading's marks is the title; an empty note is still being written.
    public var title: String { NoteRules.title(text) ?? Words.text("notes-new", fallback: "New note") }

    /// Nothing but whitespace: such a note is thrown away when its writing ends.
    public var isBlank: Bool { text.allSatisfy(\.isWhitespace) }

    /// What shows under the title while the note is open (core's `note_body`); empty for a note of one line.
    public var body: String { NoteRules.body(text) ?? "" }
}

public struct NotesModel: Equatable, Sendable {
    public var notes: [Note]
    public var tasks: TaskList
    public var stepsCollapsed = false
    public var notesCollapsed = false
    public var showDone = false

    public init(notes: [Note] = [], tasks: TaskList = TaskList()) {
        self.notes = notes
        self.tasks = tasks
    }

    /// Notes with something in them: the band's count.
    public var filledNotes: Int { notes.filter { !$0.isBlank }.count }
}
