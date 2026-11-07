// What a folded head keeps of its terminals: their tiles, loudest first as core sorts them (terminals alike keep the
// list's order), their count and a dot while one needs input or failed (`group_head.rs`).

import DesignSystem
import SwiftUI
import WorkspaceModel

struct FoldedSummary: View {
    let members: [TerminalSummary]
    let count: Int
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics

    var body: some View {
        let tiles = members.sorted { TileState($0) < TileState($1) }
        let alert = tiles.first.map(TileState.init).flatMap { $0.isAlert ? $0 : nil }
        HStack(spacing: RowGeometry.summaryGap) {
            ForEach(tiles) { terminal in
                StatusTile(state: TileState(terminal), agent: terminal.agent, size: metrics.foldedTile,
                           phase: StatusTile.phase(of: terminal.id), surface: tokens.sidebarBackground)
            }
            Text(CountBadge.label(count))
                .font(TextRole.caption.font.weight(.semibold).monospacedDigit()).foregroundStyle(tokens.textPrimary)
                .padding(.horizontal, Space._1)
                .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
                .background(tokens.border, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            if let alert {
                Circle().fill(alert == .needsYou ? tokens.agentAttention : tokens.statusDanger)
                    .frame(width: RowGeometry.alertDot, height: RowGeometry.alertDot)
            }
        }
    }
}
