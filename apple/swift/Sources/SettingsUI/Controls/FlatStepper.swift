// A small number with a minus and a plus, as System Settings draws a stepper (`style::stepper`): the value as
// plain text, then one compact control of two halves on the control fill with a hair of the card between them.

import DesignSystem
import SwiftUI

struct FlatStepper: View {
    let value: String
    let label: String
    let canDecrease: Bool
    let canIncrease: Bool
    let minus: () -> Void
    let plus: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            Text(value)
                .font(.system(size: TextRole.label.size)).monospacedDigit()
                .foregroundStyle(tokens.textPrimary)
                .frame(width: SettingsMetric.stepperValueWidth, alignment: .trailing)
            HStack(spacing: 1) {
                half("minus", "Decrease", enabled: canDecrease, action: minus)
                half("plus", "Increase", enabled: canIncrease, action: plus)
            }
            .clipShape(RoundedRectangle(cornerRadius: tokens.metrics.radius))
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label)
        .accessibilityValue(value)
        .accessibilityAdjustableAction { direction in
            switch direction {
            case .increment: if canIncrease { plus() }
            case .decrement: if canDecrease { minus() }
            @unknown default: break
            }
        }
    }

    private func half(_ symbol: String, _ name: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        StepperHalf(symbol: symbol, enabled: enabled, action: action)
    }
}

private struct StepperHalf: View {
    let symbol: String
    let enabled: Bool
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: Metric.glyph, weight: .medium))
                .frame(width: SettingsMetric.stepperHalfWidth, height: Metric.control)
                .background(hovering && enabled ? tokens.controlHover : tokens.controlBackground)
        }
        .buttonStyle(.plain)
        .foregroundStyle(enabled ? tokens.textPrimary : tokens.textDisabled)
        .disabled(!enabled)
        .onHover { hovering = $0 }
        .motion(Motion.press, value: hovering)
    }
}
