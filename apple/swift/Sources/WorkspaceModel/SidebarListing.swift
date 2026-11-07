// The sidebar's list as data: which rows, in which order, under which heading. Pure, so it is tested
// here and the view only draws it. The rules follow `wormhole-core` `view/sidebar`: pins come before the
// rest, a group is a head that can fold (a folded group lists no rows, so they get no number), and ⌘1–9
// count the first nine rows you can see, top to bottom (core's `SidebarNumbers::List`).

import Foundation

public struct SidebarRow: Sendable, Equatable, Identifiable {
    public var id: TerminalID { terminal.id }
    public var terminal: TerminalSummary
    /// ⌘1–⌘9 shortcut number, when the row has one.
    public var number: Int?
}

public struct SidebarSection: Sendable, Equatable, Identifiable {
    public var id: String
    public var title: String?  // nil: the ungrouped run
    public var isCollapsed: Bool
    /// Empty while the section is collapsed; `terminalCount` still says what is folded away.
    public var rows: [SidebarRow]
    /// Terminals in the section that pass the search, folded away or not, for a head's count.
    public var terminalCount: Int
    /// Of those, the ones that need input or failed: a folded head shows them as its alert dot.
    public var alertCount: Int
}

extension Activity {
    /// Needs the person now (core's `Urgency::is_alert`): what a folded head's dot stands for.
    var isAlert: Bool { self == .needsInput || self == .failed }
}

public enum SidebarListing {
    /// How many rows get a ⌘ number (core's `NUMBERED`).
    static let numbered = 9

    static let pinnedID = "pinned"
    static let ungroupedID = "ungrouped"

    /// A section before folding and numbering: every terminal in it that passes the search.
    private struct Run {
        var id: String
        var title: String?
        var isCollapsed: Bool
        var isGroup: Bool
        var terminals: [TerminalSummary]
    }

    /// Pinned terminals first (as one "Pinned" section: a pin lifts a terminal out of its group, as the
    /// state keeps no order inside a group to put it back in), then each group in its order, then
    /// whatever is in no group. Searching hides sections with nothing in them; without a search a group
    /// stays as a head even when empty, so it can be filled.
    public static func sections(for state: WorkspaceState) -> [SidebarSection] {
        let searching = !query(state.search).isEmpty
        var visible = 0  // rows listed so far: the next one's ⌘ number is one more
        var sections: [SidebarSection] = []
        for run in runs(in: state, matching: matcher(for: state.search))
        where !run.terminals.isEmpty || (run.isGroup && !searching) {
            var rows: [SidebarRow] = []
            if !run.isCollapsed {
                for terminal in run.terminals {
                    visible += 1
                    rows.append(SidebarRow(terminal: terminal, number: visible <= numbered ? visible : nil))
                }
            }
            sections.append(
                SidebarSection(
                    id: run.id, title: run.title, isCollapsed: run.isCollapsed, rows: rows,
                    terminalCount: run.terminals.count,
                    alertCount: run.terminals.filter { $0.activity.isAlert }.count))
        }
        return sections
    }

    /// Every terminal as listed, ignoring search and folds: what "the neighbour" means when one closes.
    static func order(of state: WorkspaceState) -> [TerminalID] {
        runs(in: state, matching: { _ in true }).flatMap { $0.terminals.map(\.id) }
    }

    private static func runs(in state: WorkspaceState, matching: (TerminalSummary) -> Bool) -> [Run] {
        let known = Dictionary(state.terminals.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        // A terminal belongs to the first place that claims it, so none is listed twice.
        var placed = Set<TerminalID>()
        func claim(_ terminals: [TerminalSummary]) -> [TerminalSummary] {
            var claimed: [TerminalSummary] = []
            for terminal in terminals where !placed.contains(terminal.id) && matching(terminal) {
                placed.insert(terminal.id)
                claimed.append(terminal)
            }
            return claimed
        }

        let pinned = claim(state.terminals.filter(\.isPinned))
        let grouped = state.groups.map { group in
            Run(
                id: group.id, title: group.name, isCollapsed: group.isCollapsed, isGroup: true,
                terminals: claim(group.members.compactMap { known[$0] }))
        }
        let inGroup = Set(state.groups.flatMap(\.members))
        let loose = claim(state.terminals.filter { !inGroup.contains($0.id) })
        return [Run(id: pinnedID, title: "Pinned", isCollapsed: false, isGroup: false, terminals: pinned)]
            + grouped
            + [Run(id: ungroupedID, title: nil, isCollapsed: false, isGroup: false, terminals: loose)]
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
