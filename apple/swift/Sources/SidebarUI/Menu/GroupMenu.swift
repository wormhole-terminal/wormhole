// A group head's context menu (`sidebar/group_popover.rs` `group_menu`): fold or open, a terminal in the group, Rename…, the
// colours (where the window can set them: `GroupEdits`), Pin group, Ungroup and Close group.

import SwiftUI
import WorkspaceModel

struct GroupMenu: View {
    let section: SidebarSection
    let head: GroupHead
    let context: SidebarContext

    var body: some View {
        Button(head.isFolded ? "Open group" : "Fold group") { context.send(.toggleGroup(head.id)) }
        Button("New terminal in group") { context.send(.newTerminalInGroup(head.id)) }
        Divider()
        Button("Rename…") { context.openPopover(head.id) }
        if let edits = context.groupEdits {
            Menu("Colour") {
                ForEach(GroupColor.allCases, id: \.self) { colour in
                    Toggle(colour.rawValue.capitalized, isOn: Binding(get: { head.color == colour }, set: { _ in edits.setColor(head.id, colour) }))
                }
                Divider()
                Button("No colour") { edits.setColor(head.id, nil) }.disabled(head.color == nil)
            }
        }
        Divider()
        Button(head.isPinned ? "Unpin group" : "Pin group") { context.send(.setGroupPinned(head.id, !head.isPinned)) }
        Button("Ungroup") { context.send(.ungroup(head.id)) }
        // A folded group lists no rows to end; the core has no action for the whole group.
        Button("Close group") { section.rows.forEach { context.send(.close($0.id)) } }.disabled(section.rows.isEmpty)
    }
}
