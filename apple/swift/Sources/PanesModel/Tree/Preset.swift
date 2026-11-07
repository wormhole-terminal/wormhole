// View → Layout: the tab arranged in one step as 2 columns, 2 rows, a 2×2 grid or a single pane. The panes
// fill the slots in reading order (the focused one always kept), new panes fill what is left, and panes
// over the count leave for terminals of their own rather than being closed, since they are running.

import Foundation

public enum Preset: Sendable, CaseIterable, Equatable {
    case twoColumns, twoRows, grid, single

    public var slots: Int {
        switch self {
        case .single: 1
        case .twoColumns, .twoRows: 2
        case .grid: 4
        }
    }

    /// The menu item's title.
    public var title: String {
        switch self {
        case .single: "Single Pane"
        case .twoColumns: "2 Columns"
        case .twoRows: "2 Rows"
        case .grid: "2×2 Grid"
        }
    }

    /// The tree of `ids` (exactly `slots` of them, in reading order) in this layout, with equal shares.
    func shape(_ ids: [PaneID]) -> Node {
        func split(_ axis: SplitAxis, _ nodes: [Node]) -> Node {
            .split(axis, nodes.map { Child(1 / Double(nodes.count), $0) })
        }
        switch self {
        case .single: return .leaf(ids[0])
        case .twoColumns: return split(.row, [.leaf(ids[0]), .leaf(ids[1])])
        case .twoRows: return split(.column, [.leaf(ids[0]), .leaf(ids[1])])
        // Reading order is top left, top right, bottom left, bottom right.
        case .grid:
            return split(.row, [split(.column, [.leaf(ids[0]), .leaf(ids[2])]), split(.column, [.leaf(ids[1]), .leaf(ids[3])])])
        }
    }
}

/// What applying a preset takes: which panes fill the slots, how many new panes to open, which leave.
public struct PresetPlan: Sendable, Equatable {
    public var keep: [PaneID]
    public var new: Int
    public var out: [PaneID]

    public init(keep: [PaneID], new: Int, out: [PaneID]) { (self.keep, self.new, self.out) = (keep, new, out) }
}

extension PaneTree {
    public func plan(_ preset: Preset, focused: PaneID) -> PresetPlan {
        let order = panes
        var keep = Array(order.prefix(preset.slots))
        var out = Array(order.dropFirst(preset.slots))
        // The focused pane is the one the person is in: it stays whatever its place.
        if let at = out.firstIndex(of: focused), let last = keep.last {
            out[at] = last
            keep[keep.count - 1] = focused
            out.sort { order.firstIndex(of: $0)! < order.firstIndex(of: $1)! }
        }
        return PresetPlan(keep: keep, new: preset.slots - keep.count, out: out)
    }

    /// This tree as `preset`, holding `ids` (exactly its slots, in reading order) with equal shares.
    @discardableResult
    mutating func arrange(_ preset: Preset, _ ids: [PaneID]) -> Bool {
        guard ids.count == preset.slots else { return false }
        root = preset.shape(ids)
        return true
    }

    /// The preset this tree has the shape of, whatever its shares, for the menu's check mark.
    public var preset: Preset? {
        func leaves(_ children: [Child]) -> Bool { children.allSatisfy { if case .leaf = $0.node { true } else { false } } }
        func pairOfPairs(_ children: [Child], inner: SplitAxis) -> Bool {
            children.count == 2
                && children.allSatisfy {
                    if case .split(inner, let kids) = $0.node { kids.count == 2 && leaves(kids) } else { false }
                }
        }
        switch root {
        case .leaf: return .single
        case .split(.row, let c) where c.count == 2 && leaves(c): return .twoColumns
        case .split(.column, let c) where c.count == 2 && leaves(c): return .twoRows
        case .split(.row, let c) where pairOfPairs(c, inner: .column): return .grid
        case .split(.column, let c) where pairOfPairs(c, inner: .row): return .grid
        case .split: return nil
        }
    }
}
