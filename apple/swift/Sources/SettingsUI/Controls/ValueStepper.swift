// The system's stepper with its value beside it. `nil` for a direction means the end of the range (that arrow dims).

import DesignSystem
import SwiftUI

struct ValueStepper: View {
    let value: String
    let label: String
    let decrease: (() -> Void)?
    let increase: (() -> Void)?
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            Text(value).monospacedDigit().foregroundStyle(tokens.textPrimary)
                .frame(width: SettingsMetric.stepperValueWidth, alignment: .trailing)
            Stepper(label, onIncrement: increase, onDecrement: decrease).labelsHidden()
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(label)
        .accessibilityValue(value)
        .accessibilityAdjustableAction { direction in
            if direction == .increment { increase?() } else { decrease?() }
        }
    }
}
