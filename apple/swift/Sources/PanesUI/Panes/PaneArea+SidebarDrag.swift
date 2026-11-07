// A terminal dragged out of the sidebar's list over the panes (`TerminalDragPointer`): the pointer is the list's own
// gesture, not a drag session, so the area follows it by hand: the same zones and tint as a pane header's drag, the row's
// chip at the pointer, and a join (or a move, for a terminal already in the split) when the button goes up on an edge.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

extension PaneArea {
    /// How far the chip stands from the pointer, so the pointer stays visible beside it.
    private static var chipOffset: CGFloat { Space._3 }

    /// Where the pointer is on the panes: aim at the pane and edge under it, or at none when it is not over them.
    func follow(_ over: TerminalDragPointer.Over?, size: CGSize, origin: CGPoint) {
        guard let over else {
            if drop.dragged?.origin == .sidebarRow { drop = PaneDropState() }
            return
        }
        drop.dragged = DraggedTerminal(id: over.terminal, origin: .sidebarRow)
        drop.aim(at: local(over.point, origin), in: panes, size: size)
    }

    /// The button went up outside the list: what is under the pointer takes the terminal, if the tree accepts it.
    func land(_ released: TerminalDragPointer.Over, size: CGSize, origin: CGPoint) {
        var landing = PaneDropState(dragged: DraggedTerminal(id: released.terminal, origin: .sidebarRow))
        landing.aim(at: local(released.point, origin), in: panes, size: size)
        if let dragged = landing.perform(on: &panes, size: size) { actions.dropped(dragged) }
        drop = PaneDropState()
        sidebarDrag?.settle()
    }

    /// The terminal's chip at the pointer, while the pointer is over this area.
    @ViewBuilder func ghost(_ over: TerminalDragPointer.Over, origin: CGPoint) -> some View {
        let at = local(over.point, origin)
        DragChip(info: info(over.terminal)).fixedSize()
            .offset(x: at.x + Self.chipOffset, y: at.y + Self.chipOffset)
            .allowsHitTesting(false)
            .transition(.opacity)
    }

    private func local(_ point: CGPoint, _ origin: CGPoint) -> CGPoint { CGPoint(x: point.x - origin.x, y: point.y - origin.y) }
}
