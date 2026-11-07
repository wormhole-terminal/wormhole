// What a select looks like (`SelectTrigger`): its value on the control fill with up/down chevrons, brighter under
// the pointer. The menu (`FlatSelect`) and the font picker's popover both open from it.

import DesignSystem
import SwiftUI

struct SelectFace<Value: View>: View {
    var width: CGFloat = SettingsMetric.controlWidth
    @ViewBuilder let value: Value
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        HStack(spacing: Space._2) {
            value
            Image(systemName: "chevron.up.chevron.down").font(.system(size: Metric.chevron, weight: .semibold))
                .foregroundStyle(tokens.textMuted)
        }
        .padding(.horizontal, Space._3)
        .frame(width: width, height: Metric.control)
        .background(hovering ? tokens.controlHover : tokens.controlBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .foregroundStyle(tokens.textPrimary)
        .contentShape(Rectangle())
        .onHover { hovering = $0 }
    }
}
