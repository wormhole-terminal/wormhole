// The flat switch of the settings rows: the theme's track (the primary while on), a white knob that slides.
// Stateless: it draws `isOn` and reports a click. VoiceOver gets a real switch with the row's label.

import DesignSystem
import SwiftUI

struct FlatSwitch: View {
    let isOn: Bool
    let label: String
    let onChange: (Bool) -> Void
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var isEnabled

    var body: some View {
        let travel = SettingsMetric.switchWidth - SettingsMetric.switchKnob - 4
        Button { onChange(!isOn) } label: {
            Capsule()
                .fill(isOn ? tokens.primary : tokens.switchTrack)
                .frame(width: SettingsMetric.switchWidth, height: SettingsMetric.switchHeight)
                .overlay(alignment: .leading) {
                    Circle()
                        .fill(tokens.switchKnob)
                        .frame(width: SettingsMetric.switchKnob, height: SettingsMetric.switchKnob)
                        .shadow(color: tokens.shadow, radius: 1, y: 0.5)
                        .offset(x: 2 + (isOn ? travel : 0))
                }
                .motion(Motion.selection, value: isOn)
                .opacity(isEnabled ? 1 : 0.5)
                .frame(minWidth: Metric.hitMin, minHeight: Metric.hitMin)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityRepresentation { Toggle(label, isOn: Binding(get: { isOn }, set: onChange)) }
    }
}
