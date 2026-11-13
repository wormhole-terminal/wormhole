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

    /// The lines after the title, which show under it while the note is open.
    public var body: String {
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false).drop(while: { $0.isEmpty })
        return lines.dropFirst().joined(separator: "\n").trimmingCharacters(in: .newlines)
    }
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
