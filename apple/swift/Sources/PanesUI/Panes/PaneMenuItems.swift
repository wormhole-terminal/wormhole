// A pane's menu (its ⋯ button and a right-click on its header) as native menu items: what a pane cannot do is greyed out, and
// each entry shows its key (`window/panes/mod.rs`).

import SwiftUI
import WorkspaceModel

struct PaneMenuItems: View {
    let id: PaneID
    let panes: PanesState
    let actions: PaneActions

    var body: some View {
        Button(PanesText.splitRight) { act { actions.split(.row) } }
            .keyboardShortcut("d", modifiers: .command)
            .disabled(!actions.canSplit(.row))
        Button(PanesText.splitDown) { act { actions.split(.column) } }
            .keyboardShortcut("d", modifiers: [.command, .shift])
            .disabled(!actions.canSplit(.column))
        Button(panes.zoomed == id ? PanesText.unzoom : PanesText.zoom) { act(actions.toggleZoom) }
            .keyboardShortcut(.return, modifiers: [.command, .shift])
            .disabled(!panes.isSplit)
        Button(PanesText.leaveSplit) { act { actions.leaveSplit(id) } }
            .disabled(!panes.isSplit)
        Divider()
        Button(PanesText.closePane) { act { actions.close(id) } }
            .keyboardShortcut("w", modifiers: .command)
    }

    /// The pane the menu belongs to takes the keyboard first, as it does when it is clicked.
    private func act(_ run: () -> Void) {
        actions.focus(id)
        run()
    }
}
