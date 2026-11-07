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
    /// A new terminal that starts in this folder (a worktree just made); selected like any other.
    case newTerminalIn(String)
    case close(TerminalID)
    case rename(TerminalID, String)
    case togglePin(TerminalID)
    /// "Hide from Agents" and "Forget text": the model keeps and shows them, the shell enforces them.
    case setAgentHidden(TerminalID, Bool)
    case setForgetText(TerminalID, Bool)
    /// A new terminal last in the group, in its folder (else where one was last used), selected.
    case newTerminalInGroup(GroupID)

    /// A group holding these terminals and the rest of their splits.
    case newGroup([TerminalID])
    /// Hosts nobody named, adopted: into the Recovered group.
    case recoverGroup([TerminalID])
    /// A new terminal in a group of its own.
    case newTerminalInNewGroup
    case renameGroup(GroupID, String)
    case toggleGroup(GroupID)
    case setGroupPinned(GroupID, Bool)
    /// Take the group apart; its terminals stand where it stood.
    case ungroup(GroupID)
    case addToGroup(TerminalID, GroupID)
    /// Take a terminal (and its split) out of its group: it stands right after it.
    case removeFromGroup(TerminalID)
    /// A colour of the nine names, or none.
    case setGroupColor(GroupID, GroupColor?)
    case setGroupCustomColor(GroupID, red: UInt8, green: UInt8, blue: UInt8)
    /// A folder pins the group to it; clearing it counts as saying no to the offer.
    case setGroupFolder(GroupID, String?)
    case setOnlyInFolder(GroupID, Bool)
    /// The person said no to the folder the core offered.
    case declineGroupFolder(GroupID)
    /// End every terminal of the group (the shell asks first when `workingCount > 0`); the group goes unless pinned.
    case closeGroup(GroupID)
    case moveGroupToWindow(GroupID)

    /// What a drag let go over: the core's own rules place it (`DropPlan` says what that does, before it is let go).
    case dropTerminal(TerminalID, over: DropOver)
    case dropGroup(GroupID, over: DropOver)
    /// A `DropPlan.move` applied as it said, and a group moved to a landing.
    case moveTerminal(TerminalID, Landing, leavesSplit: Bool, withMates: Bool)
    case moveGroup(GroupID, Landing)

    /// A new terminal beside this one, in its folder and split; it takes the selection.
    case splitRight(TerminalID)
    /// The first terminal shows beside the second ("Show beside…").
    case joinSplit(TerminalID, with: TerminalID)
    case leaveSplit(TerminalID)
    /// The second terminal joins the first's split, on its right ("Show beside").
    case showBeside(TerminalID, other: TerminalID)
    /// View > Layout: new terminals fill the slots; panes over the preset's count leave the split and keep running.
    case applyLayout(LayoutPresetID)
    /// A divider of the selected split (`path` and `index` as the core's `PanesSnapshot.dividers` name it).
    case dragDivider(path: [UInt32], index: UInt32, position: Double)
    case equalizeDivider(path: [UInt32], index: UInt32)

    case toggleSidebar
    case togglePanel
    case showPanelTab(PanelTab)
    case search(String)
    case showOverlay(Overlay?)
    /// The window's width, from which the core fits the three columns (zero: nothing is refused for want of room).
    case setWindowWidth(Double)
    /// The area the panes share.
    case setPaneArea(width: Double, height: Double)
    /// A column's edge dragged to `extent` from the window's edge it hangs on; reset puts it back to its default.
    case dragEdge(ColumnEdge, extent: Double)
    case resetEdge(ColumnEdge)
    /// A feature comes or goes, and its tab with it.
    case setFeature(FeatureKey, Bool)
    /// The title row's parts in order (`config.json`'s `title_row`).
    case setTitleRow([String])
    /// ⌘ numbers follow the list as shown (`true`), or stay with their terminal.
    case setNumbersFollowList(Bool)
}
