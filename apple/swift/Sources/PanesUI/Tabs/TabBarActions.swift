// What the tab bar's controls do that it cannot: each is the window's to carry out.

import PanesModel
import SwiftUI
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
    /// A tab's context menu (the row's) and the menu under a right-click of the + ("New terminal with"); the window fills them.
    public var tabMenu: (TerminalID) -> AnyView
    public var newMenu: () -> AnyView
    /// The switcher's footer: "⌃⌥[ / ] previous, next group" and "⌃⌥1–9", from the keymap (empty: no keys to say).
    public var groupKeys: () -> (hint: String, keys: String)

    public init(
        select: @escaping (TerminalID) -> Void = { _ in }, close: @escaping (TerminalID) -> Void = { _ in },
        newTerminal: @escaping (GroupID?) -> Void = { _ in }, openSet: @escaping (Int) -> Void = { _ in },
        newGroup: @escaping () -> Void = {}, showGroupsInSidebar: @escaping () -> Void = {},
        tabMenu: @escaping (TerminalID) -> AnyView = { _ in AnyView(EmptyView()) }, newMenu: @escaping () -> AnyView = { AnyView(EmptyView()) },
        groupKeys: @escaping () -> (hint: String, keys: String) = { ("", "") }
    ) {
        (self.select, self.close, self.newTerminal, self.openSet, self.newGroup, self.showGroupsInSidebar) =
            (select, close, newTerminal, openSet, newGroup, showGroupsInSidebar)
        (self.tabMenu, self.newMenu, self.groupKeys) = (tabMenu, newMenu, groupKeys)
    }
}
