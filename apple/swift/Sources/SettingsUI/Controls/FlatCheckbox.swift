// A labelled checkbox (`style::checkbox(...).in_form`): the system checkbox, so the keyboard and VoiceOver are
// the system's, tinted by the theme.

import DesignSystem
import SwiftUI

struct FlatCheckbox: View {
    let title: String
    let isOn: Bool
    let onChange: (Bool) -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        Toggle(title, isOn: Binding(get: { isOn }, set: onChange))
            .toggleStyle(.checkbox)
            .font(.system(size: TextRole.label.size))
            .foregroundStyle(tokens.textPrimary)
            .tint(tokens.checkboxChecked)
    }
}
