// The name, edited in place in the title row: Return gives it (and saves a copy that was waiting for one), Esc
// leaves it as it was, and so does a click elsewhere (`studio/title.rs` rename).

import DesignSystem
import StudioModel
import SwiftUI

struct NameField: View {
    @Bindable var session: StudioSession
    @State private var text = ""
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        TextField("", text: $text)
            .textFieldStyle(.roundedBorder).font(TextRole.body.font).focused($focused)
            .frame(width: StudioMetrics.nameFieldWidth)
            .onSubmit { Task { await session.finishRenaming(text) } }
            .onExitCommand { session.renaming = false }
            .onChange(of: focused) { _, now in if !now, session.renaming { session.renaming = false } }
            .onAppear { text = session.document.name; focused = true }
            .accessibilityLabel(StudioWords.nameField)
    }
}
