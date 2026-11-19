// What a folded head keeps of its terminals (`group_head.rs`): their tiles, loudest first as the core ranked them, their
// count and a dot while one needs input (amber) or failed (the danger colour) — the core says which.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct FoldedSummary: View {
    let tiles: [MemberTile]
    let count: Int
    let alert: Urgency?
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics

    var body: some View {
        HStack(spacing: RowGeometry.summaryGap) {
            ForEach(tiles) { tile in
                StatusTile(
                    state: TileState(tile), agent: tile.agent, size: metrics.foldedTile, phase: StatusTile.phase(of: tile.id),
                    surface: tokens.sidebarBackground)
                    .help(tile.name)
            }
            CountBadge(count)
            if let alert {
                Circle().fill(alert == .failed ? tokens.statusDanger : tokens.agentAttention)
                    .frame(width: RowGeometry.alertDot, height: RowGeometry.alertDot)
            }
        }
    }
}
