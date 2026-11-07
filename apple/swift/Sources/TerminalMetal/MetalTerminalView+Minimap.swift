// The minimap takes the scroll bar's place along the right edge (View > Show Minimap, `terminal_view/render/mod.rs`):
// the grid leaves its width free and the bar is hidden. The minimap itself is the window's (SwiftUI, `TerminalUI`).

import AppKit

extension MetalTerminalView {
    func minimapChanged() {
        scroller.isSuppressed = minimapWidth > 0
        syncSurface()
        needsLayout = true
        requestRedraw()
    }
}

extension MetalTerminalView {
    /// The grid's cell and what the view has beyond its whole cells (across, down), in points: what a window that resizes in
    /// steps of a cell reads (`GridProbe`).
    public var cellAndSpare: (cell: CGSize, spare: CGSize)? {
        let cell = renderer.cellPixels, scale = CGFloat(renderer.scale), grid = renderer.gridSize
        guard cell.width > 0, cell.height > 0, grid.cols > 0 else { return nil }
        let size = CGSize(width: CGFloat(cell.width) / scale, height: CGFloat(cell.height) / scale)
        let pad = 2 * MetalRenderer.padding
        return (size, CGSize(
            width: bounds.width - scroller.gutterWidth - minimapWidth - pad - size.width * CGFloat(grid.cols),
            height: bounds.height - pad - size.height * CGFloat(grid.rows)))
    }
}
