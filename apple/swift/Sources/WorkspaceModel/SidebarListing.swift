// The sidebar's list as data: which rows, in which order, under which heading. Pure, so it is tested
// here and the view only draws it. The rules are `wormhole-core` `view/sidebar`'s (`listing.rs`): the
// person's arrangement of loose terminals and groups (`Arrangement.swift`), pinned terminals first among
// the loose ones and at the top of their own group, a split's members together (`SplitRules.swift`), a
// folded group listing no rows, and ⌘1–9 on the first nine rows you can see (core's `SidebarNumbers::List`).

import Foundation

public struct SidebarRow: Sendable, Equatable, Identifiable {
    public var id: TerminalID { terminal.id }
    public var terminal: TerminalSummary
    /// ⌘1–⌘9 shortcut number, when the row has one.
    public var number: Int?
    /// How the row belongs to its split; empty for a terminal in none.
    public var splitMarks = SplitMarks()
    /// Whether the row shows its "±N": only the topmost row of a repository does, so the number is shown
    /// once, not on every row of it.
    public var showsChanges = false
}

public struct SidebarSection: Sendable, Equatable, Identifiable {
    public var id: String
    public var title: String?  // nil: a run of terminals in no group
    public var isCollapsed: Bool
    /// Empty while the section is collapsed; `terminalCount` still says what is folded away.
    public var rows: [SidebarRow]
    /// Terminals in the section that pass the search, folded away or not, for a head's count.
    public var terminalCount: Int
    /// Of those, the ones that need input or failed: a folded head shows them as its alert dot.
    public var alertCount: Int
    /// The group behind a titled section.
    public var group: TerminalGroup?
}

extension Activity {
    /// Needs the person now (core's `Urgency::is_alert`): what a folded head's dot stands for.
    var isAlert: Bool { self == .needsInput || self == .failed }
}

public enum SidebarListing {
    /// How many rows get a ⌘ number (core's `NUMBERED`).
    static let numbered = 9

    /// A section before search and numbering. `rows` are positions in `state.terminals`.
    private struct Run {
        var id: String
        var group: TerminalGroup?
        var rows: [Int]
    }

    private struct Layout {
        var runs: [Run] = []
        /// Every terminal top to bottom, folded ones included.
        var order: [Int] = []
        var marks: [Int: SplitMarks] = [:]
    }

    /// Runs of loose terminals and groups in the arrangement's order. Searching hides sections with
    /// nothing in them; without a search a group stays as a head even when empty, so it can be filled.
    public static func sections(for state: WorkspaceState) -> [SidebarSection] {
        let searching = !query(state.search).isEmpty
        let matches = matcher(for: state.search)
        let layout = layout(of: state)
        var visible = 0  // rows listed so far: the next one's ⌘ number is one more
        var sections: [SidebarSection] = []
        var repos = Set<String>()  // repositories a row above already stands for
        for run in layout.runs {
            let terminals = run.rows.map { state.terminals[$0] }
            let passing = zip(run.rows, terminals).filter { matches($1) }
            guard !passing.isEmpty || (run.group != nil && !searching) else { continue }
            let collapsed = run.group?.isCollapsed ?? false
            var rows: [SidebarRow] = []
            if !collapsed {
                for (ix, terminal) in passing {
                    visible += 1
                    let counted = terminal.repoID.map { !repos.insert($0).inserted } ?? false
                    rows.append(
                        SidebarRow(
                            terminal: terminal, number: visible <= numbered ? visible : nil,
                            splitMarks: layout.marks[ix] ?? SplitMarks(),
                            showsChanges: terminal.changeCount > 0 && !counted))
                }
            }
            sections.append(
                SidebarSection(
                    id: run.id, title: run.group?.name, isCollapsed: collapsed, rows: rows,
                    terminalCount: passing.count, alertCount: passing.filter { $1.activity.isAlert }.count,
                    group: run.group))
        }
        return sections
    }

    /// Every terminal as listed, ignoring search and folds: what "the neighbour" means when one closes.
    static func order(of state: WorkspaceState) -> [TerminalID] {
        layout(of: state).order.map { state.terminals[$0].id }
    }

    private static func layout(of state: WorkspaceState) -> Layout {
        var indexOf: [TerminalID: Int] = [:]
        for (ix, terminal) in state.terminals.enumerated() where indexOf[terminal.id] == nil { indexOf[terminal.id] = ix }
        let splits = SplitRules.normalised(state.terminals.map(\.splitID))
        let pinned = SplitRules.pinned(state.terminals.map(\.isPinned), splits)
        func pinsFirst(_ order: [Int]) -> [Int] { SplitRules.withPins(order, pinned) }

        let arranged = state.arranged()
        let looseIDs = arranged.compactMap { item -> Int? in
            if case .terminal(let id) = item { indexOf[id] } else { nil }
        }
        // The loose terminals are pinned and gathered by split as one list, then dealt back to their places.
        let loose = SplitRules.gather(pinsFirst(looseIDs), splits)
        let lead = loose.prefix { pinned[$0] }.count
        var rest = loose.dropFirst(lead).makeIterator()

        var layout = Layout()
        var bands: [Int?] = []  // the group of each place in `order`
        var shown = [Bool](repeating: false, count: state.terminals.count)
        func addLoose(_ ix: Int) {
            if layout.runs.last?.group != nil || layout.runs.isEmpty {
                layout.runs.append(Run(id: "loose-\(state.terminals[ix].id)", group: nil, rows: []))
            }
            layout.runs[layout.runs.count - 1].rows.append(ix)
            layout.order.append(ix)
            bands.append(nil)
            shown[ix] = true
        }
        func addGroup(_ group: TerminalGroup, band: Int) {
            let members = group.members.filter { state.group(of: $0) == group.id }.compactMap { indexOf[$0] }
            let rows = SplitRules.gather(pinsFirst(members), splits)
            layout.runs.append(Run(id: group.id, group: group, rows: rows))
            layout.order += rows
            bands += rows.map { _ in band }
            for ix in rows { shown[ix] = !group.isCollapsed }
        }
        for ix in loose.prefix(lead) { addLoose(ix) }
        for item in arranged {
            switch item {
            case .terminal: if let ix = rest.next() { addLoose(ix) }
            case .group(let id):
                if let at = state.groups.firstIndex(where: { $0.id == id }) { addGroup(state.groups[at], band: at) }
            }
        }
        layout.marks = SplitRules.marks(
            order: layout.order, band: bands, shown: shown, ids: splits,
            selected: state.selected.flatMap { indexOf[$0] })
        return layout
    }

    private static func query(_ search: String) -> String {
        search.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func matcher(for search: String) -> (TerminalSummary) -> Bool {
        let needle = query(search)
        guard !needle.isEmpty else { return { _ in true } }
        return { terminal in
            [terminal.title, terminal.subtitle, terminal.branch].contains {
                $0?.range(of: needle, options: .caseInsensitive) != nil
            }
        }
    }
}
