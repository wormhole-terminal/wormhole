// A slider with its value and tick labels (`style::slider_control`): the system slider tinted with the theme's
// primary, the value as quiet tabular text beside it and the marks under the track. Stateless: it reports each
// step; the owner writes it (a drag is coalesced by the backend, not here).

import DesignSystem
import SettingsModel
import SwiftUI

struct FlatSlider: View {
    let spec: ControlSpec.Slider
    let value: Double
    let label: String
    let onChange: (Double) -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            VStack(spacing: 0) {
                Slider(value: Binding(get: { value }, set: onChange), in: spec.min...spec.max, step: spec.step)
                    .tint(tokens.primary)
                    .labelsHidden()
                    .accessibilityLabel(label)
                    .accessibilityValue(spec.format(value))
                HStack {
                    ForEach(Array(spec.ticks.enumerated()), id: \.offset) { index, tick in
                        if index > 0 { Spacer() }
                        Text(spec.format(tick).replacingOccurrences(of: " %", with: ""))
                    }
                }
                .font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textFaint)
            }
            .frame(width: SettingsMetric.controlWidth)
            Text(spec.format(value))
                .font(.system(size: TextRole.label.size)).monospacedDigit().foregroundStyle(tokens.textPrimary)
                .frame(width: SettingsMetric.stepperValueWidth, alignment: .trailing)
        }
    }
}
