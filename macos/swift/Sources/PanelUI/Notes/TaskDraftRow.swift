// The step being written: the same row with an empty field that has the keyboard. ↵ adds it and opens the next
// one below, so a list is written in one go; on an empty field ↵ ends the writing, and so does esc (which keeps
// what is typed). Leaving the field with words in it keeps them too.

import DesignSystem
import SwiftUI

struct TaskDraftRow: View {
    /// Adds the words (the owner opens the next draft below them).
    let add: (String) -> Void
    let discard: () -> Void
    /// The keys go back to the terminal.
    let leave: () -> Void
    @State private var text = ""
    /// ↵ added the words and the next draft took this one's place: losing the keys then ends nothing.
    @State private var handedOver = false
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: PanelMetrics.gap) {
            TaskCheckbox(checked: false)
            TextField(panelText("steps-placeholder", "Next step"), text: $text)
                .textFieldStyle(.plain).font(TextRole.body.font).focused($focused)
                .onSubmit {
                    if text.trimmingCharacters(in: .whitespaces).isEmpty {
                        finish()
                        return leave()
                    }
                    let value = text
                    text = ""
                    handedOver = true
                    add(value)
                }
                .onExitCommand { finish(); leave() }
        }
        .padding(.horizontal, PanelMetrics.inset)
        .frame(minHeight: Metric.row)
        .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
        .onAppear { focused = true }
        .onChange(of: focused) { _, now in if !now && !handedOver { finish() } }
    }

    /// The draft ends: what is typed is kept, an empty one is dropped.
    private func finish() {
        if !text.trimmingCharacters(in: .whitespaces).isEmpty { add(text); text = "" }
        discard()
    }
}
