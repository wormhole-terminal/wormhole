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
            .keyboardShortcut(actions.shortcut("splitRight"))
            .disabled(!actions.canSplit(.row))
        Button(PanesText.splitDown) { act { actions.split(.column) } }
            .keyboardShortcut(actions.shortcut("splitDown"))
            .disabled(!actions.canSplit(.column))
        Button(panes.zoomed == id ? PanesText.unzoom : PanesText.zoom) { act(actions.toggleZoom) }
            .keyboardShortcut(actions.shortcut("zoomPane"))
            .disabled(!panes.isSplit)
        Button(PanesText.leaveSplit) { act { actions.leaveSplit(id) } }
            .disabled(!panes.isSplit)
        Divider()
        Button(PanesText.closePane) { act { actions.close(id) } }
            .keyboardShortcut(actions.shortcut("closeTerminal"))
    }

    /// The pane the menu belongs to takes the keyboard first, as it does when it is clicked.
    private func act(_ run: () -> Void) {
        actions.focus(id)
        run()
    }
}
