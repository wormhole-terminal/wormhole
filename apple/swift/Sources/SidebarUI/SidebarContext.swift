// What every row and head of one frame is given besides its own facts: who is selected, the groups a terminal can be
// moved to, how to send an action, and the two edits in flight (a name being typed, a drag). One value instead of a
// long argument list; it holds no state of its own, the list's view does.

import SwiftUI
import WorkspaceModel

struct SidebarContext {
    let selected: TerminalID?
    /// Every group, in list order, for a terminal's "Add terminal to group" and the group a terminal is in (a folded
    /// group lists no members: its rows are folded away).
    let groups: [GroupChoice]
    let send: @MainActor (WorkspaceAction) -> Void
    /// The terminal whose name is a field, and the closure that starts and ends a rename (nil ends it, keeping the
    /// old name).
    let renaming: TerminalID?
    let rename: @MainActor (TerminalID?) -> Void
    /// How many status changes each terminal has had since the list appeared: what plays the wash.
    let shifts: [TerminalID: Int]
    let drag: DragHooks
    /// Every terminal, folded ones too, for "Show beside…" and the moves a menu offers.
    let everyTerminal: [TerminalSummary]
    /// A new terminal in a group: first the question about its folder, when the core has one to offer.
    let newInGroup: @MainActor (GroupID) -> Void
    /// Close a group: the window asks first when something works in it.
    let closeGroup: @MainActor (GroupID) -> Void
    /// The group whose popover is open.
    let popover: GroupID?
    let openPopover: @MainActor (GroupID?) -> Void

    struct GroupChoice: Identifiable {
        let id: GroupID
        let name: String
        let members: [TerminalID]
    }

    func group(of terminal: TerminalID) -> GroupChoice? { groups.first { $0.members.contains(terminal) } }
}
