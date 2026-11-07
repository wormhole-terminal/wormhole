// A split's panes. Always holds at least one pane; every id appears once; no split has fewer than two
// children or a child split of its own axis; siblings' shares are positive and sum to 1. Dividers have no
// thickness in this model: the view draws them over the panes' shared edges.

import Foundation

public struct PaneTree: Sendable, Equatable {
    public internal(set) var root: Node

    public static func single(_ id: PaneID) -> PaneTree { PaneTree(root: .leaf(id)) }

    private init(root: Node) { self.root = root }

    /// A tree from a hand-built or loaded root, with the invariants restored; nil when no pane is left.
    public init?(repairing root: Node) {
        guard let root = Node.repaired(root) else { return nil }
        self.root = root
    }

    public var count: Int { root.count }
    public func contains(_ id: PaneID) -> Bool { root.contains(id) }

    /// Split `target` along `axis`, the new pane to its right (row) or below it (column), each taking half
    /// of the target's room. False when `target` isn't a pane here or `new` already is.
    @discardableResult
    public mutating func split(_ target: PaneID, axis: SplitAxis, new id: PaneID) -> Bool {
        !contains(id) && root.splitting(target, axis: axis, before: false, new: id)
    }

    /// Put the new pane `id` on `zone`'s side of `target` (left and top put it before the target).
    @discardableResult
    public mutating func insertBeside(_ target: PaneID, zone: Zone, new id: PaneID) -> Bool {
        !contains(id) && root.splitting(target, axis: zone.axis, before: zone.before, new: id)
    }

    /// Take `id` out of its place and put it on `zone`'s side of `target`. False, with the tree as it was,
    /// when either isn't a pane here, they are the same, or the panes would end up arranged as they are.
    @discardableResult
    public mutating func move(_ id: PaneID, beside target: PaneID, zone: Zone) -> Bool {
        guard id != target, contains(id), contains(target) else { return false }
        var moved = self
        guard moved.remove(id), moved.insertBeside(target, zone: zone, new: id), !root.sameShape(as: moved.root)
        else { return false }
        self = moved
        return true
    }

    /// Close a pane: its share goes to its siblings in proportion. The last pane stays.
    @discardableResult
    public mutating func remove(_ id: PaneID) -> Bool { root.removing(id) }

    /// A pane with no place yet goes at the right end, as a sibling of a row at the root or in a new row
    /// round it, taking an equal share.
    @discardableResult
    public mutating func append(_ id: PaneID) -> Bool {
        guard !contains(id) else { return false }
        if case .split(.row, var children) = root {
            children.append(Child(0, .leaf(id)))
            Node.equalShares(&children)
            root = .split(.row, children)
        } else {
            root = .split(.row, [Child(0.5, root), Child(0.5, .leaf(id))])
        }
        return true
    }

    /// Equal shares in every split.
    public mutating func equalizeAll() { root.equalizing() }

    /// The panes in reading order: left to right, then top to bottom, of the tree with equal shares, so
    /// dragging a divider never reorders them.
    public var panes: [PaneID] { Layout.readingOrder(root) }
}
