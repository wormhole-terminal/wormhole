// How a drop shows (`style::drop_line`, the card's focus outline): a line in the gap between two cards or two rows, a
// group's card outlined while a drop joins it, or a row outlined while a drop makes a split with it. The line is drawn for
// the core's plan, for a terminal (`dropPlan`) and a group (`groupDropPlan`) alike.

import CoreGraphics
import WorkspaceModel

enum DropMark: Equatable {
    /// A line across `x` at `y`.
    case line(y: CGFloat, x: ClosedRange<CGFloat>)
    case card(GroupID)
    case row(TerminalID)
}

/// What letting go does and how it shows.
struct DropAim: Equatable {
    var action: WorkspaceAction
    var mark: DropMark?
    /// The lines that step aside for the drop, with the Motion feature on.
    var room = ListRoom()

    /// What the pointer was over, as the action names it.
    var over: DropOver? {
        switch action {
        case .dropTerminal(_, let over), .dropGroup(_, let over): over
        default: nil
        }
    }
}

extension DropLines {
    func mark(for plan: DropPlan, over: DropOver, gap: CGFloat) -> DropMark? {
        switch plan {
        case .join(let target): return rows[target] == nil ? nil : .row(target)
        case .move(.group(let group, let before), _, _):
            // On a head the drop joins the group (its card is outlined); on a row it lands between rows.
            if case .head = over { return .card(group) }
            if let before, let frame = rows[before] { return line(at: frame.minY, over: frame) }
            return members[group]?.last.flatMap { rows[$0] }.map { line(at: $0.maxY, over: $0) } ?? .card(group)
        case .move(.top(let before), _, _):
            switch before {
            case .terminal(let id)?: return rows[id].map { line(at: $0.minY - gap / 2, over: $0) }
            case .group(let id)?: return cards[id].map { line(at: $0.minY - gap / 2, over: $0) }
            case nil: return lastFrame.map { line(at: $0.maxY + gap / 2, over: $0) }
            }
        }
    }

    private func line(at y: CGFloat, over frame: CGRect) -> DropMark { .line(y: y, x: frame.minX...frame.maxX) }
}
