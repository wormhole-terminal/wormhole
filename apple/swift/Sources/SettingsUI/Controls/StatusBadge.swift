// A word on a quiet tint of its own colour (`style::status_badge`): a Probe's or a project's state. Increase Contrast,
// where a faint fill is no edge, gives it an outline.

import DesignSystem
import SwiftUI

struct StatusBadge: View {
    let text: String
    let color: Color
    @Environment(\.tokens) private var tokens

    /// The tint behind the word (`STATUS_TINT_ALPHA`).
    private static let tint = 0.15

    var body: some View {
        Text(text).font(.system(size: TextRole.caption.size)).foregroundStyle(color)
            .padding(.horizontal, Space._2).frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(color.opacity(Self.tint), in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .overlay {
                if ThemeStore.shared.increaseContrast {
                    RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).strokeBorder(color, lineWidth: tokens.metrics.borderWidth)
                }
            }
            .fixedSize()
    }
}
