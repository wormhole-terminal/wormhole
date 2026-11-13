// The count on a tab of the panel's strip (`style::icon_tab`'s `count`): a `count_badge` beside the icon, and on the active
// tab (`selected_count_badge`) a stronger tint of the primary (`state.selected.inset`) with the tint's text colour. Past 999
// it reads "999+".

import DesignSystem
import SwiftUI

public struct TabCountBadge: View {
    let count: Int
    let selected: Bool
    @Environment(\.tokens) private var tokens

    public init(count: Int, selected: Bool) {
        self.count = count
        self.selected = selected
    }

    public var body: some View {
        Text(count > 999 ? "999+" : "\(count)")
            .font(tokens.font(.caption).weight(.semibold).monospacedDigit())
            .foregroundStyle(selected ? tokens.sidebarRowSelectedText : tokens.textPrimary)
            .padding(.horizontal, Space._1)
            .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(selected ? tokens.stateSelectedInset : tokens.border, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .contentTransition(.numericText(value: Double(count)))
            .accessibilityHidden(true)
    }
}
