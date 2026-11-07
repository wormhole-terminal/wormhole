// Which pane ⌥⌘arrows move the focus to, by geometry.

import Foundation

/// Edges closer than this are one edge (ratios rounded on their way through a record).
private let edgeEpsilon = 0.5

extension PaneTree {
    /// Among the panes across the shared edge, the one overlapping it most; at a tie the one whose centre
    /// is nearest; at a dead tie the top or left one. Nil at the tab's edge, so the window decides whether
    /// to wrap.
    public func neighbour(of id: PaneID, _ direction: Direction, width: Double, height: Double) -> PaneID? {
        let all = rects(width: width, height: height)
        guard let from = all.first(where: { $0.id == id })?.rect else { return nil }
        var best: (id: PaneID, overlap: Double, distance: Double)?
        for (pane, rect) in all where pane != id {
            let (touching, overlap, distance): (Bool, Double, Double)
            switch direction {
            case .right: (touching, overlap, distance) = (near(rect.x, from.right), span(rect, from, .row), abs(rect.center.y - from.center.y))
            case .left: (touching, overlap, distance) = (near(rect.right, from.x), span(rect, from, .row), abs(rect.center.y - from.center.y))
            case .down: (touching, overlap, distance) = (near(rect.y, from.bottom), span(rect, from, .column), abs(rect.center.x - from.center.x))
            case .up: (touching, overlap, distance) = (near(rect.bottom, from.y), span(rect, from, .column), abs(rect.center.x - from.center.x))
            }
            // Touching at a corner only is not sharing an edge.
            guard touching, overlap > edgeEpsilon else { continue }
            let better: Bool
            if let best {
                better = abs(overlap - best.overlap) > edgeEpsilon ? overlap > best.overlap : distance < best.distance - edgeEpsilon
            } else {
                better = true
            }
            if better { best = (pane, overlap, distance) }
        }
        return best?.id
    }

    private func near(_ a: Double, _ b: Double) -> Bool { abs(a - b) <= edgeEpsilon }

    /// How much of the two rectangles' sides across `axis`' own direction coincide: the y-sides for a move
    /// along a row, the x-sides for one along a column.
    private func span(_ a: PaneRect, _ b: PaneRect, _ axis: Axis) -> Double {
        let (a0, a1, b0, b1) = axis == .row ? (a.y, a.bottom, b.y, b.bottom) : (a.x, a.right, b.x, b.right)
        return max(min(a1, b1) - max(a0, b0), 0)
    }
}
