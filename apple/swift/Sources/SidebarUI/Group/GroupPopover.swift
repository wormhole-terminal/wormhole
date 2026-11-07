// The popover a group's name opens (`sidebar/group_popover.rs`): the name in a field, the colours as a row of swatches in
// the theme's own hues (the swatch is the colour as the head's dot shows it), and the actions. ↩ keeps the name, a swatch
// colours the group at once, and ⎋ closes it. Offered only where the window can edit groups (`GroupEdits`).

import DesignSystem
import SwiftUI
import WorkspaceModel

struct GroupPopover: View {
    let group: TerminalGroup
    let edits: GroupEdits
    let close: () -> Void
    @State private var name = ""
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    private static let swatch: CGFloat = 20
    private static let width: CGFloat = 248

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            TextField("Group name", text: $name)
                .textFieldStyle(.roundedBorder)
                .focused($focused)
                .onSubmit { commit(); close() }
                .onKeyPress(.escape) { close(); return .handled }
            HStack(spacing: Space._2) {
                ForEach(FolderColor.allCases, id: \.self) { colour in
                    Button { edits.setColor(group.id, colour) } label: { swatch(colour) }
                        .buttonStyle(.plain)
                        .accessibilityLabel(colour.rawValue.capitalized)
                        .accessibilityAddTraits(group.color == colour ? .isSelected : [])
                }
            }
            HStack(spacing: Space._2) {
                Button(group.isPinned ? "Unpin" : "Pin") { edits.togglePin(group.id); close() }
                Button("Ungroup") { edits.ungroup(group.id); close() }
                Button("Close group", role: .destructive) { edits.close(group.id); close() }
            }
            .controlSize(.small)
        }
        .padding(Space._3)
        .frame(width: Self.width)
        .onAppear { name = group.name; focused = true }
        .onDisappear(perform: commit)
    }

    private func swatch(_ colour: FolderColor) -> some View {
        Circle().fill(colour.mark(tokens).color)
            .frame(width: Self.swatch, height: Self.swatch)
            .overlay { if group.color == colour { Circle().strokeBorder(tokens.textPrimary, lineWidth: tokens.metrics.focusRingWidth) } }
    }

    /// A blank name keeps the old one, as core's `Groups::rename` does.
    private func commit() {
        let typed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        if !typed.isEmpty, typed != group.name { edits.rename(group.id, typed) }
    }
}
