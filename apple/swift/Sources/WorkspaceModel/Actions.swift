// Everything a person can do to the window. The UI sends these; it never edits state itself. Each is one of core's
// `workspace::Action`s (the bridge's `ActionMapping` says which): the core decides what it does.

import Foundation

public enum WorkspaceAction: Sendable, Equatable {
    case select(TerminalID)
    /// The next (positive) or previous terminal as listed, wrapping; folded rows are skipped.
    case selectStep(Int)
    /// ⌘1…⌘9: the terminal that holds the number.
    case selectNumber(Int)
    case newTerminal
    case close(TerminalID)
    case rename(TerminalID, String)
    case togglePin(TerminalID)
    /// A new terminal that takes its place last in the group (two core actions: the store sends both).
    case newTerminalInGroup(GroupID)

    /// A group holding these terminals and the rest of their splits.
    case newGroup([TerminalID])
    case renameGroup(GroupID, String)
    case toggleGroup(GroupID)
    case setGroupPinned(GroupID, Bool)
    /// Take the group apart; its terminals stand where it stood.
    case ungroup(GroupID)
    case addToGroup(TerminalID, GroupID)
    /// Take a terminal (and its split) out of its group: it stands right after it.
    case removeFromGroup(TerminalID)

    /// What a drag let go over: the core's own rules place it (`DropPlan` says what that does, before it is let go).
    case dropTerminal(TerminalID, over: DropOver)
    case dropGroup(GroupID, over: DropOver)

    /// A new terminal beside this one, in its folder and split; it takes the selection.
    case splitRight(TerminalID)
    /// The first terminal shows beside the second ("Show beside…").
    case joinSplit(TerminalID, with: TerminalID)
    case leaveSplit(TerminalID)

    case toggleSidebar
    case togglePanel
    case showPanelTab(PanelTab)
    case search(String)
    case showOverlay(Overlay?)
}
