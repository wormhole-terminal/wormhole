// A select: the value in a control-fill face (`SelectFace`) with a native menu under it (`style` select on
// a card). The menu is `Menu` so keyboard, type-ahead and VoiceOver are the system's. `onOpen` tells the page the
// row got attention.

import DesignSystem
import SettingsModel
import SwiftUI

struct FlatSelect: View {
    let options: [SettingOption]
    let selected: JSONValue
    let label: String
    var width: CGFloat = SettingsMetric.controlWidth
    let onSelect: (JSONValue) -> Void

    var body: some View {
        Menu {
            ForEach(options) { option in
                Button {
                    onSelect(option.value)
                } label: {
                    if option.value == selected { Label(option.label, systemImage: "checkmark") } else { Text(option.label) }
                }
            }
        } label: {
            SelectFace(width: width) {
                Text(options.first { $0.value == selected }?.label ?? selected.string ?? "")
                    .font(.system(size: TextRole.label.size)).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
        .accessibilityLabel(label)
    }
}
