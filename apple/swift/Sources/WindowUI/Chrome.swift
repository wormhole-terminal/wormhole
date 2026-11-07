// The window chrome's measurements, from the GPUI app (`style/scale.rs`, `view/layout`, `window/chrome.rs`).
// Not on the 4-pt grid on purpose: they are the app's own numbers, so the two look alike.

import DesignSystem
import SwiftUI
import WorkspaceModel

enum Chrome {
    static let titleHeight: CGFloat = 38
    static let statusHeight: CGFloat = 24
    /// A button's hit target and its glyph.
    static let hit: CGFloat = 28
    /// Where the sidebar's collapse button starts: after the traffic lights (12 + 52) and a 12 pt gap. It sits
    /// at this spot whether the sidebar is open or hidden, so it never jumps under the pointer.
    static let collapseX: CGFloat = 76
    /// Without the lights (full screen) the button's glyph sits on the 12 pt edge instead.
    static let collapseXFullScreen: CGFloat = 6
    static let sidebarWidth: CGFloat = 260
    static let sidebarRange: ClosedRange<CGFloat> = 240...420
    static let panelWidth: CGFloat = 320
    static let panelRange: ClosedRange<CGFloat> = 240...720
    /// The terminal keeps at least this much, as in the GPUI app.
    static let terminalMin: CGFloat = 320
    static let surface = Color(nsColor: .windowBackgroundColor)
    static let terminalSurface = Color(nsColor: .black)
}

extension WorkspaceState {
    var selectedTerminal: TerminalSummary? { terminals.first { $0.id == selected } }
}
