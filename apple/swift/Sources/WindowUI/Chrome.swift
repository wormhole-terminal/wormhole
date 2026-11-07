// The window chrome's measurements, each the GPUI app's own (`style/scale.rs`, `window/chrome.rs`), named as
// they are there so the two can be compared line by line. Not on the 4-pt grid on purpose.

import CoreGraphics
import WorkspaceModel

enum Chrome {
    /// The title rows across the top: the sidebar's, the terminal's breadcrumb, the panel's tabs.
    static let titleHeight: CGFloat = 38
    /// The optional status bar along the bottom.
    static let statusHeight: CGFloat = 28
    /// The least size of anything clickable.
    static let hit: CGFloat = 28
    /// A title row button's drawn face, and its glyph.
    static let face: CGFloat = 24
    static let glyph: CGFloat = 16
    /// The one standard inset from a column's edge to its content.
    static let edgeInset: CGFloat = 12
    /// The terminal's grid padding, and the surface's inset from the window's edge, the sidebar, the panel and
    /// the status bar: the two make the edge inset, so the text lines up with the traffic lights.
    static let terminalPad: CGFloat = 4
    static let surfaceInset: CGFloat = edgeInset - terminalPad
    /// The traffic lights start 12 in; three 12 pt ones 8 apart span 52 unless macOS draws them larger.
    static let lightsX: CGFloat = 12
    static let assumedLightsSpan: CGFloat = 52
    /// The collapse button starts here, 12 after the lights, and never follows the sidebar.
    static let collapseX: CGFloat = 76
    static let afterLights: CGFloat = collapseX - (lightsX + assumedLightsSpan)
    /// The sidebar row's own start: after the collapse button and a small gap.
    static let afterToggle: CGFloat = 6
    /// A title row's own content starts this far after the collapse button when the sidebar is hidden.
    static let afterCollapse: CGFloat = 8
    /// The panel toggle's distance from the window's right edge: its glyph ends on the 12 pt edge.
    static let panelToggleRight: CGFloat = edgeInset - (hit - glyph) / 2
    /// What the toggle takes from the row it floats over.
    static let panelToggleSlot: CGFloat = panelToggleRight + hit
    /// The terminal row's padding beside an open sidebar and an open panel, and at an edge with neither
    /// lights nor toggle.
    static let besideColumn: CGFloat = 4
    static let besidePanel: CGFloat = panelToggleRight
    static let titleRowPad: CGFloat = panelToggleRight
    /// A resize handle straddles its hairline this wide.
    static let resizeHandle: CGFloat = 8
    /// The right panel's tab: a square as tall as a form control.
    static let iconTab: CGFloat = 28
    static let tabGap: CGFloat = 4
    /// Room between breadcrumb parts, and a branch chip's height.
    static let crumbGap: CGFloat = 8
    static let chipHeight: CGFloat = 20
    /// The dot that marks a panel wanted but hidden for room, how far in from its button's corner it sits,
    /// and the ring of surface round it that keeps it apart from the glyph.
    static let markDot: CGFloat = 6
    static let markInset: CGFloat = 5
    static let markRing: CGFloat = 2
    /// A status bar item: 22 tall with 4 round its text, and the hairline between groups.
    static let statusItem: CGFloat = 22
    static let itemPad: CGFloat = 4
    static let statusSeparator: CGFloat = 12
    /// The terminal's status tile in the title row.
    static let titleTile: CGFloat = 18
}

extension WorkspaceState {
    var selectedTerminal: TerminalSummary? { terminals.first { $0.id == selected } }
}
