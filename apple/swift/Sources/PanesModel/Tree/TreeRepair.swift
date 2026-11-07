// Keeping a tree's invariants after a pane closes and when a saved record is loaded: repairing beats
// refusing, because the panes are still running and nobody should lose an arrangement over a number.

import Foundation

extension Node {
    /// The node with every invariant restored, or nil when no pane is left in it.
    static func repaired(_ node: Node) -> Node? {
        var seen = Set<PaneID>()
        guard var root = repair(node, &seen) else { return nil }
        root.tidy()
        return root
    }

    /// Bottom-up: drops later duplicates of an id and splits left without children.
    private static func repair(_ node: Node, _ seen: inout Set<PaneID>) -> Node? {
        switch node {
        case .leaf(let id):
            return seen.insert(id).inserted ? node : nil
        case .split(let axis, let children):
            var kept: [Child] = []
            for child in children {
                if let inner = repair(child.node, &seen) { kept.append(Child(child.ratio, inner)) }
            }
            if kept.isEmpty { return nil }
            var split = Node.split(axis, kept)
            split.tidy()
            return split
        }
    }

    /// A split whose children just changed, made canonical again: a child split left with one child is
    /// replaced by it, a child split of the same axis is merged in (scaled into the child's share), the
    /// shares sum to 1 again, and the split itself gives way to a lone child.
    mutating func tidy() {
        guard case .split(let axis, var children) = self else { return }
        for ix in children.indices { children[ix].node.liftLoneChild() }
        var merged: [Child] = []
        for child in children {
            if case .split(let inner, let grand) = child.node, inner == axis {
                merged += grand.map { Child($0.ratio * child.ratio, $0.node) }
            } else {
                merged.append(child)
            }
        }
        Self.normalise(&merged)
        self = .split(axis, merged)
        liftLoneChild()
    }

    mutating func liftLoneChild() {
        if case .split(_, let children) = self, children.count == 1 { self = children[0].node }
    }

    /// Shares summing to 1 in their proportions; one that is not a positive finite number spoils them all
    /// (a zero-size pane is as useless as a NaN one) and the split falls back to equal shares.
    static func normalise(_ children: inout [Child]) {
        let sum = children.reduce(0) { $0 + $1.ratio }
        if children.allSatisfy({ isShare($0.ratio) }) && isShare(sum) {
            for ix in children.indices { children[ix].ratio /= sum }
        } else {
            equalShares(&children)
        }
    }

    static func equalShares(_ children: inout [Child]) {
        let share = 1 / Double(max(children.count, 1))
        for ix in children.indices { children[ix].ratio = share }
    }

    private static func isShare(_ ratio: Double) -> Bool { ratio.isFinite && ratio > 0 }
}
