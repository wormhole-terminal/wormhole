// Terminals as tabs instead of the sidebar (`view/sidebar/tabs.rs`): a tab set is what a group is in the
// list; the bar shows the selected terminal's set as tabs and the others one step away.

import Foundation
import WordsModel
import WorkspaceModel

public struct TabSet: Sendable, Equatable {
    /// Nil for the terminals in no group.
    public var groupID: GroupID?
    public var label: String
    /// Its terminals in the list's order, folded ones too: the tab bar has no folds.
    public var members: [Int]

    public init(groupID: GroupID?, label: String, members: [Int]) { (self.groupID, self.label, self.members) = (groupID, label, members) }
}

public enum TabSets {
    /// What the terminals in no group are called as a set.
    public static var ungrouped: String { Words.text("tab-switcher-ungrouped", fallback: "Ungrouped") }
    /// How many sets a key reaches (⌃⌥1–9).
    public static let numbered = 9

    /// Every group (an empty pinned one is somewhere to put a new terminal), then the loose terminals.
    public static func of(groups: [TabSet], loose: [Int]) -> [TabSet] {
        loose.isEmpty ? groups : groups + [TabSet(groupID: nil, label: ungrouped, members: loose)]
    }

    public static func current(_ sets: [TabSet], containing terminal: Int) -> Int? {
        sets.firstIndex { $0.members.contains(terminal) }
    }

    /// The set one step from `current`, wrapping; empty sets are passed over (somewhere to put a new
    /// terminal, not somewhere to arrive by a key). Nil when no other set has a terminal.
    public static func step(_ sets: [TabSet], from current: Int, forward: Bool) -> Int? {
        let n = sets.count
        return (1..<max(n, 1)).lazy.map { forward ? (current + $0) % n : (current + n - $0) % n }
            .first { !sets[$0].members.isEmpty }
    }

    /// The set with this number (1 to 9, as the switcher lists them, empty ones included).
    public static func numbered(_ sets: [TabSet], _ number: Int) -> Int? {
        (1...numbered).contains(number) && number - 1 < sets.count ? number - 1 : nil
    }

    /// What the switcher's search finds: a set by its name, or a terminal by its name.
    public enum Hit: Sendable, Equatable {
        case set(Int)
        /// The terminal, in the set at `.1`.
        case terminal(Int, set: Int)
    }

    /// The sets whose names contain `query` (case does not matter), then the terminals whose names do.
    /// Nothing for a blank query: the switcher shows its cards then.
    public static func search(_ sets: [TabSet], _ query: String, name: (Int) -> String) -> [Hit] {
        let query = query.trimmingCharacters(in: .whitespaces).lowercased()
        guard !query.isEmpty else { return [] }
        let found = { (text: String) in text.lowercased().contains(query) }
        let named = sets.indices.filter { found(sets[$0].label) }.map(Hit.set)
        let terminals = sets.indices.flatMap { at in sets[at].members.filter { found(name($0)) }.map { Hit.terminal($0, set: at) } }
        return named + terminals
    }
}
