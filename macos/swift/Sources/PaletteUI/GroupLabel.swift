// The label over a group of results (`style::palette_label`): "Needs input 2", 12 / 600 muted on a one-line row,
// sitting on the rows below it, the count faint.

import DesignSystem
import SwiftUI
import WordsModel

struct GroupLabel: View {
    let title: String
    let count: Int?
    /// The trusted project's label wears a Trusted badge.
    var trusted = false
    /// The label's side inset: a palette's, or none where a column of the sheet gives it room.
    var inset: CGFloat = Space._4
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(alignment: .lastTextBaseline, spacing: Space._2) {
            Text(title)
                .font(.system(size: tokens.size(.caption), weight: .semibold))
                .foregroundStyle(tokens.textMuted)
            if trusted { StatusBadge(text: Words.text("settings-projects-trusted", fallback: "Trusted"), color: tokens.statusSuccessText) }
            if let count {
                Text(String(count))
                    .font(.system(size: tokens.size(.caption), weight: .medium))
                    .monospacedDigit()
                    .foregroundStyle(tokens.textFaint)
            }
        }
        .padding(.horizontal, inset)
        .padding(.bottom, Space._1)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
        .frame(height: Metric.row)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isHeader)
    }
}
