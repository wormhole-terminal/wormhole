// The one text field of the settings pages (`style::text_field`): the theme's input fill and border, the focus
// border while typing. It applies what was typed when it loses focus or takes Return, as the GPUI fields do. A plain
// `TextField`: the system gives the caret, selection, IME and VoiceOver.

import DesignSystem
import SwiftUI

struct InputField: View {
    let placeholder: String
    @Binding var text: String
    var width: CGFloat = SettingsMetric.fieldWidth
    let onCommit: () -> Void
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        TextField(text: $text, prompt: Text(placeholder).foregroundStyle(tokens.textFaint)) { Text(placeholder) }
            .textFieldStyle(.plain).font(.system(size: TextRole.body.size))
            .focused($focused)
            .onSubmit(onCommit)
            .onChange(of: focused) { _, now in if !now { onCommit() } }
            .padding(.horizontal, Space._3)
            .frame(width: width, height: Metric.control)
            .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .overlay {
                RoundedRectangle(cornerRadius: tokens.metrics.radius)
                    .strokeBorder(focused ? tokens.inputBorderFocus : tokens.inputBorder, lineWidth: tokens.metrics.borderWidth)
            }
    }
}
