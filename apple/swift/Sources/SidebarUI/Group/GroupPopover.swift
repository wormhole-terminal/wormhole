// The popover a group's name opens (`sidebar/group_popover.rs`): the name in a field, the colours as a row of swatches in
// the theme's own hues (the swatch is the colour as the head's dot shows it), and the actions. ↩ keeps the name, a swatch
// colours the group at once (where the window can: `GroupEdits`), and ⎋ closes it.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct GroupPopover: View {
    let head: GroupHead
    let rows: [SidebarRow]
    let context: SidebarContext
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
            if let edits = context.groupEdits {
                HStack(spacing: Space._2) {
                    ForEach(GroupColor.allCases, id: \.self) { colour in
                        Button { edits.setColor(head.id, colour) } label: { swatch(colour) }
                            .buttonStyle(.plain)
                            .accessibilityLabel(colour.rawValue.capitalized)
                            .accessibilityAddTraits(head.color == colour ? .isSelected : [])
                    }
                }
            }
            HStack(spacing: Space._2) {
                Button(head.isPinned ? "Unpin" : "Pin") { context.send(.setGroupPinned(head.id, !head.isPinned)); close() }
                Button("Ungroup") { context.send(.ungroup(head.id)); close() }
                Button("Close group", role: .destructive) { rows.forEach { context.send(.close($0.id)) }; close() }
                    .disabled(rows.isEmpty)
            }
            .controlSize(.small)
        }
        .padding(Space._3)
        .frame(width: Self.width)
        .onAppear { name = head.name; focused = true }
        .onDisappear(perform: commit)
    }

    private func swatch(_ colour: GroupColor) -> some View {
        Circle().fill(colour.mark(tokens).color)
            .frame(width: Self.swatch, height: Self.swatch)
            .overlay { if head.color == colour { Circle().strokeBorder(tokens.textPrimary, lineWidth: tokens.metrics.focusRingWidth) } }
    }

    /// A blank name keeps the old one, as core's `Groups::rename` does.
    private func commit() {
        let typed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        if !typed.isEmpty, typed != head.name { context.send(.renameGroup(head.id, typed)) }
    }
}
