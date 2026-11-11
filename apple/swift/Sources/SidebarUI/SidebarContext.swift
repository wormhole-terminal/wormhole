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
    /// A click or ↩ on a row: the terminal is shown and has the keyboard.
    let open: @MainActor (TerminalID) -> Void
    /// The terminal or group whose name is a field, and the closure that starts and ends a rename (nil ends it, keeping
    /// the old name).
    let renaming: SidebarRename?
    let rename: @MainActor (SidebarRename?) -> Void
    /// How many status changes each terminal has had since the list appeared: what plays the wash.
    let shifts: [TerminalID: Int]
    let drag: DragHooks
    /// Every terminal, folded ones too, for "Show beside…" and the moves a menu offers.
    let everyTerminal: [TerminalSummary]
    /// "Add terminal to new group": the group's name is a field at once, to name it (`group_terminal`).
    let newGroup: @MainActor (TerminalID) -> Void
    /// A new terminal in a group: first the question about its folder, when the core has one to offer.
    let newInGroup: @MainActor (GroupID) -> Void
    /// What the window answers (close questions, the keyboard); see `SidebarHooks`.
    let hooks: SidebarHooks
    /// The row that was just moved to another group (flashes once; `generation` tells one move from the next).
    let flash: (id: TerminalID, generation: Int)?
    /// Whether the fence of group `group` takes terminal `id` in, and whether the pane of `id` has room for `other` beside it.
    let admits: @MainActor (GroupID, TerminalID) -> Bool
    let hasRoom: @MainActor (_ other: TerminalID, _ beside: TerminalID) -> Bool
    /// The group whose popover is open.
    let popover: GroupID?
    let openPopover: @MainActor (GroupID?) -> Void
    /// Where each row stands in the window, for the menu ⌃↩ opens under it.
    let frames: RowFrames

    struct GroupChoice: Identifiable {
        let id: GroupID
        let name: String
        let members: [TerminalID]
    }

    /// Close a terminal or a group: the window asks first when something runs in it; without a window the core closes at once.
    @MainActor func closeTerminal(_ id: TerminalID) { hooks.closeTerminal?(id) ?? send(.close(id)) }
    @MainActor func closeGroup(_ id: GroupID) { hooks.closeGroup?(id) ?? send(.closeGroup(id)) }

    func group(of terminal: TerminalID) -> GroupChoice? { groups.first { $0.members.contains(terminal) } }
}

/// The rows' frames in the window, kept as they are laid out: read only when a key asks for one, so it is not observed.
final class RowFrames {
    var frames: [TerminalID: CGRect] = [:]
    var heads: [GroupID: CGRect] = [:]
}
