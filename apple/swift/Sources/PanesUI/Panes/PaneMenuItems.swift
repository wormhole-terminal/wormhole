// A pane's menu (its ⋯ button and a right-click on its header) and View → Layout's presets, as native menu
// items: what a pane cannot do is greyed out, and each entry shows its key (`window/panes/mod.rs`).

import PanesModel
import SwiftUI

struct PaneMenuItems: View {
    let id: PaneID
    let panes: TabPanes
    let room: PaneRoom
    let actions: PaneActions
    let focus: () -> Void
    let zoom: () -> Void

    var body: some View {
        Button(PanesText.splitRight) { act { actions.split(.row) } }
            .keyboardShortcut("d", modifiers: .command)
            .disabled(!panes.canSplit(.row, room: room))
        Button(PanesText.splitDown) { act { actions.split(.column) } }
            .keyboardShortcut("d", modifiers: [.command, .shift])
            .disabled(!panes.canSplit(.column, room: room))
        Button(panes.zoomed == id ? PanesText.unzoom : PanesText.zoom) { act(zoom) }
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
        focus()
        run()
    }
}

/// View → Layout: the four presets with a check on the one the tab has, each greyed out where the tab
/// would have to close panes it cannot open (`can_layout`: only when more must be opened than room allows).
struct LayoutPresetItems: View {
    let panes: TabPanes
    let choose: (Preset) -> Void

    var body: some View {
        ForEach(Preset.allCases, id: \.self) { preset in
            Toggle(preset.title, isOn: Binding(get: { panes.tree.preset == preset }, set: { _ in choose(preset) }))
        }
    }
}
