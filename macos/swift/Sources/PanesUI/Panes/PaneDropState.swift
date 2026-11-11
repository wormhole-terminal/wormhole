// A terminal on its way over the panes: which pane and edge the pointer is on (`window/panes/drop.rs`). What a drop there does
// is the core's to say (`PaneActions.refusal`) and to carry out (`PaneActions.drop`).

import CoreGraphics
import PanesModel
import WorkspaceModel

struct PaneDropState: Equatable {
    var dragged: DraggedTerminal?
    var target: PaneID?
    var zone: Zone?

    var isHovering: Bool { dragged != nil && target != nil }

    /// The pane and edge under `point`, in the area's coordinates (neither when it is over no pane).
    mutating func aim(at point: CGPoint, in panes: PanesState) {
        let hit = DropBand.target(panes.shown.map { (id: $0.id, rect: $0.rect) }, at: (point.x, point.y))
        (target, zone) = (hit?.pane, hit?.zone)
    }

    /// What is under the pointer, once a terminal is held over one.
    var landing: (dragged: DraggedTerminal, target: PaneID, zone: Zone)? {
        guard let dragged, let target, let zone else { return nil }
        return (dragged, target, zone)
    }
}
