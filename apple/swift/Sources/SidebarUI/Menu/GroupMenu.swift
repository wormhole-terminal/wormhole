// A group head's context menu (`sidebar/group_popover.rs` `group_menu`): fold or open, a terminal in the group, and, when
// the window can edit groups (`GroupEdits`), Rename…, the colours, Pin group, Ungroup and Close group. The model carries
// no action for those edits yet, so a window without them shows only what it can do.

import SwiftUI
import WorkspaceModel

struct GroupMenu: View {
    let section: SidebarSection
    let context: SidebarContext

    var body: some View {
        Button(section.isCollapsed ? "Open group" : "Fold group") { context.send(.toggleGroup(section.id)) }
        Button("New terminal in group") {
            context.send(.newTerminal)
            context.send(.addToGroup(context.nextTerminalID, section.id))
        }
        if let edits = context.groupEdits, let group = section.group {
            Divider()
            Button("Rename…") { context.openPopover(group.id) }
            Menu("Colour") {
                ForEach(FolderColor.allCases, id: \.self) { colour in
                    Toggle(colour.rawValue.capitalized, isOn: Binding(get: { group.color == colour }, set: { _ in edits.setColor(group.id, colour) }))
                }
                Divider()
                Button("No colour") { edits.setColor(group.id, nil) }.disabled(group.color == nil)
            }
            Divider()
            Button(group.isPinned ? "Unpin group" : "Pin group") { edits.togglePin(group.id) }
            Button("Ungroup") { edits.ungroup(group.id) }
            Button("Close group") { edits.close(group.id) }
        }
    }
}
