// A button that puts `text` on the pasteboard and says so: its symbol turns into a check for a moment (replaced, not
// swapped, and cut when Reduce Motion is on).

import AppKit
import DesignSystem
import SwiftUI

struct CopyButton: View {
    let title: String
    let text: String
    @State private var copied = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Button {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
            copied = true
            Task {
                // one-shot: the check goes back to the copy symbol after a moment
                try? await Task.sleep(for: .seconds(1.5))
                copied = false
            }
        } label: {
            Label(title, systemImage: (copied ? Icon.check : Icon.copy).rawValue)
                .contentTransition(reduceMotion ? .identity : .symbolEffect(.replace))
        }
        .buttonStyle(.bordered)
    }
}
