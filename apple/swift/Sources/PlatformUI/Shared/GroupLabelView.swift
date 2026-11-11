// The label over a group of rows (`style::palette_label`): "Needs input 2", 12 / 600 muted on a one-line row,
// sitting on the rows below it, the count faint.

import DesignSystem
import SwiftUI

struct GroupLabelView: View {
    let title: String
    let count: Int?
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(alignment: .lastTextBaseline, spacing: Space._2) {
            Text(title).font(.system(size: TextRole.caption.size, weight: .semibold)).foregroundStyle(tokens.textMuted)
            if let count {
                Text(String(count)).font(.system(size: TextRole.caption.size, weight: .medium)).monospacedDigit().foregroundStyle(tokens.textFaint)
            }
        }
        .padding(.horizontal, Space._4)
        .padding(.bottom, Space._1)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
        .frame(height: Metric.row)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isHeader)
    }
}
