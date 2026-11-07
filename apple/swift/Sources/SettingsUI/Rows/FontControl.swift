// A font row (`font_picker.rs`): the family in a select (an empty choice follows another slot and says so in
// its face) and the size as a stepper, in whole points within the slot's range. The families are the host's.

import DesignSystem
import SettingsModel
import SwiftUI

struct FontControl: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry
    let slot: ControlSpec.FontSlot

    var body: some View {
        let size = model.value(entry.keys[1]).number ?? slot.min
        HStack(spacing: Space._3) {
            FlatSelect(options: families, selected: model.value(entry.keys[0]), label: entry.label) {
                model.set(entry.keys[0], $0)
            }
            FlatStepper(
                value: "\(Int(size)) pt", label: entry.label, canDecrease: size > slot.min, canIncrease: size < slot.max,
                minus: { model.set(entry.keys[1], .number(max(slot.min, size - 1))) },
                plus: { model.set(entry.keys[1], .number(min(slot.max, size + 1))) })
        }
    }

    private var families: [SettingOption] {
        let inherit = slot.inherit.map { [SettingOption(value: "", label: $0)] } ?? []
        return inherit + model.host.fontFamilies().map { SettingOption(value: .string($0), label: $0) }
    }
}
