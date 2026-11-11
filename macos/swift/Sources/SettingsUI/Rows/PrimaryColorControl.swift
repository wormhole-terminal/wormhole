// "Use the theme's colour": a switch; off, the colour picked is the primary (`primary_color`, a hex).

import DesignSystem
import SettingsModel
import SwiftUI

struct PrimaryColorControl: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry

    var body: some View {
        HStack(spacing: Space._3) {
            if !(model.value(entry.keys[0]).bool ?? true) {
                ColorPicker(
                    entry.label,
                    selection: Binding(
                        get: { Color(hex: model.value(entry.keys[1]).string ?? "") ?? .accentColor },
                        set: { model.set(entry.keys[1], .string($0.hexString)) }),
                    supportsOpacity: false
                )
                .labelsHidden()
            }
            SettingToggle(label: entry.label, isOn: model.value(entry.keys[0]).bool ?? true) {
                model.set(entry.keys[0], .bool($0))
            }
        }
    }
}
