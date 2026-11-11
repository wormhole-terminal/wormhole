// What the menus show about the window in front (`keymap::menus::MenuState`): which items flip, are checked,
// disabled or hidden, and the parts that follow the window. The window says it; the builder reads it.

import Foundation

public struct MenuState: Equatable, Sendable {
    /// Toggles whose thing is off: their title flips ("Show Sidebar").
    public var off: Set<ActionID> = []
    public var checked: Set<ActionID> = []
    public var disabled: Set<ActionID> = []
    /// Actions whose feature is off: not in the menus at all.
    public var hidden: Set<ActionID> = []
    /// The sidebar's numbered terminals (⌘1–9) by name, and which one is selected.
    public var terminals: [String] = []
    public var currentTerminal: Int?
    /// Recent folders' names, most recent first.
    public var recent: [String] = []
    /// The front terminal's project actions: (id, title).
    public var projectActions: [ProjectAction] = []
    /// The shells New Terminal With offers, by name.
    public var shells: [String] = []
    /// The profiles' names, which one Wormhole is in, and which one the window in front is in.
    public var profiles: [String] = []
    public var currentProfile: Int?
    public var windowProfile: Int?
    /// The selected terminal is in a split: ⌘W closes its pane.
    public var split = false
    /// What Undo and Redo would undo in the key window (`Change “primary”`).
    public var undo: String?
    public var redo: String?
    /// A debug build: the Developer menu is there.
    public var debug = false
    /// Persistent terminals are on: the items about terminals that outlive the app are there.
    public var persistent = false

    public struct ProjectAction: Equatable, Sendable {
        public var id: String
        public var title: String
        public init(id: String, title: String) { (self.id, self.title) = (id, title) }
    }

    public init() {}

    /// Whether `other` differs from this only in what the terminals are called (an agent's title changes as it
    /// works), which the menus may show a little late.
    public func differsOnlyInNames(from other: MenuState) -> Bool {
        var a = self, b = other
        a.terminals = []
        b.terminals = []
        return self != other && terminals.count == other.terminals.count && a == b
    }
}
