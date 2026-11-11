// The one text field of the settings pages: the system's rounded field. It applies what was typed when it loses focus or
// takes Return, as the GPUI fields do.

import DesignSystem
import SwiftUI

struct InputField: View {
    let placeholder: String
    @Binding var text: String
    var width: CGFloat = SettingsMetric.fieldWidth
    var frameId: String?
    let onCommit: () -> Void
    @FocusState private var focused: Bool

    var body: some View {
        TextField(placeholder, text: $text)
            .textFieldStyle(.roundedBorder)
            .focused($focused)
            .onSubmit(onCommit)
            .onChange(of: focused) { _, now in if !now { onCommit() } }
            .frame(width: width)
            .marked("input")
            .recordFrame(frameId)
    }
}
