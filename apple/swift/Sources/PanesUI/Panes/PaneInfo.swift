// What a pane's header says about its terminal, and what the pane area asks the window to do.

import PanesModel
import WorkspaceModel

/// A pane's terminal as its header shows it.
public struct PaneInfo: Sendable, Equatable {
    public var title: String
    /// The terminal's folder as the sidebar's second line writes it (`~/Projects/tube`).
    public var folder: String?
    public var activity: Activity
    public var agent: AgentID?

    public init(title: String, folder: String? = nil, activity: Activity = .idle, agent: AgentID? = nil) {
        (self.title, self.folder, self.activity, self.agent) = (title, folder, activity, agent)
    }
}

/// What a pane's buttons and menu do that the tree alone cannot: closing a pane ends a terminal, splitting
/// opens one, leaving the split moves one to its own row. Each is the window's to carry out.
public struct PaneActions {
    public var close: (PaneID) -> Void
    public var split: (SplitAxis) -> Void
    public var leaveSplit: (PaneID) -> Void
    /// A terminal dropped on a pane's edge that the tree accepted: the window starts its session if needed.
    public var dropped: (DraggedTerminal) -> Void

    public init(
        close: @escaping (PaneID) -> Void = { _ in }, split: @escaping (SplitAxis) -> Void = { _ in },
        leaveSplit: @escaping (PaneID) -> Void = { _ in }, dropped: @escaping (DraggedTerminal) -> Void = { _ in }
    ) {
        (self.close, self.split, self.leaveSplit, self.dropped) = (close, split, leaveSplit, dropped)
    }
}
