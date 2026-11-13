// A run of the list as cards (`sidebar/card.rs`). A group is one card holding its head and its rows, washed in the
// group's colour; a terminal in no group is a card of its own, and the terminals of one split that sit together share one.
// Cards stand `cardMargin` in from the sidebar's sides with `cardGap` between them. A group folding closes its card's
// height while its rows fade. With cards off the same lines stand in by the margin with no fill.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SidebarSectionView: View {
    let section: SidebarSection
    let context: SidebarContext
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.sidebarLook) private var look
    @Environment(\.reduceMotion) private var reduceMotion

    var body: some View {
        if let head = section.head {
            groupCard(head)
        } else {
            ForEach(Self.cards(of: section.rows), id: \.first?.id) { rows in
                card(rows).padding(.horizontal, metrics.cardMargin).padding(.bottom, metrics.cardGap)
                    .id(SidebarStop.row(rows[0].id)).transition(.listRow(look.insertMotion, reduceMotion: reduceMotion))
            }
        }
    }

    private func groupCard(_ head: GroupHead) -> some View {
        CardSurface(wash: head.wash(tokens)?.color) {
            VStack(spacing: 0) {
                GroupHeadView(section: section, head: head, mark: head.mark(tokens)?.color, context: context)
                    .id(SidebarStop.group(head.id))
                GroupRows(section: section, head: head) { row in rowView(row, solo: false).id(SidebarStop.row(row.id)) }
            }
        }
        .dropFrame(.card(head.id))
        .popover(isPresented: popoverBinding(head.id), arrowEdge: .bottom) {
            GroupPopover(head: head, rows: section.rows, context: context) { dismissPopover() }
        }
        .padding(.horizontal, metrics.cardMargin).padding(.bottom, metrics.cardGap)
        // Under Classic the card closes on the timed curve; Drawer and Tuck close it by the rows' own height (`GroupRows`).
        .animation(look.foldMotion == "classic" ? SidebarMotion.fold : nil, value: head.isFolded)
    }

    /// One card per terminal in no group, but the members of a split that sit together in one.
    nonisolated static func cards(of rows: [SidebarRow]) -> [[SidebarRow]] {
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
        TerminalRowView(row: row, isSelected: row.id == context.selected, solo: solo, context: context)
    }

    private func popoverBinding(_ group: GroupID) -> Binding<Bool> {
        Binding(get: { context.popover == group }, set: { if !$0 { dismissPopover() } })
    }

    /// Closing the popover gives the keyboard back to the terminal (`group_popover.rs` `on_close`).
    private func dismissPopover() {
        context.openPopover(nil)
        context.hooks.focusTerminal()
    }
}
