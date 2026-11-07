// Dragging a selection: it follows the pointer, and past the top or bottom of the grid the history scrolls and
// goes on scrolling (a one-shot timer, only while the pointer stays out there) even if the mouse is still.

import AppKit
import TerminalCore

extension MetalTerminalView {
    /// Follow the pointer with the selection. Past the top or bottom the history scrolls and goes on scrolling
    /// (with a one-shot timer, only while the pointer stays out there) even if the mouse is still.
    func extendSelection(to point: CGPoint) {
        guard let interaction else { return }
        let grid = cellGrid
        let lines = grid.dragScroll(point)
        if let lines, lines != 0 { source.scroll(lines: lines) }
        if let hit = grid.hit(point) { interaction.selectionUpdate(row: hit.row, column: hit.column, side: hit.side) }
        requestRedraw()
        pointer.autoscroll?.cancel()
        pointer.autoscroll = nil
        guard let lines, lines != 0 else { return }
        let item = DispatchWorkItem { [weak self] in
            MainActor.assumeIsolated {
                guard let self, self.pointer.selecting, let point = self.pointer.lastPoint else { return }
                self.extendSelection(to: point)
            }
        }
        pointer.autoscroll = item
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.autoscrollInterval, execute: item)
    }

    /// How often a drag held outside the grid scrolls again (the system's own periodic rate for dragging in
    /// a list; the GPUI app scrolls on each pointer event and has no timer).
    static var autoscrollInterval: DispatchTimeInterval { .milliseconds(50) }
}
