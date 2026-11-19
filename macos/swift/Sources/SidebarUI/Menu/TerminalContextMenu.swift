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

    private var context: SidebarContext { .away(state: state, hooks: hooks, link: link, planner: planner, send: send) }
}
