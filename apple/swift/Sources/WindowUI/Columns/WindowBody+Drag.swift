// Dragging a column's edge. The core decides every drag (`edgeDrag` says what letting go here would leave, `dragEdge` moves
// the column): the width follows the pointer; dragging past the minimum shows "Close" and closes the column only when the
// pointer is let go, so the handle under the pointer is still there to drag back out with. The sample, which has no core
// to ask, resolves the same drag with the local copy of the rule (`ColumnResize`).

import SwiftUI
import WordsModel
import WorkspaceModel

extension WindowBody {
    func handle(_ edge: PaneLayout.Edge, at x: CGFloat, window: CGFloat, fit: ColumnFit) -> some View {
        let isSidebar = edge == .sidebar
        let width = isSidebar ? fit.sidebarWidth : fit.panelWidth
        return ColumnHandle(
            label: isSidebar ? Words.text("window-resize-sidebar", fallback: "Resize sidebar") : Words.text("window-resize-panel", fallback: "Resize panel"), width: width,
            drag: { move(edge, pointer: $0, window: window, from: width) },
            reset: { reset(edge) },
            step: { delta in set(edge, width: width + (isSidebar ? delta : -delta), window: window) }
        )
        .offset(x: x - Chrome.resizeHandle / 2)
    }

    private func reset(_ edge: PaneLayout.Edge) {
        if state.layout != nil { send(.resetEdge(edge.column)) } else { local(edge, width: edge.defaultWidth) }
    }

    /// A keyboard step: the width the column would have, as if the pointer stood there.
    private func set(_ edge: PaneLayout.Edge, width: CGFloat, window: CGFloat) {
        if state.layout != nil {
            send(.dragEdge(edge.column, extent: Double(width)))
        } else {
            local(edge, width: min(max(width, edge.range.lowerBound), edge.range.upperBound))
        }
    }

    private func local(_ edge: PaneLayout.Edge, width: CGFloat) {
        if edge == .sidebar { widths.sidebarWidth = width } else { widths.panelWidth = width }
    }

    private func move(_ edge: PaneLayout.Edge, pointer at: CGPoint?, window: CGFloat, from width: CGFloat) {
        guard let at else {
            (dragBefore, dragCancelled) = (nil, false)
            return finish()
        }
        let pointer = at.x
        // After Esc the button may still be down: the drag is over until it is let go.
        guard !dragCancelled else { return }
        if dragBefore == nil { dragBefore = DragBefore(edge: edge, width: width, widths: widths) }
        let extent = edge == .sidebar ? pointer : window - pointer
        let closing = edge == .sidebar ? Words.text("window-resize-hide", fallback: "Hide") : Words.text("window-resize-close", fallback: "Close")
        if let answer = hub.store.edgeDrag(edge.column, extent: Double(extent)) {
            // Past the point of no return the column stays as it was until the pointer is let go.
            if answer.open { send(.dragEdge(edge.column, extent: Double(extent))) }
            drag = WorkspaceRootView.DragState(
                edge: edge, pointer: pointer, pointerY: at.y, readout: answer.open ? answer.readout : closing, open: answer.open, extent: extent)
            return
        }
        var layout = widths
        layout.sidebarOpen = sidebarWanted
        layout.panelWanted = state.panelVisible
        let resize = ColumnResize(extent: extent, range: edge.range, max: layout.maxWidth(of: edge, window: window))
        local(edge, width: resize.width)
        drag = WorkspaceRootView.DragState(
            edge: edge, pointer: pointer, pointerY: at.y, readout: resize.readout(closing: closing), open: resize.open, extent: extent)
    }

    /// Esc while a column is dragged: it goes back to where the drag began (`cancel_resize`). Whether a drag was going on.
    func cancelDrag() -> Bool {
        guard drag != nil, let before = dragBefore else { return false }
        (dragCancelled, drag, dragBefore) = (true, nil, nil)
        if state.layout != nil { send(.dragEdge(before.edge.column, extent: Double(before.width))) } else { widths = before.widths }
        return true
    }

    private func finish() {
        guard let ended = drag else { return }
        drag = nil
        guard !ended.open else { return }
        // The core closes the column by being dragged past the slack; the sample toggles it.
        if state.layout != nil { send(.dragEdge(ended.edge.column, extent: Double(ended.extent))) } else {
            send(ended.edge == .sidebar ? .toggleSidebar : .togglePanel)
        }
    }
}

extension PaneLayout.Edge {
    var column: ColumnEdge { self == .sidebar ? .sidebar : .panel }
}
