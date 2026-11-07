// A run of the list as cards (`sidebar/card.rs`). A group is one card holding its head and its rows, washed in the
// group's colour; a terminal in no group (and a pin) is a card of its own. Cards stand `cardMargin` in from the
// sidebar's sides with `cardGap` between them. A group folding closes its card's height while its rows fade.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SidebarSectionView: View {
    let section: SidebarSection
    let members: [TerminalSummary]
    let selected: TerminalID?
    let wash: Color?
    let send: @MainActor (WorkspaceAction) -> Void
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics

    var body: some View {
        if section.isHeadless {
            ForEach(Array(section.rows.enumerated()), id: \.element.id) { _, row in
                card(row, solo: true).padding(.horizontal, metrics.cardMargin).padding(.bottom, metrics.cardGap)
                    .id(SidebarStop.row(row.id)).transition(.opacity.combined(with: .scale(scale: 0.96, anchor: .top)))
            }
        } else {
            let shape = RoundedRectangle(cornerRadius: metrics.cardRadius)
            VStack(spacing: 0) {
                GroupHeadView(section: section, members: members, send: send).id(SidebarStop.group(section.id))
                if !section.isCollapsed {
                    VStack(spacing: 0) {
                        ForEach(section.rows) { row in card(row, solo: false).id(SidebarStop.row(row.id)) }
                    }
                    .transition(.opacity)
                }
            }
            .padding(metrics.cardPad)
            .background {
                shape.fill(tokens.card)
                if let wash { shape.fill(wash.opacity(RowGeometry.washPeak)) }
            }
            .overlay { shape.strokeBorder(tokens.cardEdge, lineWidth: tokens.metrics.borderWidth) }
            .clipShape(shape)
            .padding(.horizontal, metrics.cardMargin).padding(.bottom, metrics.cardGap)
            .animation(SidebarMotion.fold, value: section.isCollapsed)
        }
    }

    private func card(_ row: SidebarRow, solo: Bool) -> some View {
        TerminalRowView(row: row, isSelected: row.id == selected, solo: solo, position: position(of: row), count: section.rows.count, send: send)
    }

    private func position(of row: SidebarRow) -> Int { (section.rows.firstIndex { $0.id == row.id } ?? 0) + 1 }
}
