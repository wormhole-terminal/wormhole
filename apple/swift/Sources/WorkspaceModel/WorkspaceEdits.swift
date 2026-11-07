// The state changes behind the actions that are more than a flag: each a small mutation on the value,
// so `reduce` stays a table of what each action means.

import Foundation

extension WorkspaceState {
    /// A new terminal at the end, never reusing the id of one still open; the number in its title follows it.
    @discardableResult
    mutating func addTerminal(folder: String? = nil, splitID: Int? = nil) -> TerminalID {
        let id = (terminals.map(\.id).max() ?? 0) + 1
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

    /// An empty group at the end of the list; a blank name is "Group N".
    mutating func addGroup(named name: String) {
        let taken = Set(groups.map(\.id))
        let number = (1...).first { !taken.contains("group-\($0)") } ?? 1
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        groups.append(TerminalGroup(id: "group-\(number)", name: name.isEmpty ? "Group \(groups.count + 1)" : name, members: []))
    }

    /// A terminal beside another, in the same split and group: it joins the split's members, after the last.
    mutating func splitRight(_ anchor: TerminalID) {
        guard let at = terminals.firstIndex(where: { $0.id == anchor }) else { return }
        let split = terminals[at].splitID ?? (terminals.compactMap(\.splitID).max() ?? 0) + 1
        terminals[at].splitID = split
        let anchorGroup = group(of: anchor)
        let mates = Set(terminals.filter { $0.splitID == split }.map(\.id))
        let id = addTerminal(folder: terminals[at].folder, splitID: split)
        if let last = terminals.indices.last { terminals[last].isPinned = terminals[at].isPinned }
        guard let group = groups.firstIndex(where: { $0.id == anchorGroup }) else { return }
        let after = groups[group].members.lastIndex { mates.contains($0) } ?? groups[group].members.count - 1
        groups[group].members.insert(id, at: after + 1)
    }
}
