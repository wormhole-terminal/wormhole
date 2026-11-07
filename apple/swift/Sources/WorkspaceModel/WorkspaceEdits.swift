// The state changes behind the actions that are more than a flag: each a small mutation on the value,
// so `reduce` stays a table of what each action means. Groups are in `GroupEdits.swift`, splits in
// `SplitEdits.swift`.

import Foundation

extension WorkspaceState {
    /// The id the next terminal gets: never one still open. UIs ask here, they never mirror the rule.
    public var nextTerminalID: TerminalID { (terminals.map(\.id).max() ?? 0) + 1 }

    /// The id the next group gets: the first free "group-N".
    public var nextGroupID: String {
        let taken = Set(groups.map(\.id))
        return "group-\((1...).first { !taken.contains("group-\($0)") } ?? 1)"
    }

    /// A new terminal at the end, selected; the number in its title follows its id.
    @discardableResult
    mutating func addTerminal(folder: String? = nil, splitID: Int? = nil) -> TerminalID {
        let id = nextTerminalID
        terminals.append(TerminalSummary(id: id, title: "Terminal \(id)", folder: folder, splitID: splitID))
        selected = id
        // A search that hides the new terminal would leave the selection invisible.
        search = ""
        return id
    }

    /// A blank name keeps the old one, as core's `Groups::rename` does.
    mutating func rename(_ id: TerminalID, to name: String) {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, let index = terminals.firstIndex(where: { $0.id == id }) else { return }
        terminals[index].title = name
    }

    /// Close terminals. A group that lost one and has none left goes with them, unless it is pinned
    /// (core's `retain_terminals`); a closed selection moves to the row that slides into its place.
    mutating func close(_ ids: Set<TerminalID>) {
        let listed = SidebarListing.order(of: self)
        let touched = Set(groups.filter { $0.members.contains { ids.contains($0) } }.map(\.id))
        terminals.removeAll { ids.contains($0.id) }
        for index in groups.indices { groups[index].members.removeAll { ids.contains($0) } }
        groups.removeAll { touched.contains($0.id) && !$0.isPinned && $0.members.isEmpty }
        if let current = selected, ids.contains(current) {
            let before = listed.prefix { $0 != current }.filter { !ids.contains($0) }.count
            let remaining = listed.filter { !ids.contains($0) }
            selected = remaining.isEmpty ? nil : remaining[min(before, remaining.count - 1)]
        }
    }

    mutating func toggle(_ id: TerminalID, _ flag: WritableKeyPath<TerminalSummary, Bool>) {
        if let index = terminals.firstIndex(where: { $0.id == id }) { terminals[index][keyPath: flag].toggle() }
    }
}
