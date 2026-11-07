// How wide the sidebar and the panel are at this window width, and whether the panel is in. The core fits them
// (`ChromeSnapshot`'s columns and `panelToggle`: the panel yields first, then the sidebar, the terminal keeps its least
// width); only the sample, which has no window width to give the core, falls back to the local copy of the rules
// (`PaneLayout`). A column's width is what it is when open, and what it will be when it opens again while it is shut,
// so a column sliding out keeps its shape.

import CoreGraphics
import WorkspaceModel

struct ColumnFit: Equatable {
    var sidebarWidth: CGFloat
    var panelWidth: CGFloat
    /// The panel is wanted and has room.
    var panelOpen: Bool
    /// Wanted, without room: the toggle wears a dot that says it comes back.
    var panelWaiting: Bool

    init(window: CGFloat, state: WorkspaceState, sidebarWanted: Bool, fallback: PaneLayout) {
        if let layout = state.layout {
            let columns = layout.columns, limits = columns.limits
            func chosen(_ width: Double, _ bounds: ClosedRange<Double>) -> CGFloat { CGFloat(min(max(width, bounds.lowerBound), bounds.upperBound)) }
            sidebarWidth = columns.sidebar > 0 ? CGFloat(columns.sidebar) : chosen(columns.sidebarChosen, limits.sidebarMin...limits.sidebarMax)
            panelWidth = columns.panel > 0 ? CGFloat(columns.panel) : chosen(columns.panelChosen, limits.panelMin...limits.panelMax)
            panelOpen = layout.panelToggle == .open
            panelWaiting = layout.panelToggle == .waiting
        } else {
            var layout = fallback
            layout.sidebarOpen = sidebarWanted
            layout.panelWanted = state.panelVisible
            sidebarWidth = max(min(fallback.sidebarWidth, window - PaneLayout.terminalMin), PaneLayout.sidebarRange.lowerBound)
            panelWidth = max(min(fallback.panelWidth, window - sidebarWidth - PaneLayout.terminalMin), PaneLayout.panelRange.lowerBound)
            panelOpen = layout.panelVisible(window: window)
            panelWaiting = state.panelVisible && !panelOpen
        }
    }
}
