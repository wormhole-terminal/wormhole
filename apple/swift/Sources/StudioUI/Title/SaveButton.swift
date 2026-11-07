// "Save theme" as a split button: the click saves (⌘S), and the arrow opens the other ways to write the theme
// out: a copy (⇧⌘S), Export JSON, Copy JSON (`SaveAction` of the GPUI app's title row).

import AppKit
import DesignSystem
import StudioModel
import SwiftUI

struct SaveButton: View {
    @Bindable var session: StudioSession
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: 0) {
            Button { Task { await session.save() } } label: {
                Text(StudioWords.save).font(TextRole.body.font)
            }
            .keyboardShortcut("s", modifiers: .command)
            Menu {
                Button(StudioWords.saveCopy) { session.saveCopy() }.keyboardShortcut("s", modifiers: [.command, .shift])
                Divider()
                Button(StudioWords.exportJSON) { session.exportJSON() }
                Button(StudioWords.copyJSON) { copy() }
            } label: { EmptyView() }
            .menuIndicator(.visible).menuStyle(.borderlessButton).fixedSize()
            .accessibilityLabel("More ways to save")
        }
        .buttonStyle(.borderedProminent).controlSize(.small).tint(tokens.primary)
    }

    private func copy() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(session.document.exportJSON, forType: .string)
        session.document.copiedJSON()
    }
}
