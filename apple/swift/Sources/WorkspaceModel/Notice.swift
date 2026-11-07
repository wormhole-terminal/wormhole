// What the core tells the shell to do that no snapshot carries (core `Event`s `SavedTextForgotten` and `GroupToWindow`): the
// window answers each, once.

import Foundation

public enum WorkspaceNotice: Sendable, Equatable {
    /// "Forget text" was set: erase what was saved of the terminal's screen.
    case savedTextForgotten(TerminalID)
    /// The group is to move to a window of its own with these terminals (and their splits').
    case groupToWindow(GroupID, terminals: [TerminalID])
}
