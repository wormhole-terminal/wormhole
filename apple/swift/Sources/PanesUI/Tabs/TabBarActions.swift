// What the tab bar's controls do that it cannot: each is the window's to carry out.

import PanesModel
import WorkspaceModel

public struct TabBarActions {
    public var select: (TerminalID) -> Void
    public var close: (TerminalID) -> Void
    /// A new terminal in the current group (nil: in none).
    public var newTerminal: (GroupID?) -> Void
    /// Goes to the set at this index of `TabBarModel.sets`: the terminal last selected in it, else its first.
    public var openSet: (Int) -> Void
    public var newGroup: () -> Void
    public var showGroupsInSidebar: () -> Void

    public init(
        select: @escaping (TerminalID) -> Void = { _ in }, close: @escaping (TerminalID) -> Void = { _ in },
        newTerminal: @escaping (GroupID?) -> Void = { _ in }, openSet: @escaping (Int) -> Void = { _ in },
        newGroup: @escaping () -> Void = {}, showGroupsInSidebar: @escaping () -> Void = {}
    ) {
        (self.select, self.close, self.newTerminal, self.openSet, self.newGroup, self.showGroupsInSidebar) =
            (select, close, newTerminal, openSet, newGroup, showGroupsInSidebar)
    }
}
