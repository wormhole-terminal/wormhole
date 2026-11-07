// What a folded head keeps of its terminals: their count and a dot while one needs input or failed (`group_head.rs`).
// The core's snapshot lists no rows of a folded group, so the GPUI head's tiles (loudest first) wait for it to say which
// states are folded away; the dot is amber for want of knowing whether it is a question or a failure.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct FoldedSummary: View {
    let count: Int
    let alerts: Int
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: RowGeometry.summaryGap) {
            Text(CountBadge.label(count))
                .font(TextRole.caption.font.weight(.semibold).monospacedDigit()).foregroundStyle(tokens.textPrimary)
                .padding(.horizontal, Space._1)
                .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
                .background(tokens.border, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            if alerts > 0 {
                Circle().fill(tokens.agentAttention).frame(width: RowGeometry.alertDot, height: RowGeometry.alertDot)
            }
        }
    }
}
