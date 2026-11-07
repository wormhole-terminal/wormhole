// What the group actions do, after core's `Groups` (`view/sidebar/groups.rs`) and `WindowSidebar`
// (`arrange.rs`): a group left with no terminal goes unless it is pinned, a group with a folder is
// pinned (it is a place), a new one takes the next colour in turn, and ungrouping leaves the terminals
// where the group stood.

import Foundation

extension WorkspaceState {
    private func groupIndex(_ id: String) -> Int? { groups.firstIndex { $0.id == id } }

    /// "Group 1", "Group 2" …: the first no group is called.
    private var nextGroupName: String {
        let names = Set(groups.map(\.name))
        return "Group \((1...).first { !names.contains("Group \($0)") } ?? 1)"
    }

    private var nextGroupColor: FolderColor { FolderColor.allCases[groups.count % FolderColor.allCases.count] }

    private func groupName(_ name: String?) -> String {
        let name = name?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return name.isEmpty ? nextGroupName : name
    }

    /// An empty group at the end of the list.
    mutating func addGroup(named name: String) {
        groups.append(
            TerminalGroup(id: nextGroupID, name: groupName(name), members: [], color: nextGroupColor))
    }

    /// A group holding `id` and the rest of its split, in the place of `id` (or just after its group).
    mutating func addGroup(holding id: TerminalID, named name: String?) {
        guard terminals.contains(where: { $0.id == id }) else { return }
        let members = splitMates(of: id)
        let group = TerminalGroup(
            id: nextGroupID, name: groupName(name), members: members, color: nextGroupColor)
        let anchor = self.group(of: members[0]).map(TopItem.group) ?? .terminal(members[0])
        var items: [TopItem] = []
        for item in arranged() {
            let movesIn: Bool = if case .terminal(let loose) = item { members.contains(loose) } else { false }
            if !movesIn { items.append(item) }
            guard item == anchor else { continue }
            // A loose terminal's place is the group's; a group's is just before it.
            if case .terminal = item { items.append(.group(group.id)) } else { items += [item, .group(group.id)] }
        }
        for index in groups.indices { groups[index].members.removeAll { members.contains($0) } }
        groups.removeAll { !$0.isPinned && $0.members.isEmpty }
        groups.append(group)
        arrangement = items
        arrangement = arranged()
    }

    mutating func renameGroup(_ id: String, to name: String) {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        if let index = groupIndex(id), !name.isEmpty { groups[index].name = name }
    }

    /// A folder makes the group a place, so it is pinned; taking it away leaves it pinned.
    mutating func setGroupFolder(_ id: String, to folder: String?) {
        guard let index = groupIndex(id) else { return }
        let folder = folder?.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty
        groups[index].folder = folder
        if folder != nil { groups[index].isPinned = true }
    }

    mutating func togglePinGroup(_ id: String) {
        guard let index = groupIndex(id) else { return }
        groups[index].isPinned.toggle()
        groups.removeAll { !$0.isPinned && $0.members.isEmpty }
    }

    /// Take a group apart: its terminals stand where it stood, in the order they had in it.
    mutating func ungroup(_ id: String) {
        guard let index = groupIndex(id) else { return }
        let members = groups[index].members
        groups.remove(at: index)
        arrangement = arrangedWith(group: id, replacedBy: members)
    }

    /// Items as `arranged()` lists them, with a group's place taken by `terminals` (before it is gone).
    private func arrangedWith(group id: String, replacedBy terminals: [TerminalID]) -> [TopItem] {
        var items: [TopItem] = []
        for item in arrangement {
            if item == .group(id) { items += terminals.map(TopItem.terminal) } else { items.append(item) }
        }
        return items
    }

    mutating func closeGroup(_ id: String) {
        if let index = groupIndex(id) { close(Set(groups[index].members)) }
    }

    /// Take a terminal and its split out of their group: they stand right after it, in the order they have.
    mutating func removeFromGroup(_ id: TerminalID) {
        guard let owner = group(of: id) else { return }
        let items = arranged()
        let after = items.firstIndex(of: .group(owner)).flatMap { items.indices.contains($0 + 1) ? items[$0 + 1] : nil }
        for member in splitMates(of: id) { move(member, to: .top(before: after)) }
    }

    /// A terminal in the group's folder, last in the group.
    mutating func addTerminal(inGroup id: String) {
        guard let index = groupIndex(id) else { return }
        move(addTerminal(folder: groups[index].folder), to: .group(id, before: nil))
    }
}

extension String {
    var nilIfEmpty: String? { isEmpty ? nil : self }
}
