// The panes of the selected terminal's split, each a terminal with its overlays. The pane area is PanesUI's: the
// tree, headers, dividers, drops and zoom; what a pane holds is this window's.

import DesignSystem
import PanesUI
import SwiftUI
import TerminalMetal
import TerminalUI
import WorkspaceModel

struct TerminalPanes: View {
    let hub: WindowHub
    @Environment(WindowLook.self) private var look

    var body: some View {
        let state = hub.state
        PaneArea(
            panes: hub.panesBinding, gaps: look.splitGaps, session: hub.drag, sidebarDrag: hub.terminalDrag, actions: hub.paneActions,
            info: { info($0, in: state) }
        ) { id in
            TerminalSurface(hub: hub, id: id).id(hub.restarts[id, default: 0])
        }
        .onChange(of: state.live, initial: true) { _, ids in
            hub.terminals.retain(only: Set(ids))
            hub.forgetOverlays(keeping: Set(ids))
        }
    }

    private func info(_ id: TerminalID, in state: WorkspaceState) -> PaneInfo {
        let terminal = state.terminals.first { $0.id == id }
        return PaneInfo(title: terminal?.title ?? "", folder: terminal?.folder, activity: terminal?.activity ?? .idle, agent: terminal?.agent)
    }
}

/// One terminal's grid with what floats over it, in the order the GPUI pane stacks them. The notice chip is the
/// window's, drawn once over the whole area.
struct TerminalSurface: View {
    let hub: WindowHub
    let id: TerminalID
    @Environment(WindowLook.self) private var look

    var body: some View {
        if let overlay = hub.overlay(for: id) {
            // The minimap takes the scroll bar's place at the right of the grid, and the bars stay clear of it.
            let minimap = look.minimap && overlay.hasScrollback ? MinimapGeometry.width : 0
            GeometryReader { proxy in
                ZStack(alignment: .topTrailing) {
                    TerminalHost(views: hub.terminals, id: id, overlay: overlay) { hub.actions.terminalMenu(id) }
                    TerminalMarks(overlay: overlay, showsMinimap: look.minimap)
                    if overlay.find.isOpen {
                        FindBar(model: overlay.find, onClose: overlay.find.close).padding(Space._2)
                    }
                    SizeReadoutChip(model: overlay.size)
                    PaneOverlays(
                        state: overlay.bars, shell: hub.shell, exitCode: overlay.exitCode,
                        gridWidth: proxy.size.width - minimap - 2 * MetalRenderer.padding, trailingInset: minimap, actions: overlay.actions)
                }
            }
        } else {
            Color.clear
        }
    }
}
