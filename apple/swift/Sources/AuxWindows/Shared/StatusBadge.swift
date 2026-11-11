// A short status word in a colour (trusted, changed, Attached): the outlined sibling of the count badge, with the
// same height and chip corners (`style::status_badge`).

import DesignSystem
import SwiftUI

struct StatusBadge: View {
    let text: String
    let color: Color
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text)
            .font(TextRole.caption.font.monospacedDigit())
            .foregroundStyle(color)
            .padding(.horizontal, Space._2)
            .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .overlay {
                RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
                    .strokeBorder(color, lineWidth: tokens.metrics.borderWidth)
            }
            .fixedSize()
            .marked("badge")
    }
}
