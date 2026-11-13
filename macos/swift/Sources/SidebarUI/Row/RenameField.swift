// A terminal's (or a group's) name as a field in its row or head (`rename.rs`): the whole name selected, ↩ or a click elsewhere keeps what was
// typed, ⎋ keeps the old name, and an empty name goes back to what runs in the terminal.

import AppKit
import DesignSystem
import SwiftUI
import WordsModel

/// What a name field reports: the typed name, or nil to leave the old one.
struct RowRename {
    let finish: @MainActor (String?) -> Void
}

struct RenameField: View {
    let name: String
    /// The field's font; nil is the row's own (body, medium) in the window's theme.
    var font: Font?
    let rename: RowRename
    @State private var text = ""
    @State private var done = false
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        TextField("", text: $text)
            .textFieldStyle(.plain)
            .font(font ?? tokens.font(.body).weight(.medium))
            .foregroundStyle(tokens.textPrimary)
            .focused($focused)
            .onSubmit { finish(text) }
            .onKeyPress(.escape) { finish(nil); return .handled }
            .onChange(of: focused) { _, now in if !now { finish(text) } }
            .onAppear {
                text = name
                focused = true
                // The whole name is selected, so typing replaces it.
                DispatchQueue.main.async { NSApp.sendAction(#selector(NSText.selectAll(_:)), to: nil, from: nil) }
            }
            .accessibilityLabel(Words.text("sidebar-rename-field", fallback: "Name"))
    }

    /// The field ends once, whichever of ↩, ⎋ and losing the keyboard comes first.
    private func finish(_ typed: String?) {
        guard !done else { return }
        done = true
        rename.finish(typed)
    }
}
