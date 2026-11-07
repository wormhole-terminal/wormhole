// What every row and head of one frame is given besides its own facts: who is selected, the groups a terminal can be
// moved to, how to send an action, and the two edits in flight (a name being typed, a drag). One value instead of a
// long argument list; it holds no state of its own, the list's view does.

import SwiftUI
import WorkspaceModel

struct SidebarContext {
    let selected: TerminalID?
    /// Every group, in list order, for a terminal's "Add terminal to group" and the group a terminal is in.
    let groups: [GroupChoice]
    let send: @MainActor (WorkspaceAction) -> Void
    /// The terminal whose name is a field, and the closure that starts and ends a rename (nil ends it, keeping the
    /// old name).
    let renaming: TerminalID?
    let rename: @MainActor (TerminalID?) -> Void
    /// How many status changes each terminal has had since the list appeared: what plays the wash.
    let shifts: [TerminalID: Int]
    let drag: DragHooks
    /// What the window can do to a group beyond the model's actions, and the group whose popover is open.
    let groupEdits: GroupEdits?
    let popover: String?
    let openPopover: @MainActor (String?) -> Void
    /// The ids the next terminal and the next group get (`WorkspaceState.addTerminal`, `addGroup`), so a terminal made in
    /// one action can be put in a group by the next.
    let nextTerminalID: TerminalID
    let nextGroupID: String

    struct GroupChoice: Identifiable {
        let id: String
        let name: String
        let members: [TerminalID]
    }

    func group(of terminal: TerminalID) -> GroupChoice? { groups.first { $0.members.contains(terminal) } }
}
