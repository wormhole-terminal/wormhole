// What the core tells the shell to do that no snapshot carries (core `Event`s `SavedTextForgotten` and `GroupToWindow`): the
// window answers each, once.

import Foundation

public enum WorkspaceNotice: Sendable, Equatable {
    /// "Forget text" was set: erase what was saved of the terminal's screen.
    case savedTextForgotten(TerminalID)
    /// The group is to move to a window of its own with these terminals (and their splits').
    case groupToWindow(GroupID, terminals: [TerminalID])
    /// Terminals stay in the app because no session host would start (once per run): why, in the core's words.
    case hostFallback(String)
    /// Terminals end when the app quits because it runs from a disk image or a temporary copy (once per run).
    case moveToApplications
    /// What the bar over terminal `id`'s pane says about its host changed (`hostProblem(of:)`).
    case hostChanged(TerminalID)
    /// The host of terminal `id` was ended from its bar: the terminal closes.
    case hostEnded(TerminalID)
}
