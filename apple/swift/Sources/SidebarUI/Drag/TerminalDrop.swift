// Where dropping a terminal puts it (`terminal_drop` in `place.rs`): on a row, its top before it and its bottom after it,
// inside the row's group or at the top level; on a group's head, the top quarter before the group and the rest into it
// (under the head when open, at the end when folded); in the gap under a card, after the card; under the list,
// last. Nothing when it would land where it already is.

import CoreGraphics
import WorkspaceModel

extension DropMap {
    func plan(dropping terminal: TerminalID, at point: CGPoint, gap: CGFloat) -> DropPlan? {
        guard let (line, fraction) = line(at: point.y) else { return nil }
        let item = item(of: line)
        let card = span(of: item, line)
        if point.y > line.frame.maxY {
            // The gap under the card, or the space under the list.
            return placed(terminal, before: next(after: item, skipping: .terminal(terminal)), at: card.maxY + gap / 2, over: card)
        }
        switch line.kind {
        case .head(let group):
            if fraction < DropGeometry.headBefore { return placed(terminal, before: item, at: card.minY - gap / 2, over: card) }
            let rest = (members[group] ?? []).filter { $0 != terminal }
            // An open group takes it under the head, a folded or empty one at its end.
            let first = folded.contains(group) ? nil : rest.first
            return DropPlan(action: .moveTerminal(terminal, to: .group(group, before: first)), mark: .card(group))
        case .row(let id, let group):
            guard id != terminal else { return nil }
            let after = fraction >= (DropPart(fraction) == .middle ? 0.5 : 0.75)
            if let group {
                let rest = (members[group] ?? []).filter { $0 != terminal }
                let next = after ? rest.drop { $0 != id }.dropFirst().first : id
                return DropPlan(
                    action: .moveTerminal(terminal, to: .group(group, before: next)),
                    mark: .line(y: after ? line.frame.maxY : line.frame.minY, x: line.frame.minX...line.frame.maxX))
            }
            return after
                ? placed(terminal, before: next(after: item, skipping: .terminal(terminal)), at: card.maxY + gap / 2, over: card)
                : placed(terminal, before: item, at: card.minY - gap / 2, over: card)
        }
    }

    /// The terminal at the top level before `before` (the end without one), unless that is where it is.
    private func placed(_ terminal: TerminalID, before: TopItem?, at y: CGFloat, over card: CGRect) -> DropPlan? {
        let own = TopItem.terminal(terminal)
        guard before != own, !(top.contains(own) && next(after: own) == before) else { return nil }
        return DropPlan(action: .moveTerminal(terminal, to: .top(before: before)), mark: .line(y: y, x: card.minX...card.maxX))
    }
}
