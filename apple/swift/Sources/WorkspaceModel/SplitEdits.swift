// Splits as state changes, after core's `view/sidebar/split.rs` and `arrange.rs` (`seat_beside`): a
// terminal that joins a split is seated beside the split's other members, in their group, or out of any
// group when they are in none, so a split always stands in one card.

import Foundation

extension WorkspaceState {
    /// `id` and the rest of its split, in the order the list shows them (just `id` outside a split).
    func splitMates(of id: TerminalID) -> [TerminalID] {
        guard let split = terminals.first(where: { $0.id == id })?.splitID else { return [id] }
        let mates = SidebarListing.order(of: self).filter { listed in
            terminals.first { $0.id == listed }?.splitID == split
        }
        return mates.count > 1 ? mates : [id]
    }

    /// The split `id` and `other` share: `other`'s, or a new one when it has none. `id` leaves its own.
    mutating func join(_ id: TerminalID, with other: TerminalID) {
        guard id != other, let at = terminals.firstIndex(where: { $0.id == id }),
            let to = terminals.firstIndex(where: { $0.id == other })
        else { return }
        let split = terminals[to].splitID ?? (terminals.compactMap(\.splitID).max() ?? 0) + 1
        guard terminals[at].splitID != split else { return }
        leaveSplit(id)
        terminals[to].splitID = split
        if let moved = terminals.firstIndex(where: { $0.id == id }) { terminals[moved].splitID = split }
        seat(id, beside: splitMates(of: other))
    }

    /// Out of the split. A split left with one terminal is none.
    mutating func leaveSplit(_ id: TerminalID) {
        guard let at = terminals.firstIndex(where: { $0.id == id }), let split = terminals[at].splitID else { return }
        terminals[at].splitID = nil
        let rest = terminals.indices.filter { terminals[$0].splitID == split }
        if rest.count == 1 { terminals[rest[0]].splitID = nil }
    }

    /// A new terminal beside this one, in its split, group and folder; it takes the selection.
    mutating func splitRight(_ anchor: TerminalID) {
        guard let at = terminals.firstIndex(where: { $0.id == anchor }) else { return }
        let split = terminals[at].splitID ?? (terminals.compactMap(\.splitID).max() ?? 0) + 1
        terminals[at].splitID = split
        let id = addTerminal(folder: terminals[at].folder, splitID: split)
        if let last = terminals.indices.last { terminals[last].isPinned = terminals[at].isPinned }
        seat(id, beside: splitMates(of: anchor))
    }

    /// Put `joiner` next to `mates` (the split's members as listed): into their group right after the last
    /// of them, or, when they are loose, out of its own group and right after them.
    private mutating func seat(_ joiner: TerminalID, beside mates: [TerminalID]) {
        guard let anchor = mates.last(where: { $0 != joiner }) else { return }
        if let owner = group(of: anchor), let index = groups.firstIndex(where: { $0.id == owner }) {
            let members = groups[index].members.filter { $0 != joiner }
            let next = members.lastIndex { mates.contains($0) }.map { $0 + 1 }
            move(joiner, to: .group(owner, before: next.flatMap { members.indices.contains($0) ? members[$0] : nil }))
        } else if group(of: joiner) != nil {
            let items = arranged()
            let after = items.firstIndex(of: .terminal(anchor)).flatMap {
                items.indices.contains($0 + 1) ? items[$0 + 1] : nil
            }
            move(joiner, to: .top(before: after))
        }
    }
}
