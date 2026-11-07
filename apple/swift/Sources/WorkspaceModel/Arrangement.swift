// The order of the terminal list, after `wormhole-core` `view/sidebar/arrange.rs`: terminals in no group
// and groups, side by side in the order the person gave them, like a browser's tabs and tab groups. A
// group's terminals are its own, so the top level holds only the loose terminals and the groups.
// What the saved order doesn't know yet (a terminal made since) comes after, loose terminals first and
// then the groups, so nothing is ever lost or reshuffled.

import Foundation

/// One thing at the top level of the list.
public enum TopItem: Sendable, Equatable, Hashable {
    /// A terminal in no group.
    case terminal(TerminalID)
    case group(String)
}

/// A place in the arrangement: at the top level before an item (last without one), or in a group before
/// one of its terminals (last without one).
public enum Landing: Sendable, Equatable {
    case top(before: TopItem?)
    case group(String, before: TerminalID?)
}

extension WorkspaceState {
    /// The group a terminal is in; the first that lists it if two do.
    func group(of id: TerminalID) -> String? {
        groups.first { $0.members.contains(id) }?.id
    }

    /// The top level as listed: the saved order without what no longer exists, then what it doesn't know yet.
    func arranged() -> [TopItem] {
        var items: [TopItem] = []
        func add(_ item: TopItem) { if !items.contains(item) { items.append(item) } }
        let live = Set(terminals.map(\.id))
        for item in arrangement {
            switch item {
            case .terminal(let id) where live.contains(id) && group(of: id) == nil: add(item)
            case .group(let id) where groups.contains { $0.id == id }: add(item)
            default: break
            }
        }
        for terminal in terminals where group(of: terminal.id) == nil { add(.terminal(terminal.id)) }
        for group in groups { add(.group(group.id)) }
        return items
    }

    /// Move a terminal. Leaving a group takes the group with its last terminal, unless it is pinned.
    mutating func move(_ id: TerminalID, to landing: Landing) {
        guard terminals.contains(where: { $0.id == id }) else { return }
        switch landing {
        case .top(let before):
            var items = arranged()
            leaveGroups(id)
            place(.terminal(id), in: &items, before: before)
            settle(items)
        case .group(let target, let before):
            guard groups.contains(where: { $0.id == target }) else { return }
            let items = arranged()
            leaveGroups(id, keeping: target)
            if let at = groups.firstIndex(where: { $0.id == target }) {
                let spot = before.flatMap { groups[at].members.firstIndex(of: $0) } ?? groups[at].members.count
                groups[at].members.insert(id, at: spot)
            }
            settle(items)
        }
    }

    /// Keep `items` as the arrangement, completed with what it doesn't know (a group that went, a terminal
    /// that joined one).
    private mutating func settle(_ items: [TopItem]) {
        arrangement = items
        arrangement = arranged()
    }

    /// Move a group among the top level's items.
    mutating func moveGroup(_ id: String, before: TopItem?) {
        guard groups.contains(where: { $0.id == id }), before != .group(id) else { return }
        var items = arranged()
        place(.group(id), in: &items, before: before)
        arrangement = items
    }

    private func place(_ item: TopItem, in items: inout [TopItem], before: TopItem?) {
        items.removeAll { $0 == item }
        let at = before.flatMap { items.firstIndex(of: $0) } ?? items.count
        items.insert(item, at: at)
    }

    /// Take a terminal out of its groups; one left with none goes, unless it is pinned.
    private mutating func leaveGroups(_ id: TerminalID, keeping kept: String? = nil) {
        for index in groups.indices { groups[index].members.removeAll { $0 == id } }
        groups.removeAll { !$0.isPinned && $0.members.isEmpty && $0.id != kept }
    }
}
