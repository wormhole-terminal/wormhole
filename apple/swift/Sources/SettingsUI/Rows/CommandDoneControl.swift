// "A command finished": a switch and, before it, how long a command has to run before it counts, stepped through
// the schema's list (`notify_command_after`, 10 s … 5 min).

import SettingsModel
import SwiftUI

struct CommandDoneControl: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry
    let stepKey: String
    let steps: [Int]

    var body: some View {
        let seconds = Int(model.value(stepKey).number ?? Double(steps[0]))
        // A hand-written value rounds up to the next step, as `after_step` does.
        let index = steps.firstIndex { $0 >= seconds } ?? steps.count - 1
        HStack(spacing: 12) {
            FlatStepper(
                value: Self.words(steps[index]), label: entry.label, canDecrease: index > 0,
                canIncrease: index < steps.count - 1,
                minus: { model.set(stepKey, .number(Double(steps[index - 1]))) },
                plus: { model.set(stepKey, .number(Double(steps[index + 1]))) })
            FlatSwitch(isOn: model.value(entry.keys[0]).bool ?? false, label: entry.label) {
                model.set(entry.keys[0], .bool($0))
            }
        }
    }

    static func words(_ seconds: Int) -> String {
        seconds >= 60
            ? String(localized: "settings-notifications-after-minutes", defaultValue: "\(seconds / 60) min")
            : String(localized: "settings-notifications-after-seconds", defaultValue: "\(seconds) s")
    }
}
