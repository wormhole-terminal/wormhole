// Where a drag lands on the panes of a split: which pane and which of its four edges (`panes/drop.rs`), as geometry for
// the overlay and the pointer's aim. What a drop does is the core's. A terminal dragged from the sidebar onto an edge joins
// the split on that side; a pane's header dragged onto another pane's edge moves it there.

import Foundation
import WorkspaceModel

public enum DropBand {
    /// A quarter of a pane's side, so the middle half is left for a drop that is not an edge.
    public static let share = 0.25
    /// The widest a band gets: a quarter of a wide pane is a slab of screen that would be all a pointer hits.
    public static let maxDepth = 96.0

    public static func depth(_ side: Double) -> Double { min(side * share, maxDepth) }

    /// The band along `zone`'s edge of `rect`, what the view tints while a drag is over it.
    public static func rect(_ rect: PaneRect, _ zone: Zone) -> PaneRect {
        let (bandW, bandH) = (depth(rect.w), depth(rect.h))
        switch zone {
        case .left: return PaneRect(rect.x, rect.y, bandW, rect.h)
        case .right: return PaneRect(rect.right - bandW, rect.y, bandW, rect.h)
        case .top: return PaneRect(rect.x, rect.y, rect.w, bandH)
        case .bottom: return PaneRect(rect.x, rect.bottom - bandH, rect.w, bandH)
        }
    }

    /// The half of the pane a terminal dropped on `zone` would take, tinted under the pointer.
    public static func preview(_ rect: PaneRect, _ zone: Zone) -> PaneRect {
        let (halfW, halfH) = (rect.w / 2, rect.h / 2)
        switch zone {
        case .left: return PaneRect(rect.x, rect.y, halfW, rect.h)
        case .right: return PaneRect(rect.x + halfW, rect.y, halfW, rect.h)
        case .top: return PaneRect(rect.x, rect.y, rect.w, halfH)
        case .bottom: return PaneRect(rect.x, rect.y + halfH, rect.w, halfH)
        }
    }

    /// What shows of `zone`'s band while the pointer is on `active`: the band without the corners the top
    /// and bottom bands take and without the part the lit half covers. Nil when nothing is left.
    public static func hint(_ rect: PaneRect, _ zone: Zone, active: Zone) -> PaneRect? {
        let band = Self.rect(rect, zone)
        let corner = depth(rect.h)
        var (left, right, up, down) =
            zone == .left || zone == .right
            ? (band.x, band.right, band.y + corner, band.bottom - corner) : (band.x, band.right, band.y, band.bottom)
        let lit = preview(rect, active)
        switch active {
        case .left: left = max(left, lit.right)
        case .right: right = min(right, lit.x)
        case .top: up = max(up, lit.bottom)
        case .bottom: down = min(down, lit.y)
        }
        return right > left && down > up ? PaneRect(left, up, right - left, down - up) : nil
    }

    /// The edge of `rect` a drop at `point` lands on, nil outside. Every point inside resolves to one edge:
    /// the nearest measured in bands (so a wide pane isn't biased towards its top and bottom). A tie goes
    /// right, then below, as ⌘D and ⇧⌘D split. Left and top edges belong to the pane, right and bottom to its
    /// neighbour, so no point is in two panes.
    public static func zone(in rect: PaneRect, at point: (x: Double, y: Double)) -> Zone? {
        let inside = rect.w > 0 && rect.h > 0 && point.x >= rect.x && point.x < rect.right && point.y >= rect.y
        guard inside, point.y < rect.bottom else { return nil }
        let (bandW, bandH) = (depth(rect.w), depth(rect.h))
        // `min` keeps the first of equals, so this order is the tie-break.
        let depths: [(Zone, Double)] = [
            (.right, (rect.right - point.x) / bandW), (.bottom, (rect.bottom - point.y) / bandH),
            (.left, (point.x - rect.x) / bandW), (.top, (point.y - rect.y) / bandH),
        ]
        return depths.reduce(depths[0]) { $1.1 < $0.1 ? $1 : $0 }.0
    }

    /// The pane under `point` and the edge of it the drop lands on.
    public static func target(
        _ rects: [(id: PaneID, rect: PaneRect)], at point: (x: Double, y: Double)
    ) -> (pane: PaneID, zone: Zone)? {
        for (id, rect) in rects {
            if let zone = zone(in: rect, at: point) { return (id, zone) }
        }
        return nil
    }
}
