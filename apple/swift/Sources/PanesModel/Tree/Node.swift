// A split's panes as a tree, after `wormhole-core` `view/panes`: one pane, or a split along an axis whose
// children each take a share of its extent. Pure values; the window draws them and owns the terminals.

import Foundation

/// A pane's id: the id of its terminal's session, handed out by the window.
public typealias PaneID = Int

/// How a split lays its children out: `row` side by side (⌘D adds to it), `column` stacked (⇧⌘D).
public enum SplitAxis: Sendable, Equatable, Hashable {
    case row, column
}

/// Where ⌥⌘← → ↑ ↓ moves the focus.
public enum Direction: Sendable, Equatable, Hashable, CaseIterable {
    case left, right, up, down
}

/// A split's child with its share of the split's extent (siblings' shares sum to 1).
public struct Child: Sendable, Equatable {
    public var ratio: Double
    public var node: Node

    public init(_ ratio: Double, _ node: Node) {
        self.ratio = ratio
        self.node = node
    }
}

public enum Node: Sendable, Equatable {
    case leaf(PaneID)
    /// Never fewer than two children inside a `PaneTree`, and never a child split of its own axis.
    case split(SplitAxis, [Child])

    var count: Int {
        switch self {
        case .leaf: 1
        case .split(_, let children): children.reduce(0) { $0 + $1.node.count }
        }
    }

    func contains(_ id: PaneID) -> Bool {
        switch self {
        case .leaf(let own): own == id
        case .split(_, let children): children.contains { $0.node.contains(id) }
        }
    }

    /// Whether two nodes arrange the same panes the same way, shares aside.
    func sameShape(as other: Node) -> Bool {
        switch (self, other) {
        case (.leaf(let a), .leaf(let b)): return a == b
        case (.split(let a, let ac), .split(let b, let bc)):
            return a == b && ac.count == bc.count && zip(ac, bc).allSatisfy { $0.node.sameShape(as: $1.node) }
        default: return false
        }
    }
}
