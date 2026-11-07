// Dragging a column's edge: one rule decides every drag (`ColumnResize`, ported from core). The width follows
// the pointer; dragging past the minimum shows what letting go does and closes the column only then, so the
// handle under the pointer is still there to drag back out with.

import SwiftUI
import WorkspaceModel

extension WindowBody {
    func handle(_ edge: PaneLayout.Edge, at x: CGFloat, window: CGFloat) -> some View {
        let isSidebar = edge == .sidebar
        return ColumnHandle(
            label: isSidebar ? "Resize sidebar" : "Resize panel", width: isSidebar ? widths.sidebarWidth : widths.panelWidth,
            drag: { move(edge, pointer: $0, window: window) },
            reset: { set(edge, width: edge.defaultWidth) },
            step: { delta in set(edge, width: (isSidebar ? widths.sidebarWidth : widths.panelWidth) + (isSidebar ? delta : -delta)) }
        )
        .offset(x: x - Chrome.resizeHandle / 2)
    }

    private func set(_ edge: PaneLayout.Edge, width: CGFloat) {
        let clamped = min(max(width, edge.range.lowerBound), edge.range.upperBound)
        if edge == .sidebar { widths.sidebarWidth = clamped } else { widths.panelWidth = clamped }
    }

    private func move(_ edge: PaneLayout.Edge, pointer: CGFloat?, window: CGFloat) {
        guard let pointer else { return finish() }
        var layout = widths
        layout.sidebarOpen = sidebarWanted
        layout.panelWanted = state.panelVisible
        let extent = edge == .sidebar ? pointer : window - pointer
        let resize = ColumnResize(extent: extent, range: edge.range, max: layout.maxWidth(of: edge, window: window))
        set(edge, width: resize.width)
        drag = WorkspaceRootView.DragState(edge: edge, pointer: pointer, resize: resize)
    }

    private func finish() {
        guard let ended = drag else { return }
        drag = nil
        if !ended.resize.open { send(ended.edge == .sidebar ? .toggleSidebar : .togglePanel) }
    }
}
