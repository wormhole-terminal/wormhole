// The mark where a drop would land: a 2-point line in the primary colour between two lines of the list, or a group's
// card (or a row) outlined while a drop would join it (`style::drop_line`, the card's focus outline), and the lifted item that
// follows the pointer, a card with the shadow of something floating.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct DropMarkView: View {
    let mark: DropMark
    let lines: DropLines
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics

    private static let line: CGFloat = 2

    var body: some View {
        switch mark {
        case .line(let y, let x):
            Capsule().fill(tokens.primary)
                .frame(width: x.upperBound - x.lowerBound, height: Self.line)
                .position(x: (x.lowerBound + x.upperBound) / 2, y: y)
        case .card(let group): outline(lines.cards[group], radius: metrics.cardRadius)
        case .row(let id): outline(lines.rows[id], radius: metrics.rowRadius)
        }
    }

    @ViewBuilder private func outline(_ frame: CGRect?, radius: CGFloat) -> some View {
        if let frame {
            RoundedRectangle(cornerRadius: radius)
                .strokeBorder(tokens.primary, lineWidth: tokens.metrics.focusRingWidth)
                .frame(width: frame.width, height: frame.height)
                .position(x: frame.midX, y: frame.midY)
        }
    }
}

/// What follows the pointer while a row or a head is dragged: its tile and name on a card.
struct DragGhost: View {
    let terminal: TerminalSummary?
    let title: String
    var shadow = true
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics

    var body: some View {
        HStack(spacing: metrics.tileGap) {
            if let terminal {
                StatusTile(state: TileState(terminal), agent: terminal.agent, size: metrics.tile, phase: 0, surface: tokens.card)
            }
            Text(title).font(TextRole.body.font.weight(.medium)).foregroundStyle(tokens.textPrimary).lineLimit(1)
        }
        .padding(.horizontal, metrics.rowInset)
        .padding(.vertical, Space._1)
        .background(tokens.surfaceElevated, in: RoundedRectangle(cornerRadius: metrics.cardRadius))
        .overlay { RoundedRectangle(cornerRadius: metrics.cardRadius).strokeBorder(tokens.cardEdge, lineWidth: tokens.metrics.borderWidth) }
        .shadow(color: shadow ? tokens.shadow : .clear, radius: Space._3, y: Space._1)
        .allowsHitTesting(false)
    }
}
