// What the tab bar's controls do that it cannot: each is the window's to carry out.

import DesignSystem
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
    /// A group's native menu, under a right-click of its card in the switcher (`switcher.rs`); the window builds it.
    public var groupMenu: @MainActor (GroupID) -> NSMenu? = { _ in nil }
    /// A terminal's tile as its row in the list draws it (the agent's logo, the status light and badge), at this size
    /// (`tab_bar.rs` `tile_for`); nil draws the plain tile with the state's dot.
    public var tile: ((TerminalID, CGFloat) -> AnyView)?
    /// A set's terminals in the order a card shows their tiles: the loudest first.
    public var loudest: ([TerminalID]) -> [TerminalID] = { $0 }
    /// What a group's colour washes its card with in the switcher, in this theme; nil for a group with none.
    public var cardWash: (GroupID, ThemeTokens) -> Color? = { _, _ in nil }
    /// The popover a card's name opens (the group's own, as its head's name does), given how to close it.
    public var groupPopover: ((GroupID, @escaping () -> Void) -> AnyView)?
    /// "Rename…" of a group's menu while the list is not on screen: the group, and a count that moves with each ask. Its
    /// card opens the popover, whose name is a field.
    public var renameAsk: (group: GroupID, count: Int)?

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
