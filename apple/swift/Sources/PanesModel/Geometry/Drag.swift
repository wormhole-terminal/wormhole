// Dragging a divider and setting a pane's share: only the two children involved change size, and the
// position is held where either of them (or any pane inside them) would drop under its minimum.

import Foundation

extension Node {
    /// The split at `path` with its rectangle, `rect` being the root's.
    fileprivate mutating func editSplit(at path: ArraySlice<Int>, in rect: PaneRect, _ body: (inout [Child], SplitAxis, PaneRect) -> Double?)
        -> Double?
    {
        guard case .split(let axis, var children) = self else { return nil }
        guard let first = path.first else {
            let result = body(&children, axis, rect)
            self = .split(axis, children)
            return result
        }
        guard let inner = Layout.childRects(rect, axis, children)[safe: first] else { return nil }
        let result = children[first].node.editSplit(at: path.dropFirst(), in: inner, body)
        self = .split(axis, children)
        return result
    }

    /// The least room a subtree needs along `axis` for every pane in it to keep its minimum: panes in a
    /// split of that axis add up, panes across it need the widest one's.
    func minExtent(_ axis: SplitAxis, _ minW: Double, _ minH: Double) -> Double {
        switch self {
        case .leaf: return axis == .row ? minW : minH
        case .split(let own, let children):
            let extents = children.map { $0.node.minExtent(axis, minW, minH) }
            return own == axis ? extents.reduce(0, +) : (extents.max() ?? 0)
        }
    }

    /// Where pane `id` stands directly in a split: its index among that split's children, with `path` left
    /// as the child indexes from the root down to the split.
    func parentIndex(of id: PaneID, _ path: inout [Int]) -> Int? {
        guard case .split(_, let children) = self else { return nil }
        if let at = children.firstIndex(where: { $0.node == .leaf(id) }) { return at }
        for (ix, child) in children.enumerated() {
            path.append(ix)
            if let found = child.node.parentIndex(of: id, &path) { return found }
            path.removeLast()
        }
        return nil
    }
}

extension Array {
    subscript(safe ix: Int) -> Element? { indices.contains(ix) ? self[ix] : nil }
}

extension PaneTree {
    /// Drag a divider to `position` (x for a row's, y for a column's). Returns where it landed, or nil when
    /// it isn't there or can't move (the tab is too small for both sides' minimums: ratios stay).
    @discardableResult
    public mutating func drag(
        _ divider: DividerID, to position: Double, width: Double, height: Double,
        minWidth: Double = PaneMin.width, minHeight: Double = PaneMin.height
    ) -> Double? {
        var edit = root
        let landed = edit.editSplit(at: divider.path[...], in: PaneRect(0, 0, width, height)) { children, axis, rect in
            let k = divider.index
            let (start, extent) = rect.along(axis)
            guard k + 1 < children.count, extent > 0 else { return nil }
            let before = children[..<k].reduce(0) { $0 + $1.ratio }
            let pair = children[k].ratio + children[k + 1].ratio
            let firstStart = start + extent * before
            let low = firstStart + children[k].node.minExtent(axis, minWidth, minHeight)
            let high = firstStart + extent * pair - children[k + 1].node.minExtent(axis, minWidth, minHeight)
            guard low <= high else { return nil }
            let held = min(max(position, low), high)
            children[k].ratio = (held - firstStart) / extent
            children[k + 1].ratio = pair - children[k].ratio
            return held
        }
        if landed != nil { root = edit }
        return landed
    }

    /// Equal shares for the children of the split this divider belongs to (a double-click on it).
    @discardableResult
    public mutating func equalize(_ divider: DividerID) -> Bool {
        var edit = root
        let done = edit.editSplit(at: divider.path[...], in: PaneRect(0, 0, 0, 0)) { children, _, _ in
            guard divider.index + 1 < children.count else { return nil }
            Node.equalShares(&children)
            return 1
        }
        if done != nil { root = edit }
        return done != nil
    }

    /// Give `pane` the share `share` (0...1) of the split it stands in, taking from or giving to its
    /// neighbour `beside`; the pair keeps what it had between them. Held where either would drop under its
    /// minimum; with an unknown tab size (0) it only keeps both from vanishing. Returns the share it ended
    /// with, or nil, nothing changed, when they are not neighbours or the tab is too small for both.
    @discardableResult
    public mutating func setShare(
        of pane: PaneID, beside: PaneID, to share: Double, width: Double, height: Double,
        minWidth: Double = PaneMin.width, minHeight: Double = PaneMin.height
    ) -> Double? {
        var path: [Int] = []
        guard let at = root.parentIndex(of: pane, &path) else { return nil }
        var edit = root
        let result = edit.editSplit(at: path[...], in: PaneRect(0, 0, width, height)) { children, axis, rect in
            guard let other = children.firstIndex(where: { $0.node == .leaf(beside) }), abs(at - other) == 1 else {
                return nil
            }
            let extent = rect.along(axis).extent
            let pair = children[at].ratio + children[other].ratio
            let (low, high) =
                extent > 0
                ? (children[at].node.minExtent(axis, minWidth, minHeight) / extent,
                   pair - children[other].node.minExtent(axis, minWidth, minHeight) / extent)
                : (0.01, pair - 0.01)
            guard low <= high else { return nil }
            let held = min(max(share, low), high)
            children[at].ratio = held
            children[other].ratio = pair - held
            return held
        }
        if result != nil { root = edit }
        return result
    }
}
