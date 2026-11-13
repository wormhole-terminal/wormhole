// The group head's one multi-step action: a terminal in the group. The core says whether the group has a folder worth asking
// about; the person's answer is two more core actions (`setGroupFolder`, `declineGroupFolder`), then the terminal opens.

import SwiftUI
import WorkspaceModel

extension SidebarView {
    /// What a keyboard menu asked for.
    func answer(_ request: SidebarLink.Request?) {
        switch request {
        case .rename(let what)?: renaming = what
        case .popover(let group)?: popover = group
        case .newInGroup(let group)?: newInGroup(group)
        case nil: break
        }
    }

    /// A group of this terminal, whose name is a field at once. The store is read again for the group the core made: this
    /// view's own `state` is the frame before it.
    func newGroup(for id: TerminalID) {
        send(.newGroup([id]))
        if let head = dropPlanner?.state.sections.first(where: { $0.rows.contains { $0.id == id } })?.head { renaming = .group(head.id) }
    }

    func newInGroup(_ group: GroupID) {
        let name = state.groups.first { $0.id == group }?.name ?? ""
        let send = send, planner = dropPlanner
        // The question first (a test instance holds it for its control folder), the terminal after the answer, whatever it was.
        Task { @MainActor in
            await GroupFolderQuestion.openTerminal(inGroup: group, name: name, planner: planner, send: send)
        }
    }
}
