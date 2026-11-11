// A slider with its value and tick labels (`style::slider_control`): the system slider with the schema's marks under the
// track and the value as quiet tabular text beside it. Stateless: it reports each step; the owner writes it.

import DesignSystem
import SettingsModel
import SwiftUI

struct SettingSlider: View {
    let spec: ControlSpec.Slider
    let value: Double
    let label: String
    let onChange: (Double) -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            // The ticks form of the system slider has no step: the value is snapped to the schema's step here.
            Slider(value: Binding(get: { value }, set: { onChange(spec.min + (($0 - spec.min) / spec.step).rounded() * spec.step) }), in: spec.min...spec.max) {
                Text(label)
            } ticks: {
                SliderTickContentForEach(spec.ticks, id: \.self) { tick in
                    SliderTick(tick) { Text(spec.format(tick).replacingOccurrences(of: " %", with: "")) }
                }
            }
            .labelsHidden()
            .accessibilityValue(spec.format(value))
            .frame(width: SettingsMetric.controlWidth)
            Text(spec.format(value)).monospacedDigit().foregroundStyle(tokens.textMuted)
                .frame(width: SettingsMetric.stepperValueWidth, alignment: .trailing)
        }
    }
}
