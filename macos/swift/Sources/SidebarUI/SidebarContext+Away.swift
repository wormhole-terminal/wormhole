// The list's context for a view that stands away from the list (a tab's menu, a switcher card's popover): the same facts
// and the same window answers, with nothing of the list's own edits (no drag, no cursor, no flash). What only the list
// can do (a name turning into its field) is asked of it through `SidebarLink`.

import SwiftUI
import WorkspaceModel

extension SidebarContext {
    @MainActor static func away(
        state: WorkspaceState, hooks: SidebarHooks, link: SidebarLink, planner: WorkspaceStore,
        send: @escaping @MainActor (WorkspaceAction) -> Void
    ) -> SidebarContext {
        SidebarContext(
            selected: state.selected,
            groups: state.groups.map { head in
                SidebarContext.GroupChoice(id: head.id, name: head.name, members: state.all.filter { $0.group == head.id }.map(\.id))
            },
            send: send, open: { send(.select($0)) }, renaming: nil, rename: { if let what = $0 { link.ask(.rename(what)) } },
            shifts: [:], drag: .none, everyTerminal: state.all,
            newGroup: { id in
                send(.newGroup([id]))
                if let head = planner.state.sections.first(where: { $0.rows.contains { $0.id == id } })?.head { link.ask(.rename(.group(head.id))) }
            },
            newInGroup: { group in
                // First the question about the group's folder, when the core has one to offer (as the head's + asks).
                let name = state.groups.first { $0.id == group }?.name ?? ""
                Task { @MainActor in
                    await GroupFolderQuestion.openTerminal(inGroup: group, name: name, planner: planner, send: send)
                }
            },
            hooks: hooks, flash: nil,
            admits: { planner.groupAdmits($0, terminal: $1) },
            hasRoom: { planner.paneDropRefusal(dragged: $0, target: $1, zone: .right) == nil },
            popover: nil, openPopover: { _ in }, frames: RowFrames())
    }
}
