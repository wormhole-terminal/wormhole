// Where dropping a group puts it (`group_drop` in `place.rs`): always at the top level. Over another group the pointer in
// the upper half of its head stands before it and the lower half after; over one of its terminals, the upper half of its
// rows before it and the lower half after; over a terminal in no group, its top quarter before it and the rest after.

import CoreGraphics
import WorkspaceModel

extension DropMap {
    func plan(dropping group: String, at point: CGPoint, gap: CGFloat) -> DropPlan? {
        guard let (line, fraction) = line(at: point.y) else { return nil }
        let own = TopItem.group(group)
        let item = item(of: line)
        guard item != own else { return nil }
        let card = span(of: item, line)
        let after: Bool
        if point.y > line.frame.maxY {
            after = true
        } else {
            switch line.kind {
            case .head: after = fraction >= 0.5
            case .row(let id, let inGroup?):
                let rows = members[inGroup] ?? []
                let place = rows.firstIndex(of: id) ?? 0
                after = place * 2 >= rows.count && DropPart(fraction) != .top || place * 2 > rows.count
            case .row: after = DropPart(fraction) != .top
            }
        }
        let before = after ? next(after: item, skipping: own) : item
        // Dropped where it already is: before its own next neighbour.
        guard before != next(after: own) else { return nil }
        return DropPlan(
            action: .moveGroup(group, before: before),
            mark: .line(y: after ? card.maxY + gap / 2 : card.minY - gap / 2, x: card.minX...card.maxX))
    }
}
