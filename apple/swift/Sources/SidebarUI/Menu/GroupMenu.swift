// A group head's context menu (`sidebar/group_popover.rs` `group_menu`): Rename…, the colours with Other colour… and No colour,
// the folder's items, then New terminal in group, Ungroup, Close group, Pin group and Move group to new window. Every item is
// one of the core's actions; the folder is chosen in the system's open panel.

import AppKit
import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

struct GroupMenu: View {
    let section: SidebarSection
    let head: GroupHead
    let context: SidebarContext
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(Words.text("sidebar-menu-rename", fallback: "Rename…")) { context.openPopover(head.id) }
        Divider()
        // The colours as one row of dots, as GPUI's menu draws them (`group_popover.rs`): each picks and closes the menu.
        ControlGroup {
            ForEach(GroupColor.allCases, id: \.self) { colour in
                Button { context.send(.setGroupColor(head.id, colour)) } label: {
                    Label(colour.rawValue.capitalized, systemImage: head.color == colour ? "checkmark.circle.fill" : "circle.fill")
                }
                .tint(colour.mark(tokens).color)
            }
        }
        .controlGroupStyle(.palette)
        // A colour picked freely is the popover's picker.
        Toggle(Words.text("sidebar-menu-group-colour-other", fallback: "Other colour…"), isOn: Binding(get: { head.customColor != nil }, set: { _ in context.openPopover(head.id) }))
        Toggle(Words.text("sidebar-menu-group-colour-none", fallback: "No colour"), isOn: Binding(get: { head.color == nil && head.customColor == nil }, set: { _ in context.send(.setGroupColor(head.id, nil)) }))
        Divider()
        folder
        Divider()
        Button(Words.text("sidebar-menu-group-new-terminal", fallback: "New terminal in group")) { context.newInGroup(head.id) }
        Button(Words.text("sidebar-menu-group-ungroup", fallback: "Ungroup")) { context.send(.ungroup(head.id)) }
        Button(Words.text("sidebar-menu-group-close", fallback: "Close group")) { context.closeGroup(head.id) }
        Button(head.isPinned ? Words.text("sidebar-menu-group-unpin", fallback: "Unpin group") : Words.text("sidebar-menu-group-pin", fallback: "Pin group")) { context.send(.setGroupPinned(head.id, !head.isPinned)) }
        Button(Words.text("sidebar-menu-group-move-to-window", fallback: "Move group to new window")) { context.send(.moveGroupToWindow(head.id)) }
    }

    @ViewBuilder private var folder: some View {
        if let path = head.folder { Text(path) }
        Button(Words.text("sidebar-menu-group-set-folder", fallback: "Set folder…")) { Self.choose(for: head.id, send: context.send) }
        if head.folderPath != nil {
            Button(Words.text("sidebar-menu-group-clear-folder", fallback: "Clear folder")) { context.send(.setGroupFolder(head.id, nil)) }
            Toggle(Words.text("sidebar-group-only-in-folder", fallback: "Only terminals in this folder"), isOn: Binding(get: { head.onlyInFolder }, set: { context.send(.setOnlyInFolder(head.id, $0)) }))
        }
    }

    /// The system's open panel for a folder; the group is pinned to what is chosen.
    @MainActor static func choose(for group: GroupID, send: @MainActor (WorkspaceAction) -> Void) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.prompt = Words.text("files-choose-prompt", fallback: "Choose")
        if panel.runModal() == .OK, let url = panel.url { send(.setGroupFolder(group, url.path)) }
    }
}
