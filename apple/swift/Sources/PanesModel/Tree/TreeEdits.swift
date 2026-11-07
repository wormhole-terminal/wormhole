// The edits of `Node`: splitting a pane, closing one, equalizing. `PaneTree` calls them from its root.

import Foundation

extension Node {
    /// Splits the pane `target` along `axis`, the new pane after it, or before it when `before`. Along the
    /// parent's own axis the new pane is a sibling, not a nested split, and takes half of the target's share
    /// so the other siblings keep theirs.
    mutating func splitting(_ target: PaneID, axis: SplitAxis, before: Bool, new id: PaneID) -> Bool {
        switch self {
        case .leaf(let own) where own == target:
            let (first, second) = before ? (id, target) : (target, id)
            self = .split(axis, [Child(0.5, .leaf(first)), Child(0.5, .leaf(second))])
            return true
        case .leaf:
            return false
        case .split(let own, var children):
            if own == axis, let at = children.firstIndex(where: { $0.node == .leaf(target) }) {
                let half = children[at].ratio / 2
                children[at].ratio = half
                children.insert(Child(half, .leaf(id)), at: before ? at : at + 1)
                self = .split(own, children)
                return true
            }
            for ix in children.indices where children[ix].node.splitting(target, axis: axis, before: before, new: id) {
                self = .split(own, children)
                return true
            }
            return false
        }
    }

    /// Removes the pane from this split or one below it, then tidies the split so each level sees
    /// canonical children.
    mutating func removing(_ id: PaneID) -> Bool {
        guard case .split(let axis, var children) = self else { return false }
        var removed = false
        if let at = children.firstIndex(where: { $0.node == .leaf(id) }) {
            children.remove(at: at)
            removed = true
        } else {
            for ix in children.indices where children[ix].node.removing(id) {
                removed = true
                break
            }
        }
        if removed {
            self = .split(axis, children)
            tidy()
        }
        return removed
    }

    mutating func equalizing() {
        guard case .split(let axis, var children) = self else { return }
        Self.equalShares(&children)
        for ix in children.indices { children[ix].node.equalizing() }
        self = .split(axis, children)
    }
}
