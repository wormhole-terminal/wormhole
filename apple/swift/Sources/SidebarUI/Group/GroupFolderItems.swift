// The group's folder as the popover lists it above the actions (`group_popover.rs` `folder_items`): the folder as a greyed
// line (it is not a choice) when there is one, Set folder…, and with a folder Clear folder and the fence's switch. The group
// menu has the same items as menu entries (`PlatformModel.GroupMenu`); the folder is chosen in the system's open panel.

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

struct GroupFolderItems: View {
    let head: GroupHead
    let context: SidebarContext
    let close: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let path = head.folder {
            Text(path).font(TextRole.body.font).foregroundStyle(tokens.textDisabled).lineLimit(1).truncationMode(.head)
                .padding(.horizontal, Space._2)
        }
        PopoverItem(title: Words.text("sidebar-menu-group-set-folder", fallback: "Set folder…")) { GroupFolderPanel.choose(for: head.id, send: context.send); close() }
        if head.folderPath != nil {
            PopoverItem(title: Words.text("sidebar-menu-group-clear-folder", fallback: "Clear folder")) { context.send(.setGroupFolder(head.id, nil)); close() }
            PopoverItem(title: Words.text("sidebar-group-only-in-folder", fallback: "Only terminals in this folder"), checked: head.onlyInFolder) {
                context.send(.setOnlyInFolder(head.id, !head.onlyInFolder))
                close()
            }
        }
    }
}

/// One action in the popover: a full-width line that fills under the pointer, ticked when it is a switch that is on.
struct PopoverItem: View {
    let title: String
    var checked = false
    let action: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._2) {
                Text(title).font(TextRole.body.font).foregroundStyle(tokens.textPrimary)
                Spacer(minLength: 0)
                if checked { LucideView(icon: .check, size: Metric.glyph).foregroundStyle(tokens.textPrimary) }
            }
            .padding(.horizontal, Space._2).padding(.vertical, Space._1)
            .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(checked ? .isSelected : [])
        .onHover { hovering = $0 }
    }
}
