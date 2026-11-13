// Terminals as tabs instead of the sidebar (`view/sidebar/tabs.rs`): a tab set is what a group is in the
// list; the bar shows the selected terminal's set as tabs and the others one step away.

import Foundation
import Synchronization
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

/// The tab bar's rules, the core's (`view::sidebar::tabs`): which set a terminal is in, the one a step or a number reaches, and
/// what the switcher's search finds. `WorkspaceBridge` installs them; without the core (no FFI) no set is reached by a key and
/// nothing is found, so a build without it is inert, never different.
public protocol TabSetRules: Sendable {
    func current(_ sets: [TabSet], containing terminal: Int) -> Int?
    func step(_ sets: [TabSet], from current: Int, forward: Bool) -> Int?
    func numbered(_ sets: [TabSet], _ number: Int) -> Int?
    func search(_ sets: [TabSet], _ query: String, name: (Int) -> String) -> [TabSets.Hit]
}

public enum TabSets {
    private static let installed = Mutex<(any TabSetRules)?>(nil)

    public static func install(_ rules: (any TabSetRules)?) { installed.withLock { $0 = rules } }

    private static var rules: any TabSetRules { installed.withLock { $0 } ?? Inert() }

    private struct Inert: TabSetRules {
        func current(_ sets: [TabSet], containing terminal: Int) -> Int? { nil }
        func step(_ sets: [TabSet], from current: Int, forward: Bool) -> Int? { nil }
        func numbered(_ sets: [TabSet], _ number: Int) -> Int? { nil }
        func search(_ sets: [TabSet], _ query: String, name: (Int) -> String) -> [Hit] { [] }
    }

    /// What the terminals in no group are called as a set.
    public static var ungrouped: String { Words.text("tab-switcher-ungrouped", fallback: "Ungrouped") }

    /// Every group (an empty pinned one is somewhere to put a new terminal), then the loose terminals: the core's listing
    /// of them (`tab_sets`) from the groups and terminals the window already has, whose label is the language's.
    public static func of(groups: [TabSet], loose: [Int]) -> [TabSet] {
        loose.isEmpty ? groups : groups + [TabSet(groupID: nil, label: ungrouped, members: loose)]
    }

    public static func current(_ sets: [TabSet], containing terminal: Int) -> Int? { rules.current(sets, containing: terminal) }

    /// The set one step from `current`, wrapping; empty sets are passed over. Nil when no other set has a terminal.
    public static func step(_ sets: [TabSet], from current: Int, forward: Bool) -> Int? {
        rules.step(sets, from: current, forward: forward)
    }

    /// The set with this number (1 to 9, as the switcher lists them, empty ones included).
    public static func numbered(_ sets: [TabSet], _ number: Int) -> Int? { rules.numbered(sets, number) }

    /// What the switcher's search finds: a set by its name, or a terminal by its name.
    public enum Hit: Sendable, Equatable {
        case set(Int)
        /// The terminal, in the set at `.1`.
        case terminal(Int, set: Int)
    }

    /// The sets whose names contain `query` (case does not matter), then the terminals whose names do.
    /// Nothing for a blank query: the switcher shows its cards then.
    public static func search(_ sets: [TabSet], _ query: String, name: (Int) -> String) -> [Hit] {
        rules.search(sets, query, name: name)
    }
}
