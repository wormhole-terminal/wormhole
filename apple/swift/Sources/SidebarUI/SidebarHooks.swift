// What the window answers for the list, which it cannot do itself: the close questions (a running agent or command is
// asked about before it ends), the keyboard going between the list and the terminal, and the folder's copy and reveal. Every hook
// is optional so a list drawn alone (a picture, a test) works with the core's own answers.

import AppKit
import WorkspaceModel

public struct SidebarHooks {
    /// Close a terminal or a group, asking first when something runs in it; nil sends the close at once.
    public var closeTerminal: (@MainActor (TerminalID) -> Void)?
    public var closeGroup: (@MainActor (GroupID) -> Void)?
    /// The keyboard goes to the terminal on screen (↩, a rename ending, a group's popover closing).
    public var focusTerminal: @MainActor () -> Void = {}
    /// Space: the terminal shown next leaves the keyboard in the list.
    public var browse: @MainActor () -> Void = {}
    /// "Copy path" and "Reveal in Finder" of a terminal's folder: the window copies (and says "Copied") and reveals (a test
    /// instance reveals nothing).
    public var copyPath: @MainActor (String) -> Void = { _ in }
    public var revealFolder: @MainActor (String) -> Void = { _ in }
    /// A group's menu for a right-click on its head: the window's native one, whose row of colour dots no SwiftUI menu
    /// item draws (`sidebar/group_popover.rs` `group_menu`). A list drawn alone has none.
    public var groupMenu: @MainActor (GroupID) -> NSMenu? = { _ in nil }

    public init() {}
}
