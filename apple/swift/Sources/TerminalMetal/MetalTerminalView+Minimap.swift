// The minimap takes the scroll bar's place along the right edge (View > Show Minimap, `terminal_view/render/mod.rs`):
// the grid leaves its width free and the bar is hidden. The minimap itself is the window's (SwiftUI, `TerminalUI`).

extension MetalTerminalView {
    func minimapChanged() {
        scroller.isSuppressed = minimapWidth > 0
        syncSurface()
        needsLayout = true
        requestRedraw()
    }
}
