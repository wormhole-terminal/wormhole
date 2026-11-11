// How wide the three columns are, ported from the GPUI app's `view::layout::Panes` so a window of a given
// width lays out the same in both: the panel yields first, the sidebar next, the terminal keeps its minimum.
// Pure values; the window feeds it what the person wants and the window's width.

import CoreGraphics

struct PaneLayout: Equatable {
    static let sidebarDefault: CGFloat = 350
    static let sidebarRange: ClosedRange<CGFloat> = 240...420
    static let panelDefault: CGFloat = 360
    static let panelRange: ClosedRange<CGFloat> = 240...720
    static let terminalMin: CGFloat = 320

    var sidebarOpen = true
    var sidebarWidth = sidebarDefault
    var panelWanted = true
    var panelWidth = panelDefault

    struct Columns: Equatable {
        var sidebar: CGFloat
        var terminal: CGFloat
        var panel: CGFloat
    }

    /// The sidebar's width while open: its chosen one unless the window cannot spare it.
    func sidebar(window: CGFloat) -> CGFloat? {
        guard sidebarOpen else { return nil }
        return max(min(sidebarWidth, window - Self.terminalMin), Self.sidebarRange.lowerBound)
    }

    /// Whether the panel, at its least width, fits beside the sidebar and a usable terminal.
    func panelFits(window: CGFloat) -> Bool {
        window >= (sidebar(window: window) ?? 0) + Self.terminalMin + Self.panelRange.lowerBound
    }

    /// Wanted and roomy. A panel hidden for room stays wanted, so widening the window brings it back.
    func panelVisible(window: CGFloat) -> Bool { panelWanted && panelFits(window: window) }

    func columns(window: CGFloat) -> Columns {
        let sidebar = sidebar(window: window) ?? 0
        let panel =
            panelVisible(window: window)
            ? max(min(panelWidth, window - sidebar - Self.terminalMin), Self.panelRange.lowerBound) : 0
        return Columns(sidebar: sidebar, terminal: max(window - sidebar - panel, Self.terminalMin), panel: panel)
    }

    /// The widest a column may be dragged: its own maximum, or less when the window cannot spare it.
    func maxWidth(of edge: Edge, window: CGFloat) -> CGFloat {
        let room = edge == .sidebar ? window - Self.terminalMin : window - (sidebar(window: window) ?? 0) - Self.terminalMin
        return max(min(edge.range.upperBound, room), edge.range.lowerBound)
    }

    enum Edge {
        case sidebar, panel

        var range: ClosedRange<CGFloat> { self == .sidebar ? PaneLayout.sidebarRange : PaneLayout.panelRange }
        var defaultWidth: CGFloat { self == .sidebar ? PaneLayout.sidebarDefault : PaneLayout.panelDefault }
    }
}
