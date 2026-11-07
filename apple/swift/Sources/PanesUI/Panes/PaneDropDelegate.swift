// The drop onto the panes: a terminal dragged from the sidebar joins the split on the side of the pane it
// lands on, a pane's header dragged onto another pane's edge moves it there. The terminal travels as text
// (`DraggedTerminal`), read when the drag enters so the hover can say what a drop would do.

import PanesModel
import SwiftUI
import UniformTypeIdentifiers

struct PaneDropDelegate: DropDelegate {
    @Binding var state: PaneDropState
    @Binding var panes: TabPanes
    let size: CGSize
    let onDropped: (DraggedTerminal) -> Void

    private var room: PaneRoom { PaneRoom(size.width, size.height) }

    func validateDrop(info: DropInfo) -> Bool { info.hasItemsConforming(to: [.plainText]) }

    func dropEntered(info: DropInfo) {
        guard let provider = info.itemProviders(for: [.plainText]).first else { return }
        _ = provider.loadObject(ofClass: NSString.self) { text, _ in
            guard let dragged = (text as? String).flatMap(DraggedTerminal.init(encoded:)) else { return }
            Task { @MainActor in state.dragged = dragged }
        }
    }

    func dropUpdated(info: DropInfo) -> DropProposal? {
        let rects = panes.shown(width: size.width, height: size.height)
        let hit = DropBand.target(rects, at: (info.location.x, info.location.y))
        (state.target, state.zone) = (hit?.pane, hit?.zone)
        guard let dragged = state.dragged, let hit else { return DropProposal(operation: .cancel) }
        let refused = panes.dropRefusal(dragged, on: hit.pane, hit.zone, room: room) != nil
        return DropProposal(operation: refused ? .forbidden : .move)
    }

    func dropExited(info: DropInfo) { state = PaneDropState() }

    func performDrop(info: DropInfo) -> Bool {
        defer { state = PaneDropState() }
        guard let dragged = state.dragged, let target = state.target, let zone = state.zone else { return false }
        guard panes.drop(dragged, on: target, zone, room: room) == nil else { return false }
        onDropped(dragged)
        return true
    }
}
