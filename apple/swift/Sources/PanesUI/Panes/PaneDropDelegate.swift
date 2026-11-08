// The drop onto the panes: a terminal dragged from the sidebar joins the split on the side of the pane it lands on, a pane's
// header dragged onto another pane's edge moves it there (the core tells which, by whether the terminal is in the split). The
// terminal travels as text (`DraggedTerminal`), read when the drag enters so the hover can say what a drop would do.

import PanesModel
import SwiftUI
import UniformTypeIdentifiers
import WorkspaceModel

struct PaneDropDelegate: DropDelegate {
    @Binding var state: PaneDropState
    let panes: PanesState
    let actions: PaneActions

    func validateDrop(info: DropInfo) -> Bool { info.hasItemsConforming(to: [.plainText]) }

    func dropEntered(info: DropInfo) {
        guard let provider = info.itemProviders(for: [.plainText]).first else { return }
        _ = provider.loadObject(ofClass: NSString.self) { text, _ in
            guard let dragged = (text as? String).flatMap(DraggedTerminal.init(encoded:)) else { return }
            Task { @MainActor in state.dragged = dragged }
        }
    }

    func dropUpdated(info: DropInfo) -> DropProposal? {
        state.aim(at: info.location, in: panes)
        guard let hit = state.landing else { return DropProposal(operation: .cancel) }
        return DropProposal(operation: actions.refusal(hit.dragged, hit.target, hit.zone) == nil ? .move : .forbidden)
    }

    func dropExited(info: DropInfo) { state = PaneDropState() }

    func performDrop(info: DropInfo) -> Bool {
        defer { state = PaneDropState() }
        guard let hit = state.landing, actions.refusal(hit.dragged, hit.target, hit.zone) == nil else { return false }
        actions.drop(hit.dragged, hit.target, hit.zone)
        return true
    }
}
