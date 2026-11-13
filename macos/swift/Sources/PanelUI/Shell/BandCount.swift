// A count beside a band's title: a chip filled with the hairline colour, at least as wide as it is tall, the
// digits in `text.primary` 600 at caption size, tabular so they do not wobble (`style::count_badge`).

import DesignSystem
import SwiftUI

struct BandCount: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text)
            .font(tokens.font(.caption).weight(.semibold).monospacedDigit()).lineLimit(1).fixedSize()
            .foregroundStyle(tokens.textPrimary)
            .padding(.horizontal, Space._1)
            .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(tokens.border, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .contentTransition(.numericText())
            .accessibilityHidden(true)
    }
}
