// A stepper over a number key: the value in words, the range from the schema (the backend sanitises again).

import SettingsModel
import SwiftUI

struct NumberStepper: View {
    let model: SettingsModel
    let key: String
    let label: String
    let spec: ControlSpec.Stepper

    var body: some View {
        let value = model.value(key).number ?? spec.min
        FlatStepper(
            value: spec.format(value), label: label, canDecrease: value > spec.min, canIncrease: value < spec.max,
            minus: { model.set(key, .number(max(spec.min, value - spec.step))) },
            plus: { model.set(key, .number(min(spec.max, value + spec.step))) })
    }
}
