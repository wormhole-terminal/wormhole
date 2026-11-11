// The chip of a terminal dragged out of its window: what the lifted row looks like (`DragGhost`), drawn by the window
// of its own that follows the pointer over the desktop (`TearOffChip`) and again as the face of the window that grows
// from it (`TearOffGrow`). Those hosts are outside the window's tree, so it brings its own theme and metrics.

import DesignSystem
import SwiftUI
import WorkspaceModel

public struct TearOffChipView: View {
    let terminal: TerminalSummary?
    let title: String
    let appearance: SidebarAppearance
    /// Room around the chip for its shadow (a window of its own is cut at its edge).
    let bleed: Bool

    public init(terminal: TerminalSummary?, title: String, appearance: SidebarAppearance, bleed: Bool = true) {
        (self.terminal, self.title, self.appearance, self.bleed) = (terminal, title, appearance, bleed)
    }

    public var body: some View {
        DragGhost(terminal: terminal, title: title, shadow: bleed)
            .frame(maxWidth: Self.widest)
            .environment(\.sidebarMetrics, SidebarMetrics(density: appearance.density, cards: appearance.cards))
            .fixedSize()
            .padding(bleed ? Self.margin : 0)
            .themed()
    }

    /// The widest the card gets (`style::drag_ghost` stops at 240): a longer name is cut.
    static let widest: CGFloat = 240

    /// The room a window-sized chip has around the card: the shadow's radius and drop (`DragGhost`).
    public static var margin: CGFloat { Space._4 }
}
