// "Save theme" as a split button: the click saves (⌘S), and the arrow opens the other ways to write the theme
// out: Export JSON, Copy JSON and Use in this project (`title.rs`'s save menu; Duplicate is in the theme menu).

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
            .keyboardShortcut("s", modifiers: .command).disabled(!session.document.isDirty && session.document.subject.url != nil)
            Menu {
                Button(StudioWords.exportJSON) { Task { await session.exportJSON() } }
                Button(StudioWords.copyJSON) { session.copyJSON() }
                Divider()
                Button(StudioWords.useInProject) { session.useInProject() }
            } label: { EmptyView() }
            .menuIndicator(.visible).menuStyle(.borderlessButton).fixedSize()
            .accessibilityLabel(StudioWords.saveMore)
        }
        .buttonStyle(.bordered).controlSize(.small).tint(session.document.isDirty ? tokens.primary : nil)
    }
}
