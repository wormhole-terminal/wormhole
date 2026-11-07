// Each split's pane tree as the window keeps it. The workspace state knows only which terminals share a split;
// where each pane sits, its share and the zoom are the window's (`TabPanes`, the port of core's `view/panes`),
// so a layout is saved under its split and restored against whoever is still a member. Pure values, so the
// rules (a drop joins, a split names its new terminal, the selection is the focus) are tested without a window.

import PanesModel
import WorkspaceModel

struct PaneLayouts: Equatable {
    private var saved: [Int: TabPanes] = [:]

    /// A terminal alone has no split; its layout lives under its own negated id, which no split id can be.
    private static func key(_ terminal: TerminalSummary) -> Int { terminal.splitID ?? -terminal.id }

    /// The terminals on screen with `selected`: its split's members in the list's order, or it alone.
    static func members(in state: WorkspaceState) -> [TerminalID] {
        guard let terminal = state.selectedTerminal else { return [] }
        guard let split = terminal.splitID else { return [terminal.id] }
        return state.terminals.filter { $0.splitID == split }.map(\.id)
    }

    /// The panes on screen: the saved layout as far as it still holds, the selected terminal with the keyboard.
    func panes(in state: WorkspaceState) -> TabPanes? {
        guard let terminal = state.selectedTerminal,
            var panes = TabPanes.restored(saved[Self.key(terminal)], ids: Self.members(in: state))
        else { return nil }
        panes.focus(terminal.id)
        return panes
    }

    /// What a change of the panes (a drop, a divider, a focus click) means for the workspace, and the layout kept.
    /// Terminals the panes gained join the split; the pane that took the keyboard is selected.
    mutating func store(_ panes: TabPanes, in state: WorkspaceState) -> [WorkspaceAction] {
        guard let terminal = state.selectedTerminal else { return [] }
        let members = Set(Self.members(in: state))
        var actions: [WorkspaceAction] = panes.panes.filter { !members.contains($0) }.map { .joinSplit($0, with: terminal.id) }
        // A joiner or a click on another pane takes the keyboard (the workspace's join leaves the selection alone).
        if panes.focused != terminal.id { actions.append(.select(panes.focused)) }
        // The split may be named by this very change (a lone terminal gaining a pane).
        let next = actions.reduce(state, reduce)
        if let anchor = next.terminals.first(where: { $0.id == terminal.id }) { saved[Self.key(anchor)] = panes }
        return actions
    }

    /// ⌘D / ⇧⌘D: a new terminal beside the selected one along `axis`. Nil when nothing is selected or the room
    /// refuses it.
    mutating func split(_ axis: SplitAxis, in state: WorkspaceState, room: PaneRoom = PaneRoom(0, 0)) -> WorkspaceAction? {
        guard let selected = state.selected, var panes = panes(in: state), panes.canSplit(axis, room: room) else { return nil }
        let action = WorkspaceAction.splitRight(selected)
        let next = reduce(state, action)
        guard let new = next.selected, new != selected, panes.split(axis, new: new),
            let anchor = next.terminals.first(where: { $0.id == selected })
        else { return nil }
        saved[Self.key(anchor)] = panes
        return action
    }
}
