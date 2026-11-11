// What follows the pointer while a pane is dragged by its header: its tile and name on a small card.

import DesignSystem
import SwiftUI

struct DragChip: View {
    let info: PaneInfo
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            TerminalTile(activity: info.activity)
            Text(info.title).font(TextRole.label.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
        }
        .padding(.horizontal, Space._2).frame(height: Metric.row)
        .background(tokens.surfaceElevated, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
    }
}
