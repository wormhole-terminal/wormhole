// The box of a next step in the lead column (`style::checkbox`): 14 pt in the chip corners, `checkbox.background`
// with a `checkbox.border` outline, filled `checkbox.checked` with a mark in `text.on_primary` when on. The fill
// pops in and the mark draws itself in one spring; Reduce Motion cross-fades the two states.

import DesignSystem
import SwiftUI

struct TaskCheckbox: View {
    let checked: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.panelMotion) private var motion

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusChip, style: .continuous)
        ZStack {
            shape.fill(tokens.checkboxBackground)
            shape.fill(tokens.checkboxChecked).opacity(checked ? 1 : 0).scaleEffect(checked || motion.reduced ? 1 : 0.5)
            shape.strokeBorder(checked ? tokens.checkboxChecked : tokens.checkboxBorder, lineWidth: tokens.metrics.borderWidth)
            Tick().trim(from: 0, to: checked ? 1 : 0)
                .stroke(tokens.textOnPrimary, style: StrokeStyle(lineWidth: 1.6, lineCap: .round, lineJoin: .round))
                .frame(width: PanelMetrics.checkMark, height: PanelMetrics.checkMark * 0.75)
        }
        .frame(width: PanelMetrics.checkbox, height: PanelMetrics.checkbox)
        .frame(width: Metric.lead, height: Metric.lead)
        .animation(motion.tick, value: checked)
        .accessibilityHidden(true)
    }

    private struct Tick: Shape {
        func path(in rect: CGRect) -> Path {
            var path = Path()
            path.move(to: CGPoint(x: rect.minX, y: rect.midY))
            path.addLine(to: CGPoint(x: rect.minX + rect.width * 0.38, y: rect.maxY))
            path.addLine(to: CGPoint(x: rect.maxX, y: rect.minY))
            return path
        }
    }
}
