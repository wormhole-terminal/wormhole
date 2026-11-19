// The values a folder provider and the picker's panel trade (`view/picker/types.rs`). All plain, owned data, so
// a provider written elsewhere (the Rust one over the FFI, a Probe later) answers in the same words.

import Foundation
import WorkspaceModel

/// Where the picker was opened: what a provider may use to make sense of the typed text.
public struct PickerContext: Sendable, Equatable {
    /// The folder of the terminal the picker is for.
    public var cwd: String
    public var home: String
    public var profile: String
    public init(cwd: String, home: String = HomeFolder.path, profile: String = "default") {
        (self.cwd, self.home, self.profile) = (cwd, home, profile)
    }
}

public struct PickerQuery: Sendable, Equatable {
    public var text: String
    public var context: PickerContext
    public init(text: String, context: PickerContext) { (self.text, self.context) = (text, context) }
}

public struct PickerRow: Sendable, Equatable, Identifiable {
    /// What the row is, the same whenever it is listed again: decorations and the selection kept across answers
    /// go by it.
    public var key: String
    public var label: String
    /// Where the typed text matched in `label`, as character offsets, to highlight.
    public var marks: [Range<Int>] = []
    /// An SF Symbol's name.
    public var icon = "folder"
    public var badge: String?
    public var detail: String?
    /// There is more behind this row (a chevron): a decorator may set it later.
    public var chevron = false
    /// The field text that goes into this row (⇥, →, the ghost); nil: it can't be gone into.
    public var descend: String?
    /// What picking the row hands over; nil: it can't be picked.
    public var insert: String?
    /// The section it is listed in: an index into `PickerAnswer.sections`.
    public var section = 0
    /// The person can ask to forget this row (⌘⌫).
    public var forgettable = false
    public var kind = "folder"
    public var score = 0

    public var id: String { key }

    public init(key: String, label: String) { (self.key, self.label) = (key, label) }
}

public struct PickerCrumb: Sendable, Equatable {
    public var label: String
    /// The text that goes in the field when it is clicked.
    public var text: String
    public init(label: String, text: String) { (self.label, self.text) = (label, text) }
}

/// Something the list says besides its rows.
public enum PickerNote: Sendable, Equatable {
    case empty, failed, reading
    case limited(Int)
    case searched(Int)
    case custom(String)
}

public struct PickerAnswer: Sendable, Equatable {
    /// The titles of the sections, in order; an empty title is a section with no header.
    public var sections: [String] = [""]
    public var rows: [PickerRow] = []
    public var notes: [PickerNote] = []
    /// The text that goes in the field to go one level up (←).
    public var up: String?
    /// ← at the end of the text goes up too, because the text ends where a level does (a path's closing `/`).
    public var upAtEnd = false
    public var crumbs: [PickerCrumb] = []
    public init() {}
}

/// What a decorator found out about a row, by its key. A field left nil says nothing.
public struct PickerDecoration: Sendable, Equatable {
    public var key: String
    public var badge: String?
    public var chevron: Bool?
    public init(key: String, badge: String? = nil, chevron: Bool? = nil) { (self.key, self.badge, self.chevron) = (key, badge, chevron) }
}
