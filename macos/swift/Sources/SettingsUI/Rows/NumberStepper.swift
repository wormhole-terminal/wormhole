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
        ValueStepper(
            value: spec.format(value), label: label,
            decrease: value > spec.min ? { model.set(key, .number(max(spec.min, value - spec.step))) } : nil,
            increase: value < spec.max ? { model.set(key, .number(min(spec.max, value + spec.step))) } : nil)
    }
}
