// The schema's choices as the system's pop-up button (many) or segmented control (a few, `segmented`). A value the list
// lacks (a hand-written one) is shown as written rather than as an empty pop-up.

import SettingsModel
import SwiftUI

struct OptionPicker: View {
    let options: [SettingOption]
    let selected: JSONValue
    let label: String
    var segmented = false
    var width: CGFloat = SettingsMetric.controlWidth
    let onSelect: (JSONValue) -> Void

    var body: some View {
        let known = options.contains { $0.value == selected } || selected == .null
        let shown = known ? options : options + [SettingOption(value: selected, label: selected.string ?? "")]
        let picker = Picker(label, selection: Binding(get: { selected }, set: onSelect)) {
            ForEach(shown) { option in
                if segmented, let icon = option.icon { Label(option.label, systemImage: icon).tag(option.value) } else { Text(option.label).tag(option.value) }
            }
        }
        .labelsHidden()
        if segmented { picker.pickerStyle(.segmented).fixedSize() } else { picker.pickerStyle(.menu).frame(width: width) }
    }
}
