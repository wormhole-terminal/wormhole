// One terminal in the list (`sidebar/row/`): its tile, then the name over a second line. A terminal in no group is a
// card of its own (`solo`): the card takes the selection's tint and the hover, the row draws neither. In a group's card
// the row is a rounded stretch with its own. The pointer shows the × at the line's end; ⌘ held shows the keycap.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct TerminalRowView: View {
    let row: SidebarRow
    let isSelected: Bool
    let solo: Bool
    /// Its place among the visible rows, for VoiceOver.
    let position: Int
    let count: Int
    let send: @MainActor (WorkspaceAction) -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.showsHints) private var showsHints
    @Environment(\.keyedStop) private var keyed
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var terminal: TerminalSummary { row.terminal }
    private var inset: CGFloat { metrics.rowInset + (solo ? metrics.cardPad : 0) }

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: solo ? metrics.cardRadius : metrics.rowRadius)
        ZStack(alignment: .topTrailing) {
            Button { send(.select(terminal.id)) } label: { content }
                .buttonStyle(.plain)
            if hovering && !showsHints {
                CloseButton(isSelected: isSelected) { send(.close(terminal.id)) }
                    .padding(.top, RowGeometry.closeTop).padding(.trailing, inset - (RowGeometry.closeBox - Metric.glyph) / 2)
                    .transition(.opacity)
            }
        }
        .background(shape.fill(fill))
        .overlay { if solo { shape.strokeBorder(isSelected ? tokens.sidebarRowSelected : tokens.cardEdge, lineWidth: tokens.metrics.borderWidth) } }
        .overlay {
            if keyed == .row(terminal.id) {
                RoundedRectangle(cornerRadius: Radius.row).strokeBorder(tokens.borderFocus, lineWidth: tokens.metrics.focusRingWidth)
            }
        }
        .contentShape(shape)
        .onHover { hovering = $0 }
        .animation(SidebarMotion.state, value: hovering)
        .animation(SidebarMotion.state, value: isSelected)
        .contextMenu { SidebarMenu.row(terminal, send: send) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(SidebarListing.spokenLabel(for: row, selected: isSelected, position: position, of: count))
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction { send(.select(terminal.id)) }
        .accessibilityAction(named: terminal.isPinned ? "Unpin" : "Pin") { send(.togglePin(terminal.id)) }
        .accessibilityAction(named: "Close terminal") { send(.close(terminal.id)) }
    }

    /// The card's own fill when alone; the row's when it stands in a group's card (whose fill shows through).
    private var fill: Color {
        if isSelected { return tokens.sidebarRowSelected }
        if hovering { return tokens.stateHover }
        return solo ? tokens.card : .clear
    }

    private var content: some View {
        HStack(spacing: metrics.tileGap) {
            StatusTile(state: TileState(terminal), size: metrics.tile, phase: StatusTile.phase(of: terminal.id),
                       surface: isSelected ? tokens.sidebarRowSelected : tokens.sidebarBackground)
            RowLines(terminal: terminal, number: row.number, isSelected: isSelected, hovering: hovering)
        }
        .padding(.horizontal, inset)
        .padding(.vertical, metrics.density.twoLines ? Space._1 : 0)
        .frame(maxWidth: .infinity, minHeight: metrics.rowHeight, alignment: .leading)
        .contentShape(Rectangle())
    }
}
