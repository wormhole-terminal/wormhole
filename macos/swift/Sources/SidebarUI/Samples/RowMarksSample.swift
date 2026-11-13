// Two small parts of a terminal's row on their own, for the Theme Studio's gallery: the chain mark a row wears when its
// split's other terminals are elsewhere in the list (`style::chain_mark`, `gallery/sidebar.rs`), and the × that closes a
// terminal inside the room that answers a click (`style::hit_area`, `gallery/chrome.rs`: a 22 pt × in its 28 pt hit area).

import DesignSystem
import SwiftUI
import WorkspaceModel

/// What a row says at its first line's end, with the chain mark of a split spread over the list.
public struct ChainMarkSpecimen: View {
    /// A specimen takes nothing: its sample is its own.
    public init() {}

    /// The sample, drawn by the app's own views.
    public var body: some View {
        RowDetails(
            terminal: TerminalSummary(id: 1, title: "landing hero", noteCount: 2),
            marks: SplitMarks(chain: true, members: 2), isSelected: false)
    }
}

/// The row's ×, with the area that takes its click outlined round it, and the × as a selected row wears it.
public struct CloseHitSpecimen: View {
    @Environment(\.tokens) private var tokens

    /// A specimen takes nothing: its sample is its own.
    public init() {}

    /// The sample, drawn by the app's own views.
    public var body: some View {
        HStack(spacing: Space._4) {
            CloseButton(isSelected: false) {}
                .frame(width: Metric.hitMin, height: Metric.hitMin)
                .overlay { Rectangle().strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
            CloseButton(isSelected: true) {}
                .frame(width: Metric.hitMin, height: Metric.hitMin)
                .background(tokens.sidebarRowSelected, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
        }
    }
}
