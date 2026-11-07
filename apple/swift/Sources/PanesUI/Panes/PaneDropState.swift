// A terminal on its way over the panes: which pane and edge the pointer is on, and what would become of a
// drop there (`window/panes/drop.rs`).

import PanesModel

struct PaneDropState: Equatable {
    var dragged: DraggedTerminal?
    var target: PaneID?
    var zone: Zone?

    var isHovering: Bool { dragged != nil && target != nil }
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
