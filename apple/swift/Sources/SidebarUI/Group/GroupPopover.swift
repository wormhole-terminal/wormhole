// The popover a group's name opens (`sidebar/group_popover.rs`): the name in a field, the colours as a row of swatches in
// the theme's own hues (the swatch is the colour as the head's dot shows it) led by the empty ring for none, a picker for
// any other, the folder's items, and the actions. ↩ keeps the name, a swatch or the picker colours the group at once, and
// ⎋ closes it; closing it gives the keyboard back to the terminal (the list does that, as it owns the popover).

import DesignSystem
import SwiftUI
import WordsModel
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
        VStack(alignment: .leading, spacing: Space._2) {
            TextField(Words.text("sidebar-popover-group-name", fallback: "Group name"), text: $name)
                .textFieldStyle(.roundedBorder)
                .focused($focused)
                .onSubmit { commit(); close() }
                .onKeyPress(.escape) { close(); return .handled }
            HStack(spacing: Space._2) {
                noColour
                ForEach(GroupColor.allCases, id: \.self) { colour in
                    Button { context.send(.setGroupColor(head.id, colour)) } label: { swatch(colour) }
                        .buttonStyle(.plain)
                        .accessibilityLabel(colour.rawValue.capitalized)
                        .accessibilityAddTraits(head.color == colour ? .isSelected : [])
                }
                ColorPicker(Words.text("sidebar-popover-colour-other", fallback: "Other colour"), selection: custom, supportsOpacity: false).labelsHidden()
            }
            Divider()
            GroupFolderItems(head: head, context: context, close: close)
            Divider()
            item(Words.text("sidebar-menu-group-new-terminal", fallback: "New terminal in group")) { context.newInGroup(head.id) }
            item(Words.text("sidebar-menu-group-ungroup", fallback: "Ungroup")) { context.send(.ungroup(head.id)) }
            item(Words.text("sidebar-menu-group-close", fallback: "Close group")) { context.closeGroup(head.id) }
            item(head.isPinned ? Words.text("sidebar-menu-group-unpin", fallback: "Unpin group") : Words.text("sidebar-menu-group-pin", fallback: "Pin group")) {
                context.send(.setGroupPinned(head.id, !head.isPinned))
            }
            item(Words.text("sidebar-menu-group-move-to-window", fallback: "Move group to new window")) { context.send(.moveGroupToWindow(head.id)) }
        }
        .padding(Space._3)
        .frame(width: Self.width)
        .onAppear { name = head.name; focused = true }
        .onDisappear(perform: commit)
    }

    /// An action of the popover: it runs, and the popover closes.
    private func item(_ title: String, _ action: @escaping () -> Void) -> some View {
        PopoverItem(title: title) { action(); close() }
    }

    /// The empty ring first among the swatches: it takes the colour away, and is ringed while the group has none.
    private var noColour: some View {
        Button { context.send(.setGroupColor(head.id, nil)) } label: {
            Circle().strokeBorder(tokens.textMuted, lineWidth: tokens.metrics.borderWidth)
                .frame(width: Self.swatch, height: Self.swatch)
                .overlay { if head.color == nil && head.customColor == nil { Circle().strokeBorder(tokens.textPrimary, lineWidth: tokens.metrics.focusRingWidth) } }
        }
        .buttonStyle(.plain)
        .accessibilityLabel(Words.text("sidebar-menu-group-colour-none", fallback: "No colour"))
        .accessibilityAddTraits(head.color == nil && head.customColor == nil ? .isSelected : [])
    }

    /// A colour picked freely: the group's own when it has one, else the theme's grey to start from; each pick is the core's
    /// `setGroupCustomColor`, so the group follows the picker as it moves.
    private var custom: Binding<Color> {
        Binding(
            get: { head.customColor.flatMap(Color.init(hex:)) ?? tokens.textMuted },
            set: { colour in
                guard let (red, green, blue) = colour.rgb8 else { return }
                context.send(.setGroupCustomColor(head.id, red: red, green: green, blue: blue))
            })
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
