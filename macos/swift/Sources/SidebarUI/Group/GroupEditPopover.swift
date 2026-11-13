// A group's popover away from the list: the name on a group's card in the tab bar's switcher opens the same popover the
// name on its head does (`switcher.rs` `on_label`), over the same facts and the same window answers.

import SwiftUI
import WorkspaceModel

public struct GroupEditPopover: View {
    let group: GroupID
    let state: WorkspaceState
    let appearance: SidebarAppearance
    let hooks: SidebarHooks
    let link: SidebarLink
    let planner: WorkspaceStore
    let send: @MainActor (WorkspaceAction) -> Void
    let close: () -> Void

    public init(
        group: GroupID, state: WorkspaceState, appearance: SidebarAppearance, hooks: SidebarHooks, link: SidebarLink,
        planner: WorkspaceStore, send: @escaping @MainActor (WorkspaceAction) -> Void, close: @escaping () -> Void
    ) {
        (self.group, self.state, self.appearance, self.hooks, self.link, self.planner, self.send, self.close) =
            (group, state, appearance, hooks, link, planner, send, close)
    }

    public var body: some View {
        if let head = state.groups.first(where: { $0.id == group }) {
            GroupPopover(
                head: head, rows: [], context: .away(state: state, hooks: hooks, link: link, planner: planner, send: send), close: close)
                .environment(\.sidebarLook, appearance)
        }
    }
}
