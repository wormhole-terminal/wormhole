// A terminal's row menu away from the list: a tab's right-click shows the same items as the row's (`tab_bar.rs`
// `with_row_menu`), over the same facts and the same window answers. The list is not on screen then, so what only it can
// do (Rename… turns the name into its field) goes to the list through `SidebarLink`, as the keyboard's menu does.

import SwiftUI
import WorkspaceModel

public struct TerminalContextMenu: View {
    let id: TerminalID
    let state: WorkspaceState
    let appearance: SidebarAppearance
    let hooks: SidebarHooks
    let link: SidebarLink
    let planner: WorkspaceStore
    let send: @MainActor (WorkspaceAction) -> Void

    public init(
        id: TerminalID, state: WorkspaceState, appearance: SidebarAppearance, hooks: SidebarHooks, link: SidebarLink,
        planner: WorkspaceStore, send: @escaping @MainActor (WorkspaceAction) -> Void
    ) {
        (self.id, self.state, self.appearance, self.hooks, self.link, self.planner, self.send) =
            (id, state, appearance, hooks, link, planner, send)
    }

    public var body: some View {
        if let terminal = state.all.first(where: { $0.id == id }) {
            RowMenu(terminal: terminal, context: context).environment(\.sidebarLook, appearance)
        }
    }

    private var context: SidebarContext {
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
            newInGroup: { send(.newTerminalInGroup($0)) }, hooks: hooks, flash: nil,
            admits: { planner.groupAdmits($0, terminal: $1) },
            hasRoom: { planner.paneDropRefusal(dragged: $0, target: $1, zone: .right) == nil },
            popover: nil, openPopover: { _ in }, frames: RowFrames())
    }
}
