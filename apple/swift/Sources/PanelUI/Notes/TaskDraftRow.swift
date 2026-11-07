// The step being written: the same row with an empty field that has the keyboard. ↵ adds it and opens the next
// one, so a list is written in one go; esc, or leaving it empty, drops it.

import DesignSystem
import SwiftUI

struct TaskDraftRow: View {
    let add: (String) -> Void
    let discard: () -> Void
    @State private var text = ""
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: PanelMetrics.gap) {
            TaskCheckbox(checked: false)
            TextField("New next step", text: $text)
                .textFieldStyle(.plain).font(TextRole.body.font).focused($focused)
                .onSubmit {
                    let value = text
                    text = ""
                    add(value)
                    focused = true
                }
                .onExitCommand(perform: discard)
        }
        .padding(.horizontal, PanelMetrics.inset)
        .frame(minHeight: Metric.row)
        .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
        .onAppear { focused = true }
        .onChange(of: focused) { _, now in if !now && text.isEmpty { discard() } }
    }
}
