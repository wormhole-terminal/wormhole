// What a split's panes measure, each the GPUI app's own (`style/panes.rs`, `style/scale.rs`), named as there.

import CoreGraphics

enum PaneMetrics {
    /// The gap between panes that are each their own rounded surface (`split_gaps`, `PANE_GAP`).
    static let gap: CGFloat = 8
    /// How far the tile stands from such a pane's top and left, and its text from the sides (`PANE_INSET`).
    static let inset: CGFloat = 8
    /// The terminal grid's own padding; the text pad is what is left of the inset (`TERMINAL_PAD`).
    static let terminalPad: CGFloat = 4
    static let textPad: CGFloat = inset - terminalPad
    /// The status tile of a pane's header (`TILE_TITLE`).
    static let tile: CGFloat = 18
    /// A header is as tall as a one-line row (`ROW_H`), or with gaps the tile and the inset above and below.
    static let rowHeader: CGFloat = 28
    static var gapHeader: CGFloat { tile + 2 * inset }
    /// The line under the focused pane's header, in the primary (`PANE_FOCUS_LINE`).
    static let focusLine: CGFloat = 2
    /// How much of its strength a pane without the keyboard keeps (`PANE_DIM`).
    static let dim: Double = 0.72
    /// A divider's handle straddles the edge this wide (`RESIZE_HANDLE_W`); its grabber is a pill this long.
    static let handle: CGFloat = 8
    static let grabber: CGFloat = 32
    /// The pane being dragged by its header stays in place, lifted (`DRAG_LIFT_OPACITY`).
    static let dragLift: Double = 0.4
    /// A header's name is at most this wide (`TITLE_TAB_MAX_W`).
    static let nameMax: CGFloat = 220
    /// A header's button: a control inside a row (`CONTROL_H_SM`), and its glyph (`GLYPH`).
    static let button: CGFloat = 22
    static let glyph: CGFloat = 14
    /// The drop line round the zone the pointer is on (`DROP_LINE_W`).
    static let dropLine: CGFloat = 2
    /// A divider lights after the pointer rests on it this long, so crossing it doesn't flash it
    /// (`RESIZE_HANDLE_HOVER_DELAY`, 150 ms).
    static let hoverDelay: Duration = .milliseconds(150)
    /// The folder in a header keeps this many characters, cut from the front (a sidebar row's second line).
    static let folderMax = 38
}
