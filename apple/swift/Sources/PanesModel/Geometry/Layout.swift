// Where the panes and dividers of a tab are, for a tab of a given size.

import Foundation

enum Layout {
    /// Each child's end is the next one's start, computed once, so neighbours' edges compare equal. `equal`
    /// lays them out as if every share were 1/n.
    static func childRects(_ rect: PaneRect, _ axis: Axis, _ children: [Child], equal: Bool = false) -> [PaneRect] {
        let (start, extent) = rect.along(axis)
        var edges = [start]
        var cumulative = 0.0
        for (ix, child) in children.enumerated() {
            cumulative += equal ? 1 / Double(children.count) : child.ratio
            // The last edge is the split's own, whatever rounding left of the sum.
            edges.append(ix + 1 == children.count ? start + extent : start + extent * cumulative)
        }
        return zip(edges, edges.dropFirst()).map { from, to in
            axis == .row ? PaneRect(from, rect.y, to - from, rect.h) : PaneRect(rect.x, from, rect.w, to - from)
        }
    }

    static func layout(_ node: Node, _ rect: PaneRect, equal: Bool, _ out: inout [(PaneID, PaneRect)]) {
        switch node {
        case .leaf(let id): out.append((id, rect))
        case .split(let axis, let children):
            for (child, inner) in zip(children, childRects(rect, axis, children, equal: equal)) {
                layout(child.node, inner, equal: equal, &out)
            }
        }
    }

    /// Equal shares on a unit square, sorted by top then left: the order depends on the shape alone.
    static func readingOrder(_ root: Node) -> [PaneID] {
        var rects: [(PaneID, PaneRect)] = []
        layout(root, PaneRect(0, 0, 1, 1), equal: true, &rects)
        // Shares of 1/n reached along different paths can differ by an ulp; that is one row.
        rects.sort { a, b in abs(a.1.y - b.1.y) > 1e-4 ? a.1.y < b.1.y : a.1.x < b.1.x }
        return rects.map(\.0)
    }

    static func dividers(_ node: Node, _ rect: PaneRect, _ path: inout [Int], _ out: inout [Divider]) {
        guard case .split(let axis, let children) = node else { return }
        let rects = childRects(rect, axis, children)
        for (index, next) in rects.enumerated().dropFirst() {
            let line = axis == .row ? PaneRect(next.x, rect.y, 0, rect.h) : PaneRect(rect.x, next.y, rect.w, 0)
            out.append(Divider(id: DividerID(path: path, index: index - 1), axis: axis, rect: line))
        }
        for (ix, (child, inner)) in zip(children, rects).enumerated() {
            path.append(ix)
            dividers(child.node, inner, &path, &out)
            path.removeLast()
        }
    }
}

extension PaneTree {
    /// Every pane's rectangle in a tab `width` × `height`, in reading order.
    public func rects(width: Double, height: Double) -> [(id: PaneID, rect: PaneRect)] {
        var rects: [(PaneID, PaneRect)] = []
        Layout.layout(root, PaneRect(0, 0, width, height), equal: false, &rects)
        let order = panes
        return rects.sorted { order.firstIndex(of: $0.0)! < order.firstIndex(of: $1.0)! }.map { (id: $0.0, rect: $0.1) }
    }

    /// Every divider, splits before their children.
    public func dividers(width: Double, height: Double) -> [Divider] {
        var out: [Divider] = []
        var path: [Int] = []
        Layout.dividers(root, PaneRect(0, 0, width, height), &path, &out)
        return out
    }
}
