// A run of the list as cards (`sidebar/card.rs`). A group is one card holding its head and its rows, washed in the
// group's colour; a terminal in no group is a card of its own, and the terminals of one split that sit together share one.
// Cards stand `cardMargin` in from the sidebar's sides with `cardGap` between them. A group folding closes its card's
// height while its rows fade. With cards off the same lines stand in by the margin with no fill.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SidebarSectionView: View {
    let section: SidebarSection
    let members: [TerminalSummary]
    let context: SidebarContext
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics

    var body: some View {
        if section.isHeadless {
            ForEach(Self.cards(of: section.rows), id: \.first?.id) { rows in
                card(rows).padding(.horizontal, metrics.cardMargin).padding(.bottom, metrics.cardGap)
                    .id(SidebarStop.row(rows[0].id)).transition(.opacity.combined(with: .scale(scale: 0.96, anchor: .top)))
            }
        } else {
            CardSurface(wash: section.group?.color.map { $0.wash(tokens).color }) {
                VStack(spacing: 0) {
                    GroupHeadView(section: section, members: members, mark: section.group?.color.map { $0.mark(tokens).color }, context: context)
                        .id(SidebarStop.group(section.id))
                    if !section.isCollapsed {
                        VStack(spacing: 0) { ForEach(section.rows) { row in rowView(row, solo: false).id(SidebarStop.row(row.id)) } }
                            .transition(.opacity)
                    }
                }
            }
            .dropFrame(.card(section.id))
            .popover(isPresented: popoverBinding, arrowEdge: .bottom) {
                if let group = section.group, let edits = context.groupEdits {
                    GroupPopover(group: group, edits: edits) { context.openPopover(nil) }
                }
            }
            .padding(.horizontal, metrics.cardMargin).padding(.bottom, metrics.cardGap)
            .animation(SidebarMotion.fold, value: section.isCollapsed)
        }
    }

    /// One card per terminal in no group, but the members of a split that sit together in one.
    static func cards(of rows: [SidebarRow]) -> [[SidebarRow]] {
        var cards: [[SidebarRow]] = []
        for row in rows {
            if let part = row.splitMarks.bracket, part != .start, !cards.isEmpty { cards[cards.count - 1].append(row) } else { cards.append([row]) }
        }
        return cards
    }

    @ViewBuilder private func card(_ rows: [SidebarRow]) -> some View {
        if rows.count == 1 {
            rowView(rows[0], solo: metrics.cards)
        } else {
            CardSurface { VStack(spacing: 0) { ForEach(rows) { rowView($0, solo: false) } } }
        }
    }

    private func rowView(_ row: SidebarRow, solo: Bool) -> some View {
        let position = (section.rows.firstIndex { $0.id == row.id } ?? 0) + 1
        return TerminalRowView(row: row, isSelected: row.id == context.selected, solo: solo, position: position, count: section.rows.count, context: context)
    }

    private var popoverBinding: Binding<Bool> {
        Binding(get: { context.popover == section.id && context.groupEdits != nil }, set: { if !$0 { context.openPopover(nil) } })
    }
}
