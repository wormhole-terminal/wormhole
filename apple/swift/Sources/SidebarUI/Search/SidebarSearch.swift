// The one thing outside the sidebar that must know about its search field: ⌘F. Find in the terminal is an app key
// that reaches the terminal from anywhere in the window (core's keymap), so the window asks here first and
// only gets the terminal's find when the sidebar does not have the keyboard.

import Observation

@MainActor @Observable
public final class SidebarSearch {
    /// Counts up with every ask for the field; the list answers by showing it and giving it the keyboard.
    public private(set) var requests = 0
    /// Whether the list or its field has the keyboard (the list writes it).
    public internal(set) var hasKeyboard = false

    /// Counts up with every F6 or ⇥ that comes to the list; the list answers by taking the keyboard.
    public private(set) var keyboardRequests = 0

    public init() {}

    /// F6 and ⇥ into the list (`view::focus`: the sidebar is one stop).
    public func requestKeyboard() { keyboardRequests += 1 }

    /// ⌘F: `true` when the sidebar has the keyboard and takes it for its own search.
    @discardableResult
    public func revealIfFocused() -> Bool {
        guard hasKeyboard else { return false }
        reveal()
        return true
    }

    /// Show the field and put the caret in it, whatever had the keyboard.
    public func reveal() { requests += 1 }
}
