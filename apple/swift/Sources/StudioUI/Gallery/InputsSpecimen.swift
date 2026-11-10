// Form controls in their states: a field, checkboxes, switches and radios (`gallery/inputs.rs`).

import DesignSystem
import SwiftUI

struct InputsSpecimen: View {
    @State private var text = "A real field"
    @State private var checked = true
    @State private var on = true
    @State private var radio = 1
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            TextField("A field", text: $text).textFieldStyle(.roundedBorder).frame(maxWidth: 220)
            HStack(spacing: Space._4) {
                Toggle("Checkbox", isOn: .constant(false)).toggleStyle(.checkbox)
                Toggle("Checked", isOn: $checked).toggleStyle(.checkbox)
            }
            HStack(spacing: Space._4) {
                Toggle("Switch", isOn: .constant(false)).toggleStyle(.switch)
                Toggle("On", isOn: $on).toggleStyle(.switch)
            }
            Picker("", selection: $radio) { Text("Radio").tag(0); Text("Selected").tag(1) }
                .pickerStyle(.radioGroup).horizontalRadioGroupLayout().labelsHidden()
        }
        .tint(tokens.primary)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
