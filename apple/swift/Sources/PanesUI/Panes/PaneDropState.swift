// A terminal on its way over the panes: which pane and edge the pointer is on, and what would become of a
// drop there (`window/panes/drop.rs`).

import CoreGraphics
import PanesModel

struct PaneDropState: Equatable {
    var dragged: DraggedTerminal?
    var target: PaneID?
    var zone: Zone?

    var isHovering: Bool { dragged != nil && target != nil }

    /// The pane and edge under `point`, in the area's coordinates (neither when it is over no pane).
    mutating func aim(at point: CGPoint, in panes: TabPanes, size: CGSize) {
        let hit = DropBand.target(panes.shown(width: size.width, height: size.height), at: (point.x, point.y))
        (target, zone) = (hit?.pane, hit?.zone)
    }

    /// Let go on what the pointer is over: the terminal that was dragged when the tree took it, else nil.
    func perform(on panes: inout TabPanes, size: CGSize) -> DraggedTerminal? {
        guard let dragged, let target, let zone else { return nil }
        return panes.drop(dragged, on: target, zone, room: PaneRoom(size.width, size.height)) == nil ? dragged : nil
    }
}

extension TabPanes {
    /// What a drop of `dragged` on `target`'s `zone` would do to these panes: a terminal already in the split
    /// is moved, any other joins it. Nil when it works.
    func dropRefusal(_ dragged: DraggedTerminal, on target: PaneID, _ zone: Zone, room: PaneRoom) -> DropError? {
        contains(dragged.id)
            ? moveRefusal(dragged.id, beside: target, zone: zone, room: room)
            : joinRefusal(dragged.id, beside: target, zone: zone, room: room)
    }

    mutating func drop(_ dragged: DraggedTerminal, on target: PaneID, _ zone: Zone, room: PaneRoom) -> DropError? {
        do throws(DropError) {
            if contains(dragged.id) {
                try move(dragged.id, beside: target, zone: zone, room: room)
            } else {
                try join(dragged.id, beside: target, zone: zone, room: room)
            }
        } catch { return error }
        return nil
    }
}
