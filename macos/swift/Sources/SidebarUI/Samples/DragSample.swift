// The list while a terminal is dragged, for the Theme Studio's gallery (`studio/gallery/sidebar.rs` `drops`,
// `navigation.rs`'s drag ghost and lifted row): the real list caught mid-drag. On the left the drop would make a split, so
// the row under it is outlined and the dragged one is the compact chip on it; on the right the drop lands between two
// cards, a line in the gap, and the row itself follows the pointer as a lifted card.

import DesignSystem
import SwiftUI
import WorkspaceModel

/// The sidebar's list caught in two drags, for the Studio's gallery.
public struct SidebarDragSpecimen: View {
    @Environment(\.tokens) private var tokens

    private static let state = WorkspaceState(terminals: [
        TerminalSummary(id: 1, title: "sidebar polish", folder: "~/Projects/wormhole", activity: .working, agent: .claude),
        TerminalSummary(id: 2, title: "cargo test", folder: "~/Projects/wormhole"),
        TerminalSummary(id: 3, title: "diff perf", folder: "~/Projects/wormhole", activity: .needsInput, agent: .claude),
    ])
    /// The terminal dragged in both lists: the last one.
    private static let dragged: TerminalID = 3
    /// A list's width: the lifted chip at its widest (`style::drag_ghost` stops at 240) and the card's margins.
    private static let width = TearOffChipView.widest + 2 * SidebarMetrics().cardMargin

    /// A specimen takes nothing: its sample is its own.
    public init() {}

    /// The sample, drawn by the app's own views.
    public var body: some View {
        let metrics = SidebarMetrics()
        // Where the list puts the line for a drop under the first card: half the gap below it (`DropLines.mark`).
        let card = Self.card(metrics, tokens)
        let gap = Space._1 + card + metrics.cardGap / 2
        let pointer = CGPoint(x: Self.width / 2, y: gap + metrics.cardGap)
        HStack(alignment: .top, spacing: Space._4) {
            list(LiveDrag(
                item: .terminal(Self.dragged), point: CGPoint(x: Self.width / 2, y: Space._1 + card / 2),
                aim: DropAim(action: .dropTerminal(Self.dragged, over: .row(1, .middle)), mark: .row(1))))
            list(LiveDrag(
                item: .terminal(Self.dragged), point: pointer,
                aim: DropAim(
                    action: .dropTerminal(Self.dragged, over: .row(2, .top)),
                    mark: .line(y: gap, x: metrics.cardMargin...(Self.width - metrics.cardMargin))),
                grab: CGPoint(x: metrics.titleX, y: metrics.rowHeight / 2)))
        }
        .allowsHitTesting(false)
    }

    private func list(_ drag: LiveDrag) -> some View {
        let rows = CGFloat(Self.state.terminals.count)
        let metrics = SidebarMetrics()
        return SidebarView(
            state: Self.state, appearance: SidebarAppearance(), hooks: SidebarHooks(),
            caught: SidebarView.Caught(drag: drag), send: { _ in })
            .frame(width: Self.width, height: Space._1 + rows * (Self.card(metrics, tokens) + metrics.cardGap))
            .clipped()
    }

    /// A lone terminal's card: its row and the hairline round it (`TerminalRowView`).
    private static func card(_ metrics: SidebarMetrics, _ tokens: ThemeTokens) -> CGFloat {
        metrics.rowHeight + 2 * CGFloat(tokens.metrics.borderWidth)
    }
}
