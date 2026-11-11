// The keyboard-shortcut sheet as the core's keymap lists it (`keys/keymap/sheet.rs`): sections of labelled keys, the
// filter already applied. The rows, their order, the keys and what a filter finds are the core's; see `KeymapLists`.

import Foundation

public struct ShortcutRow: Sendable, Equatable, Identifiable {
    public var label: String
    /// The keys to draw: each action's side by side, or for a run its first and last.
    public var keys: [Shortcut]
    public var isRun: Bool
    /// Where the filter matched the label, as `Character` offsets, to mark.
    public var hit: Range<Int>?
    public var id: String { label }

    public init(label: String, keys: [Shortcut], isRun: Bool = false, hit: Range<Int>? = nil) {
        (self.label, self.keys, self.isRun, self.hit) = (label, keys, isRun, hit)
    }
}

public struct ShortcutSection: Sendable, Equatable, Identifiable {
    public var title: String
    public var rows: [ShortcutRow]
    public var id: String { title }

    public init(title: String, rows: [ShortcutRow]) { (self.title, self.rows) = (title, rows) }
}

/// The sheet as its filter leaves it, with the counts for the footer and the key its example finds ("⌘W").
public struct FilteredShortcuts: Sendable, Equatable {
    public var sections: [ShortcutSection]
    public var shown: Int
    public var total: Int
    public var exampleKey: String

    public init(sections: [ShortcutSection] = [], shown: Int = 0, total: Int = 0, exampleKey: String = "") {
        (self.sections, self.shown, self.total, self.exampleKey) = (sections, shown, total, exampleKey)
    }
}
