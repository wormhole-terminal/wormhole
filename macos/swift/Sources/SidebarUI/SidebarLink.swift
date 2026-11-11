// What the window and the list say to each other outside the views' own data: who has the keyboard, the keyboard's
// menus (a SwiftUI `.contextMenu` cannot be opened from code, so ⌃↩ asks the window for a native one under the row or
// head) and a rename the keyboard's menu chose. Find in the terminal is an app key that reaches the terminal from
// anywhere in the window; the GPUI list has no search field of its own, and neither has this one.

import CoreGraphics
import Observation
import WorkspaceModel

@MainActor @Observable
public final class SidebarLink {
    /// Whether the list has the keyboard (the list writes it).
    public internal(set) var hasKeyboard = false

    /// Counts up with every F6 or ⇥ that comes to the list; the list answers by taking the keyboard.
    public private(set) var keyboardRequests = 0

    /// ⌃↩ on a row or a head: the window shows its menu at this frame (in the window's coordinates, from its top left).
    @ObservationIgnored public var rowMenu: @MainActor (TerminalID, CGRect) -> Void = { _, _ in }
    @ObservationIgnored public var groupMenu: @MainActor (GroupID, CGRect) -> Void = { _, _ in }

    /// What the keyboard's menus ask of the list (a SwiftUI menu's items run in the list itself): the list answers each ask,
    /// counted by `requests`.
    public private(set) var requests = 0
    public private(set) var request: Request?

    public enum Request: Equatable, Sendable {
        /// "Rename…": the name turns into a field.
        case rename(SidebarRename)
        /// "Other colour…": the group's popover.
        case popover(GroupID)
        /// "New terminal in group": first the question about its folder, when the core has one.
        case newInGroup(GroupID)
    }

    public init() {}

    public func ask(_ request: Request) {
        self.request = request
        requests += 1
    }

    /// F6 and ⇥ into the list (`view::focus`: the sidebar is one stop).
    public func requestKeyboard() { keyboardRequests += 1 }
}

/// The name being typed in the list: a terminal's or a group's.
public enum SidebarRename: Equatable, Sendable {
    case terminal(TerminalID)
    case group(GroupID)
}
