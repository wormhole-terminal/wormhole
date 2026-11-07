// A group head's context menu (`sidebar/group_popover.rs` `group_menu`): Rename…, the colours with Other colour… and No colour,
// the folder's items, then New terminal in group, Ungroup, Close group, Pin group and Move group to new window. Every item is
// one of the core's actions; the folder is chosen in the system's open panel.

import AppKit
import SwiftUI
import WorkspaceModel

struct GroupMenu: View {
    let section: SidebarSection
    let head: GroupHead
    let context: SidebarContext

    var body: some View {
        Button("Rename…") { context.openPopover(head.id) }
        Divider()
        Menu("Colour") {
            ForEach(GroupColor.allCases, id: \.self) { colour in
                Toggle(colour.rawValue.capitalized, isOn: Binding(get: { head.color == colour }, set: { _ in context.send(.setGroupColor(head.id, colour)) }))
            }
            Divider()
            // A colour picked freely is the popover's picker.
            Toggle("Other colour…", isOn: Binding(get: { head.customColor != nil }, set: { _ in context.openPopover(head.id) }))
            Toggle("No colour", isOn: Binding(get: { head.color == nil && head.customColor == nil }, set: { _ in context.send(.setGroupColor(head.id, nil)) }))
        }
        Divider()
        folder
        Divider()
        Button("New terminal in group") { context.newInGroup(head.id) }
        Button("Ungroup") { context.send(.ungroup(head.id)) }
        Button("Close group") { context.closeGroup(head.id) }
        Button(head.isPinned ? "Unpin group" : "Pin group") { context.send(.setGroupPinned(head.id, !head.isPinned)) }
        Button("Move group to new window") { context.send(.moveGroupToWindow(head.id)) }
    }

    @ViewBuilder private var folder: some View {
        if let path = head.folder { Text(path) }
        Button("Set folder…") { Self.choose(for: head.id, send: context.send) }
        if head.folderPath != nil {
            Button("Clear folder") { context.send(.setGroupFolder(head.id, nil)) }
            Toggle("Only terminals in this folder", isOn: Binding(get: { head.onlyInFolder }, set: { context.send(.setOnlyInFolder(head.id, $0)) }))
        }
    }

    /// The system's open panel for a folder; the group is pinned to what is chosen.
    @MainActor static func choose(for group: GroupID, send: @MainActor (WorkspaceAction) -> Void) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.prompt = "Choose"
        if panel.runModal() == .OK, let url = panel.url { send(.setGroupFolder(group, url.path)) }
    }
}
